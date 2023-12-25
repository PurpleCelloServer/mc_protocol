// Yeahbut December 2023

pub mod serverbound {

    use tokio::net::tcp::OwnedReadHalf;

    use crate::mc_types::{self, Result, Packet, PacketError, MCTypes};
    use purple_cello_mc_protocol_derive::packet_derive;

    pub enum HandshakeEnum {
        Handshake(Handshake),
    }

    impl HandshakeEnum {
        pub async fn read(stream: &mut OwnedReadHalf) -> Result<Self> {
            let mut data = mc_types::read_data(stream).await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Handshake::packet_id() {
                return Ok(Self::Handshake(Handshake::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    #[derive(packet_derive)]
    pub struct Handshake {
        pub protocol_version: MCTypes::VarInt,
        pub server_address: MCTypes::String,
        pub server_port: MCTypes::UnsignedShort,
        pub next_state: MCTypes::VarInt,
    }

    // impl Packet for Handshake {

    //     fn packet_id() -> i32 {0}

    //     fn get(data: &mut Vec<u8>) -> Result<Self> {
    //         Ok(Self {
    //             protocol_version: mc_types::get_var_int(data)?,
    //             server_address: mc_types::get_string(data)?,
    //             server_port: mc_types::get_unsigned_short(data)?,
    //             next_state: mc_types::get_var_int(data)?,
    //         })
    //     }

    //     fn convert(&self) -> Vec<u8> {
    //         let mut data: Vec<u8> = vec![];
    //         data.append(&mut mc_types::convert_var_int(Self::packet_id()));
    //         data.append(&mut mc_types::convert_var_int(self.protocol_version));
    //         data.append(&mut mc_types::convert_string(&self.server_address));
    //         data.append(
    //             &mut mc_types::convert_unsigned_short(self.server_port));
    //         data.append(&mut mc_types::convert_var_int(self.next_state));

    //         data
    //     }

    // }
}
