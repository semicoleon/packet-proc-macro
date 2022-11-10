use packet_codegen::Packet;

#[derive(Packet)]
#[packet(world_opcode = 100)]
#[packet(login_opcode = 12)]
struct Income {
    name: String,
    age: u64,
    nickname_size: u32,
    #[packet(dynamic = [nickname_size])]
    nickname: String,
}

impl Income {
    pub fn should_read_nickname(dep_values: Income) -> bool {
        dep_values.nickname_size > 0
    }
}
