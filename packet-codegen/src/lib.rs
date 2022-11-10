use std::fmt::Debug;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    bracketed, parse_macro_input, punctuated::Punctuated, spanned::Spanned, DeriveInput, Expr,
    Ident, Token, Type,
};

#[proc_macro_derive(Packet, attributes(packet))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse syn's input type into what we actually expect
    let input = match Input::parse(&input) {
        Ok(input) => input,
        Err(e) => return e.into_compile_error().into(),
    };

    let ident = &input.name;

    // If we have a login opcode, we need to set omit_bytes to a different value.
    let omit_bytes_extra = if let Some(login_opcode) = input.login_opcode {
        // "as u8" doesn't seem to make sense here, are you sure you wanted that?
        quote! {
            omit_bytes = (#login_opcode as u8).to_le_bytes().len();
        }
    } else {
        // Use an empty token stream since we don't need to do anything in this case
        Default::default()
    };

    let is_compressed_setter = if let Some(compressed_opcode) = input.compressed_opcode {
        quote! {
            if #compressed_opcode == opcode {
                omit_bytes += 6;
                is_compressed = true;
            }
        }
    } else {
        // Use an empty token stream since we don't need to do anything in this case
        Default::default()
    };

    quote! {
        impl #ident {
            #[allow(dead_code)]
            pub fn from_binary(buffer: &Vec<u8>) -> Self {
                #![allow(unused_mut)]
                #![allow(unused_variables)]
                #![allow(unused_assignments)]

                // Using the crate module here for simplicity, in a macro you wanted other people to use that would probably not be the correct choice though
                let mut omit_bytes: usize = crate::INCOMING_HEADER_LENGTH;

                #omit_bytes_extra

                let mut is_compressed = false;
                let mut reader = std::io::Cursor::new(buffer[2..].to_vec());
                let opcode = byteorder::ReadBytesExt::read_u16::<byteorder::LittleEndian>(
                    &mut reader
                ).unwrap();

                #is_compressed_setter

                let mut internal_buffer: Vec<u8> = Vec::new();
                if is_compressed {
                    let data = &buffer[omit_bytes..];
                    let mut decoder = flate2::read::DeflateDecoder::new(data);
                    std::io::Read::read_to_end(&mut decoder, &mut internal_buffer).unwrap();
                }

                let buffer = if internal_buffer.is_empty() {
                    buffer[omit_bytes..].to_vec()
                } else {
                    internal_buffer
                };
                let mut reader = std::io::Cursor::new(&buffer);

                todo!()
            }
        }
    }
    .into()
}

/// Parsed data about the struct we are deriving for.
struct Input {
    /// The name of the struct
    name: Ident,
    /// The login opcode expression, if one was given
    login_opcode: Option<Expr>,
    /// The world opcode expression, if one was given
    world_opcode: Option<Expr>,
    /// The compressed opcode expression, if one was given
    ///
    /// This was changed from the `compressed_if` input in the original macro since it seemed to be required to be an opcode.
    compressed_opcode: Option<Expr>,
    /// The fields of the struct.
    fields: Vec<Field>,
}

impl Input {
    fn parse(input: &DeriveInput) -> syn::Result<Self> {
        // An ident to compare attribute paths against.
        let packet = Ident::new("packet", Span::call_site());

        // The item level attribute settings we know about.
        let mut login_opcode = None::<Expr>;
        let mut world_opcode = None::<Expr>;
        let mut compressed_opcode = None::<Expr>;
        for a in &input.attrs {
            // If it's one of our attributes, attempt to parse the arguments.
            if !a.path.is_ident(&packet) {
                continue;
            }

            let attr = a.parse_args_with(ItemAttr::parse_args)?;

            use ItemAttr::*;
            match attr {
                LoginOpcode(expr) => {
                    if login_opcode.is_some() {
                        return Err(syn::Error::new_spanned(
                            &a.path,
                            "Duplicate attribute not supported here",
                        ));
                    }
                    login_opcode = Some(expr);
                }
                WorldOpcode(expr) => {
                    if world_opcode.is_some() {
                        return Err(syn::Error::new_spanned(
                            &a.path,
                            "Duplicate attribute not supported here",
                        ));
                    }
                    world_opcode = Some(expr);
                }
                CompressedOpcode(expr) => {
                    if compressed_opcode.is_some() {
                        return Err(syn::Error::new_spanned(
                            &a.path,
                            "Duplicate attribute not supported here",
                        ));
                    }
                    compressed_opcode = Some(expr);
                }
            }
        }

        let data = match &input.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(named) => named,
                syn::Fields::Unit => {
                    return Err(syn::Error::new(
                        input.span(),
                        "Unit structs are not supported",
                    ))
                }
                syn::Fields::Unnamed(_) => {
                    return Err(syn::Error::new(
                        input.span(),
                        "Tuple structs are not supported",
                    ))
                }
            },
            syn::Data::Enum(data) => {
                return Err(syn::Error::new(
                    data.enum_token.span(),
                    "Enums are not supported",
                ))
            }
            syn::Data::Union(data) => {
                return Err(syn::Error::new(
                    data.union_token.span(),
                    "Unions are not supported",
                ))
            }
        };

        let fields = data
            .named
            .iter()
            .map(|f| {
                let ident = f.ident.clone().ok_or_else(|| {
                    // We aren't supporting tuple structs so we should always have a name.
                    syn::Error::new_spanned(f, "Fields must have an identifier")
                })?;

                let attributes = f
                    .attrs
                    .iter()
                    .filter_map(|a| {
                        if a.path.is_ident(&packet) {
                            Some(a.parse_args_with(FieldAttr::parse_args))
                        } else {
                            None
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok::<_, syn::Error>(Field {
                    name: ident,
                    ty: f.ty.clone(),
                    attributes,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            name: input.ident.clone(),
            login_opcode,
            world_opcode,
            compressed_opcode,
            fields,
        })
    }
}

enum ItemAttr {
    LoginOpcode(Expr),
    WorldOpcode(Expr),
    CompressedOpcode(Expr),
}

impl ItemAttr {
    fn parse_args(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        match name.to_string().as_str() {
            "login_opcode" => {
                let _eq: Token![=] = input.parse()?;
                let lit = input.parse()?;
                Ok(Self::LoginOpcode(lit))
            }
            "world_opcode" => {
                let _eq: Token![=] = input.parse()?;
                let lit = input.parse()?;
                Ok(Self::WorldOpcode(lit))
            }
            "compressed_opcode" => {
                let _eq: Token![=] = input.parse()?;
                let lit = input.parse()?;
                Ok(Self::CompressedOpcode(lit))
            }
            _ => Err(syn::Error::new(name.span(), "Unexpected attribute name")),
        }
    }
}

/// A field in the struct, along with any attributes.
struct Field {
    name: Ident,
    ty: Type,
    attributes: Vec<FieldAttr>,
}

#[derive(Debug)]
enum FieldAttr {
    Dynamic { deps: Vec<Ident> },
}

impl FieldAttr {
    fn parse_args(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        match name.to_string().as_str() {
            "dynamic" => {
                let deps_parse;
                let _eq: Token![=] = input.parse()?;
                bracketed!(deps_parse in input);
                let deps = Punctuated::<Ident, Token![,]>::parse_terminated(&deps_parse)?
                    .into_iter()
                    .collect();

                Ok(Self::Dynamic { deps })
            }
            _ => Err(syn::Error::new(name.span(), "Unexpected attribute name")),
        }
    }
}
