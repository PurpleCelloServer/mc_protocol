// Yeahbut December 2023

pub mod clientbound {

    use crate::mc_types::{self, Result, Packet, PacketArray, PacketError};

    pub enum Login {
        Disconnect(Disconnect),
        EncryptionRequest(EncryptionRequest),
        LoginSuccess(LoginSuccess),
        SetCompression(SetCompression),
        PluginRequest(PluginRequest),
        CookieRequest(CookieRequest),
    }

    impl Login {
        pub async fn read<T: mc_types::ProtocolRead>(
            conn: &mut T,
        ) -> Result<Self> {
            let mut data = conn.read_data().await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Disconnect::packet_id() {
                return Ok(Self::Disconnect(Disconnect::get(&mut data)?))
            } else if packet_id == EncryptionRequest::packet_id() {
                return Ok(Self::EncryptionRequest(
                    EncryptionRequest::get(&mut data)?))
            } else if packet_id == LoginSuccess::packet_id() {
                return Ok(Self::LoginSuccess(LoginSuccess::get(&mut data)?))
            } else if packet_id == SetCompression::packet_id() {
                return Ok(Self::SetCompression(
                    SetCompression::get(&mut data)?))
            } else if packet_id == PluginRequest::packet_id() {
                return Ok(Self::PluginRequest(
                    PluginRequest::get(&mut data)?))
            } else if packet_id == CookieRequest::packet_id() {
                return Ok(Self::CookieRequest(
                    CookieRequest::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    pub struct Disconnect {
        pub reason: String
    }

    impl Packet for Disconnect {

        fn packet_id() -> i32 {0}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                reason: mc_types::get_string(&mut data)?
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.reason));

            data
        }

    }

    pub struct EncryptionRequest {
        pub server_id: String,
        pub public_key: Vec<u8>,
        pub verify_token: Vec<u8>,
    }

    impl Packet for EncryptionRequest {

        fn packet_id() -> i32 {1}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                server_id: mc_types::get_string(&mut data)?,
                public_key: mc_types::get_byte_array(&mut data)?,
                verify_token: mc_types::get_byte_array(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.server_id));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.public_key.clone()));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.verify_token.clone()));

            data
        }

    }

    pub struct LoginSuccess {
        pub uuid: u128,
        pub username: String,
        pub properties: Vec<LoginSuccessProperty>,
        pub strict_error_handling: bool,
    }

    impl Packet for LoginSuccess {

        fn packet_id() -> i32 {2}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                uuid: mc_types::get_uuid(&mut data),
                username: mc_types::get_string(&mut data)?,
                properties: LoginSuccessProperty::get_array(&mut data)?,
                strict_error_handling: mc_types::get_bool(&mut data),
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_uuid(self.uuid));
            data.append(&mut mc_types::convert_string(&self.username));
            data.append(&mut LoginSuccessProperty::convert_array(
                &mut self.properties.clone()));
            data.append(&mut mc_types::convert_bool(
                self.strict_error_handling));

            data
        }

    }

    pub struct LoginSuccessProperty {
        name: String,
        value: String,
        signature: Option<String>,
    }

    impl Clone for LoginSuccessProperty {
        fn clone(&self) -> Self {
            Self {
                name: self.name.clone(),
                value: self.value.clone(),
                signature: self.signature.clone(),
            }
        }
    }

    impl PacketArray for LoginSuccessProperty {

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            let name = mc_types::get_string(&mut data)?;
            let value = mc_types::get_string(&mut data)?;
            let is_signed = mc_types::get_bool(&mut data);
            let mut signature: Option<String> = None;
            if is_signed {
                signature = Some(mc_types::get_string(&mut data)?);
            }
            Ok(Self {
                name,
                value,
                signature,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_string(&self.name));
            data.append(&mut mc_types::convert_string(&self.value));
            match &self.signature {
                Some(value) => {
                    data.append(&mut &mut mc_types::convert_bool(true));
                    data.append(&mut mc_types::convert_string(&value));
                },
                None => data.append(&mut &mut mc_types::convert_bool(false))
            }

            data
        }

    }

    pub struct SetCompression {
        pub threshold: i32,
    }

    impl Packet for SetCompression {

        fn packet_id() -> i32 {3}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                threshold: mc_types::get_var_int(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_var_int(self.threshold));

            data
        }

    }

    pub struct PluginRequest {
        pub message_id: i32,
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl Packet for PluginRequest {

        fn packet_id() -> i32 {4}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                message_id: mc_types::get_var_int(&mut data)?,
                channel: mc_types::get_string(&mut data)?,
                data: data.clone(),
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_var_int(self.message_id));
            data.append(&mut mc_types::convert_string(&self.channel));
            data.append(&mut self.data.clone());

            data
        }

    }

    pub struct CookieRequest {
        pub key: String,
    }

    impl Packet for CookieRequest {

        fn packet_id() -> i32 {5}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                key: mc_types::get_string(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.key));

            data
        }

    }

}

