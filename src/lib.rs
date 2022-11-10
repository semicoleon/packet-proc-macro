mod income;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{BufRead, Error, ErrorKind, Read, Write};

pub type PacketOutcome = (u32, Vec<u8>);

pub const OUTCOMING_HEADER_LENGTH: usize = 6;
pub const OUTCOMING_OPCODE_LENGTH: usize = 4;
pub const INCOMING_HEADER_LENGTH: usize = 4;
pub const INCOMING_OPCODE_LENGTH: usize = 2;

pub trait BinaryConverter {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error>;
    fn read_from<R: BufRead>(reader: R) -> Result<Self, Error>
    where
        Self: Sized;
}

impl BinaryConverter for u8 {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_u8(*self)
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        reader.read_u8()
    }
}

impl BinaryConverter for u16 {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_u16::<LittleEndian>(*self)
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        reader.read_u16::<LittleEndian>()
    }
}

impl BinaryConverter for u32 {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_u32::<LittleEndian>(*self)
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        reader.read_u32::<LittleEndian>()
    }
}

impl BinaryConverter for u64 {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_u64::<LittleEndian>(*self)
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        reader.read_u64::<LittleEndian>()
    }
}

impl BinaryConverter for String {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_all(self.as_bytes())
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        let mut internal_buf = vec![];
        reader.read_until(0, &mut internal_buf)?;
        match String::from_utf8(internal_buf[..internal_buf.len()].to_vec()) {
            Ok(string) => Ok(string),
            Err(err) => Err(Error::new(ErrorKind::Other, err.to_string())),
        }
    }
}

impl<const N: usize> BinaryConverter for [u8; N] {
    fn write_into(&self, buffer: &mut Vec<u8>) -> Result<(), Error> {
        buffer.write_all(self)
    }

    fn read_from<R: BufRead>(mut reader: R) -> Result<Self, Error> {
        let mut internal_buf = [0; N];
        reader.read_exact(&mut internal_buf)?;
        Ok(internal_buf)
    }
}
