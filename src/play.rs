
use crate::mc_types::{self, Packet, Result};


pub enum Play {
    PlayPacket(PlayPacket),
}

impl Play {
    pub async fn read(
        conn: &mut mc_types::ProtocolConnection<'_>,
    ) -> Result<Self> {
        let mut data = conn.read_data().await?;
        Ok(Self::PlayPacket(PlayPacket::get(&mut data)?))
    }
}

pub struct PlayPacket {
    pub data: Vec<u8>
}

impl Packet for PlayPacket {

    fn packet_id() -> i32 {0}

    fn get(data: &mut Vec<u8>) -> Result<Self> {
        Ok(Self {
            data: data.clone()
        })
    }

    fn convert(&self) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];
        data.append(&mut self.data.clone());

        data
    }

}
