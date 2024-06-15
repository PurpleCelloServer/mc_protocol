// Yeahbut December 2023

pub mod clientbound {

    use serde::{Serialize, Deserialize, Deserializer};
    use serde::de::{self, Visitor, MapAccess};
    use std::fmt;

    use crate::mc_types::{self, Result, Packet, PacketError};

    #[derive(Serialize, Deserialize)]
    pub struct StatusVersion {
        pub name: String,
        pub protocol: i32,
    }

    #[derive(Serialize)]
    pub enum StatusDescription {
        String(String),
        Chat(mc_types::Chat),
    }

    impl<'de> Deserialize<'de> for StatusDescription {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StatusDescriptionVisitor;

        impl<'de> Visitor<'de> for StatusDescriptionVisitor {
            type Value = StatusDescription;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a string or a map representing a Chat object")
            }

            fn visit_str<E>(self, value: &str)
                -> std::result::Result<StatusDescription, E>
            where
                E: de::Error,
            {
                Ok(StatusDescription::String(value.to_string()))
            }

            fn visit_map<M>(self, map: M)
                -> std::result::Result<StatusDescription, M::Error>
            where
                M: MapAccess<'de>,
            {
                let chat = mc_types::Chat::deserialize(
                    de::value::MapAccessDeserializer::new(map))?;
                Ok(StatusDescription::Chat(chat))
            }
        }

        deserializer.deserialize_any(StatusDescriptionVisitor)
    }
}

    #[derive(Serialize, Deserialize)]
    pub struct StatusPlayerInfo {
        pub name: String,
        pub id: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct StatusPlayers {
        pub max: i32,
        pub online: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sample: Option<Vec<StatusPlayerInfo>>
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize)]
    pub struct StatusResponseData {
        pub version: StatusVersion,
        pub description: StatusDescription,
        pub players: StatusPlayers,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub favicon: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub enforcesSecureChat: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub previewsChat: Option<bool>,
    }

    pub enum StatusPackets {
        Status(Status),
        Ping(Ping),
    }

    impl StatusPackets {
        pub async fn read<T: mc_types::ProtocolRead>(
            conn: &mut T,
        ) -> Result<Self> {
            let mut data = conn.read_data().await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Status::packet_id() {
                return Ok(Self::Status(Status::get(&mut data)?))
            } else if packet_id == Ping::packet_id() {
                return Ok(Self::Ping(Ping::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    pub struct Status {
        pub response: String
    }

    impl Status {

        pub fn from_json(data: StatusResponseData) -> Result<Self> {
            Ok(Self {
                response: serde_json::to_string(&data)?
            })
        }

        pub fn get_json(&self) -> Result<StatusResponseData> {
            Ok(serde_json::from_str(&self.response)?)
        }

    }

    impl Packet for Status {

        fn packet_id() -> i32 {0}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                response: mc_types::get_string(&mut data)?
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.response));

            data
        }

    }

    pub struct Ping {
        pub payload: i64
    }

    impl Packet for Ping {

        fn packet_id() -> i32 {1}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                payload: mc_types::get_i64(&mut data)
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_i64(self.payload));

            data
        }

    }

}

pub mod serverbound {

    use crate::mc_types::{self, Result, Packet, PacketError};

    pub enum StatusPackets {
        Status(Status),
        Ping(Ping),
    }

    impl StatusPackets {
        pub async fn read<T: mc_types::ProtocolRead>(
            conn: &mut T,
        ) -> Result<Self> {
            let mut data = conn.read_data().await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Status::packet_id() {
                return Ok(Self::Status(Status::get(&mut data)?))
            } else if packet_id == Ping::packet_id() {
                return Ok(Self::Ping(Ping::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    pub struct Status {}

    impl Packet for Status {

        fn packet_id() -> i32 {0}

        fn get(_data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {})
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));

            data
        }

    }

    pub struct Ping {
        pub payload: i64
    }

    impl Packet for Ping {

        fn packet_id() -> i32 {1}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                payload: mc_types::get_i64(&mut data)
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_i64(self.payload));

            data
        }

    }

}