pub mod serverbound {

    use crate::mc_types::{self, Result, Packet, PacketError};

    pub enum Login {
        LoginStart(LoginStart),
        EncryptionResponse(EncryptionResponse),
        PluginResponse(PluginResponse),
        Acknowledged(Acknowledged),
        CookieResponse(CookieResponse),
    }

    impl Login {
        pub async fn read<T: mc_types::ProtocolRead>(
            conn: &mut T,
        ) -> Result<Self> {
            let mut data = conn.read_data().await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == LoginStart::packet_id() {
                return Ok(Self::LoginStart(LoginStart::get(&mut data)?))
            } else if packet_id == EncryptionResponse::packet_id() {
                return Ok(Self::EncryptionResponse(
                    EncryptionResponse::get(&mut data)?))
            } else if packet_id == PluginResponse::packet_id() {
                return Ok(Self::PluginResponse(
                    PluginResponse::get(&mut data)?))
            } else if packet_id == Acknowledged::packet_id() {
                return Ok(Self::Acknowledged(
                    Acknowledged::get(&mut data)?))
            } else if packet_id == CookieResponse::packet_id() {
                return Ok(Self::CookieResponse(
                    CookieResponse::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    pub struct LoginStart {
        pub name: String,
        pub player_uuid: u128,
    }

    impl Packet for LoginStart {

        fn packet_id() -> i32 {0}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            let name = mc_types::get_string(&mut data)?;
            let player_uuid: u128 = mc_types::get_uuid(&mut data);
            Ok(Self {
                name,
                player_uuid,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.name));
            data.append(&mut mc_types::convert_uuid(self.player_uuid));

            data
        }

    }

    pub struct EncryptionResponse {
        pub shared_secret: Vec<u8>,
        pub verify_token: Vec<u8>,
    }

    impl Packet for EncryptionResponse {

        fn packet_id() -> i32 {1}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                shared_secret: mc_types::get_byte_array(&mut data)?,
                verify_token: mc_types::get_byte_array(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.shared_secret.clone()));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.verify_token.clone()));

            data
        }

    }

    pub struct PluginResponse {
        pub message_id: i32,
        pub successful: bool,
        pub data: Vec<u8>,
    }

    impl Packet for PluginResponse {

        fn packet_id() -> i32 {2}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                message_id: mc_types::get_var_int(&mut data)?,
                successful: mc_types::get_bool(&mut data),
                data: data.clone(),
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_var_int(self.message_id));
            data.append(&mut mc_types::convert_bool(self.successful));
            data.append(&mut self.data.clone());

            data
        }

    }

    pub struct Acknowledged {}

    impl Packet for Acknowledged {

        fn packet_id() -> i32 {3}

        fn get(_data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {})
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));

            data
        }

    }

    pub struct CookieResponse {
        pub key: String,
        pub has_payload: bool,
        pub payload_load: i32,
        pub payload: Vec<u8>,
    }

    impl Packet for CookieResponse {

        fn packet_id() -> i32 {4}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                key: mc_types::get_string(&mut data)?,
                has_payload: mc_types::get_bool(&mut data),
                payload_load: mc_types::get_var_int(&mut data)?,
                payload: data.clone(),
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.key));
            data.append(&mut mc_types::convert_bool(self.has_payload));
            data.append(&mut mc_types::convert_var_int(self.payload_load));
            data.append(&mut self.payload.clone());

            data
        }

    }

}
