// Yeahbut December 2023

use std::error::Error;
use std::fmt;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::{EncodePublicKey, DecodePublicKey};
use rand::Rng;

use crate::login;
use crate::encrypt;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const VERSION_NAME: &str = "1.19.4";
pub const VERSION_PROTOCOL: i32 = 762;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Debug)]
pub enum PacketError {
    ValueTooLarge,
    RanOutOfBytes,
    InvalidPacketId,
    EncryptionError,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketError::ValueTooLarge =>
                write!(f, "VarInt value is too large"),
            PacketError::RanOutOfBytes =>
                write!(f, "Ran out of bytes while reading VarInt"),
            PacketError::InvalidPacketId =>
                write!(f, "Invalid packet id"),
            PacketError::EncryptionError =>
                write!(f, "Encryption Error"),
        }
    }
}

impl Error for PacketError {}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub text: String,
}

pub struct ProtocolConnection<'a> {
    pub stream_read: &'a mut OwnedReadHalf,
    pub stream_write: &'a mut OwnedWriteHalf,
    rsa_private_key: Option<RsaPrivateKey>,
    rsa_public_key: Option<RsaPublicKey>,
    aes_encryption_key: Option<[u8; 16]>,
    verify_token: Option<[u8; 16]>,
}

impl<'a> ProtocolConnection<'a> {
    pub fn new(
        stream_read: &'a mut OwnedReadHalf,
        stream_write: &'a mut OwnedWriteHalf,
    ) -> Self {
        ProtocolConnection {
            stream_read,
            stream_write,
            rsa_private_key: None,
            rsa_public_key: None,
            aes_encryption_key: None,
            verify_token: None,
        }
    }

    pub async fn read_data(&mut self) -> Result<Vec<u8>> {
        match self.aes_encryption_key {
            Some(aes_key) => {
                let mut buffer: Vec<u8> = vec![0; 16];
                self.stream_read.read_exact(&mut buffer).await?;
                buffer = encrypt::decrypt_aes(
                    &aes_key, buffer[0..16].try_into().unwrap());
                let raw_length = read_var_int_vec(&mut buffer)?;
                let length =
                    if (raw_length - buffer.len() as i32) % 16 == 0 {
                        (raw_length - buffer.len() as i32) / 16
                    } else {
                        ((raw_length - buffer.len() as i32) / 16) + 1
                    };

                for _ in 0..length {
                    let mut block: Vec<u8> = vec![0; 16];
                    self.stream_read.read_exact(&mut block).await?;
                    buffer.append(&mut block);
                }

                Ok(buffer)
            },
            None => {
                let length = read_var_int_stream(
                    self.stream_read).await? as usize;

                let mut buffer: Vec<u8> = vec![0; length];
                self.stream_read.read_exact(&mut buffer).await?;

                Ok(buffer)
            }
        }
    }

    pub async fn write_data(
        &mut self,
        data: &mut Vec<u8>,
    ) -> Result<()> {
        let mut out_data = convert_var_int(data.len() as i32);
        out_data.append(data);
        match self.aes_encryption_key {
            Some(aes_key) => {
                let length =
                    if (data.len() as i32) % 16 == 0 {
                        (data.len() as i32) / 16
                    } else {
                        ((data.len() as i32) / 16) + 1
                    };

                for _ in 0..length {
                    let mut block: Vec<u8> = out_data[0..16].to_vec();
                    block = encrypt::encrypt_aes(
                        &aes_key, block[0..16].try_into().unwrap());
                    self.stream_write.write_all(&block).await?;
                }


                Ok(())
            },
            None => {
                self.stream_write.write_all(&out_data).await?;

                Ok(())
            }
        }
    }

    pub fn create_encryption_request(
        &mut self,
        private_key: RsaPrivateKey,
    ) -> Result<login::clientbound::EncryptionRequest> {
        match self.rsa_private_key {
            Some(_) => {},
            None => {
                let public_key = RsaPublicKey::from(&private_key);
                let mut rng = rand::thread_rng();
                self.rsa_private_key = Some(private_key);
                self.rsa_public_key = Some(public_key);
                self.verify_token = Some(rng.gen());
            }
        };
        match &self.rsa_public_key {
            Some (key) => {
                match &self.verify_token {
                    Some (token) =>
                        Ok(login::clientbound::EncryptionRequest {
                            server_id: "".to_string(),
                            public_key: key
                                .to_public_key_der()?
                                .as_ref()
                                .to_vec(),
                            verify_token: token[0..16].to_vec(),
                        }),
                    None => Err(Box::new(PacketError::EncryptionError))
                }
            },
            None => Err(Box::new(PacketError::EncryptionError))
        }
    }

    pub fn handle_encryption_request(
        &mut self,
        request: login::clientbound::EncryptionRequest,
    ) -> Result<login::serverbound::EncryptionResponse> {
        self.rsa_public_key = Some(
            RsaPublicKey::from_public_key_der(&request.public_key)?);
        let mut rng = rand::thread_rng();
        self.aes_encryption_key = Some(rng.gen());
        match self.aes_encryption_key {
            Some(key) => {
                match &self.rsa_public_key {
                    Some(public_key) => {
                        Ok(login::serverbound::EncryptionResponse {
                            shared_secret: encrypt::encrypt_rsa(
                                public_key, &key)?,
                            verify_token: encrypt::encrypt_rsa(
                                public_key,
                                request.verify_token[0..16]
                                    .try_into()
                                    .unwrap(),
                            )?,
                        })
                    },
                    None => Err(Box::new(PacketError::EncryptionError))
                }
            },
            None => Err(Box::new(PacketError::EncryptionError))
        }
    }

    pub fn handle_encryption_response(
        &mut self,
        response: login::serverbound::EncryptionResponse,
    ) -> Result<()> {
        match &self.verify_token {
            Some (token) => {
                match &self.rsa_private_key {
                    Some (private_key) => {
                        if &encrypt::decrypt_rsa(
                            &private_key,
                            response.verify_token.as_slice()
                        )? == token {
                            self.aes_encryption_key =
                                Some(encrypt::decrypt_rsa(
                                    &private_key,
                                    response.shared_secret.as_slice()
                                )?[0..16].try_into().unwrap());
                            Ok(())
                        } else {
                            Err(Box::new(PacketError::EncryptionError))
                        }
                    }
                    None => Err(Box::new(PacketError::EncryptionError))
                }
            }
            None => Err(Box::new(PacketError::EncryptionError))
        }
    }
}

#[async_trait]
pub trait Packet: Sized {
    fn packet_id() -> i32;
    fn get(data: &mut Vec<u8>) -> Result<Self>;
    fn convert(&self) -> Vec<u8>;

    async fn read(conn: &mut ProtocolConnection<'_>) -> Result<Self> {
        let mut data = conn.read_data().await?;
        let packet_id = get_var_int(&mut data)?;
        if packet_id == Self::packet_id() {
            return Ok(Self::get(&mut data)?)
        } else {
            return Err(Box::new(PacketError::InvalidPacketId))
        }
    }

    async fn write(&self, conn: &mut ProtocolConnection<'_>) -> Result<()> {
        conn.write_data(&mut self.convert()).await
    }
}

async fn read_var_int_stream(stream: &mut OwnedReadHalf) -> Result<i32> {
    let mut data: Vec<u8> = vec![];

    loop {
        let current_byte = stream.read_u8().await?;

        data.append(&mut vec![current_byte]);

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }
    }

    let varint = get_var_int(&mut data)?;

    Ok(varint)
}
fn read_var_int_vec(stream: &mut Vec<u8>) -> Result<i32> {
    let mut data: Vec<u8> = vec![];

    loop {
        let current_byte = stream.remove(0);

        data.append(&mut vec![current_byte]);

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }
    }

    let varint = get_var_int(&mut data)?;

    Ok(varint)
}

pub trait PacketArray: Sized {
    fn get(data: &mut Vec<u8>) -> Result<Self>;
    fn convert(&self) -> Vec<u8>;

    fn get_array(data: &mut Vec<u8>) -> Result<Vec<Self>> {
        let length = get_var_int(data)?;
        let mut out_data: Vec<Self> = vec![];
        for _ in 0..length {
            out_data.push(Self::get(data)?);
        }
        Ok(out_data)
    }

    fn convert_array(array: &mut Vec<Self>) -> Vec<u8> {
        let length = array.len() as i32;
        let mut data: Vec<u8> = convert_var_int(length);
        for element in array {
            data.append(&mut Self::convert(element));
        }
        data
    }
}

pub fn get_bool(data: &mut Vec<u8>) -> bool {
    data.remove(0) != 0
}
pub fn convert_bool(value: bool) -> Vec<u8> {
    vec![value as u8]
}

pub fn get_u8(data: &mut Vec<u8>) -> u8 {
    data.remove(0)
}
pub fn convert_u8(value: u8) -> Vec<u8> {
    vec![value]
}

pub fn get_i8(data: &mut Vec<u8>) -> i8 {
    get_u8(data) as i8
}
pub fn convert_i8(value: i8) -> Vec<u8> {
    convert_u8(value as u8)
}

pub fn get_u16(data: &mut Vec<u8>) -> u16 {
    ((data.remove(0) as u16) << 8) |
    (data.remove(0) as u16)
}
pub fn convert_u16(value: u16) -> Vec<u8> {
    vec![
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i16(data: &mut Vec<u8>) -> i16 {
    get_u16(data) as i16
}
pub fn convert_i16(value: i16) -> Vec<u8> {
    convert_u16(value as u16)
}

pub fn get_u32(data: &mut Vec<u8>) -> u32 {
    ((data.remove(0) as u32) << 24) |
    ((data.remove(0) as u32) << 16) |
    ((data.remove(0) as u32) << 8) |
    (data.remove(0) as u32)
}
pub fn convert_u32(value: u32) -> Vec<u8> {
    vec![
        ((value & 0xFF0000) >> 24) as u8,
        ((value & 0xFF0000) >> 16) as u8,
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i32(data: &mut Vec<u8>) -> i32 {
    get_u32(data) as i32
}
pub fn convert_i32(value: i32) -> Vec<u8> {
    convert_u32(value as u32)
}

pub fn get_f32(data: &mut Vec<u8>) -> f32 {
    get_u32(data) as f32
}
pub fn convert_f32(value: f32) -> Vec<u8> {
    convert_u32(value as u32)
}

pub fn get_u64(data: &mut Vec<u8>) -> u64 {
    ((data.remove(0) as u64) << 56) |
    ((data.remove(0) as u64) << 48) |
    ((data.remove(0) as u64) << 40) |
    ((data.remove(0) as u64) << 32) |
    ((data.remove(0) as u64) << 24) |
    ((data.remove(0) as u64) << 16) |
    ((data.remove(0) as u64) << 8) |
    (data.remove(0) as u64)
}
pub fn convert_u64(value: u64) -> Vec<u8> {
    vec![
        ((value & 0xFF00000000000000) >> 56) as u8,
        ((value & 0xFF000000000000) >> 48) as u8,
        ((value & 0xFF0000000000) >> 40) as u8,
        ((value & 0xFF00000000) >> 32) as u8,
        ((value & 0xFF000000) >> 24) as u8,
        ((value & 0xFF0000) >> 16) as u8,
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i64(data: &mut Vec<u8>) -> i64 {
    get_u64(data) as i64
}
pub fn convert_i64(value: i64) -> Vec<u8> {
    convert_u64(value as u64)
}

pub fn get_f64(data: &mut Vec<u8>) -> f64 {
    get_u64(data) as f64
}
pub fn convert_f64(value: f64) -> Vec<u8> {
    convert_u64(value as u64)
}

pub fn get_uuid(data: &mut Vec<u8>) -> u128 {
    ((data.remove(0) as u128) << 120) |
    ((data.remove(0) as u128) << 112) |
    ((data.remove(0) as u128) << 104) |
    ((data.remove(0) as u128) << 96) |
    ((data.remove(0) as u128) << 88) |
    ((data.remove(0) as u128) << 80) |
    ((data.remove(0) as u128) << 72) |
    ((data.remove(0) as u128) << 64) |
    ((data.remove(0) as u128) << 56) |
    ((data.remove(0) as u128) << 48) |
    ((data.remove(0) as u128) << 40) |
    ((data.remove(0) as u128) << 32) |
    ((data.remove(0) as u128) << 24) |
    ((data.remove(0) as u128) << 16) |
    ((data.remove(0) as u128) << 8) |
    (data.remove(0) as u128)
}
pub fn convert_uuid(value: u128) -> Vec<u8> {
    vec![
        ((value & 0xFF000000000000000000000000000000) >> 120) as u8,
        ((value & 0xFF0000000000000000000000000000) >> 112) as u8,
        ((value & 0xFF00000000000000000000000000) >> 104) as u8,
        ((value & 0xFF000000000000000000000000) >> 96) as u8,
        ((value & 0xFF0000000000000000000000) >> 88) as u8,
        ((value & 0xFF00000000000000000000) >> 80) as u8,
        ((value & 0xFF000000000000000000) >> 72) as u8,
        ((value & 0xFF0000000000000000) >> 64) as u8,
        ((value & 0xFF00000000000000) >> 56) as u8,
        ((value & 0xFF000000000000) >> 48) as u8,
        ((value & 0xFF0000000000) >> 40) as u8,
        ((value & 0xFF00000000) >> 32) as u8,
        ((value & 0xFF000000) >> 24) as u8,
        ((value & 0xFF0000) >> 16) as u8,
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_var_int(data: &mut Vec<u8>) -> Result<i32> {
    Ok(get_var(data, 32)? as i32)
}
pub fn convert_var_int(value: i32) -> Vec<u8> {
    convert_var(value as i64)
}

pub fn get_var_long(data: &mut Vec<u8>) -> Result<i64> {
    get_var(data, 64)
}
pub fn convert_var_long(value: i64) -> Vec<u8> {
    convert_var(value)
}

fn get_var(data: &mut Vec<u8>, size: u8) -> Result<i64> {
    let mut value: i64 = 0;
    let mut position: u8 = 0;

    loop {
        if data.is_empty() {
            return Err(Box::new(PacketError::RanOutOfBytes));
        }

        let current_byte = data.remove(0);
        value |= ((current_byte & SEGMENT_BITS) as i64) << position;

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }

        position += 7;

        if position >= size {
            return Err(Box::new(PacketError::ValueTooLarge));
        }
    }

    Ok(value)
}
fn convert_var(mut value: i64) -> Vec<u8> {
    let mut data: Vec<u8> = vec![];
    loop {
        if (value & !(SEGMENT_BITS as i64)) == 0 {
            data.append(&mut vec![value as u8]);
            return data;
        }
        data.append(
            &mut vec![(value & (SEGMENT_BITS as i64)) as u8 | CONTINUE_BIT]);
        value >>= 7;
    }
}

pub fn get_string(data: &mut Vec<u8>) -> Result<String> {
    let length = get_var_int(data)? as usize;
    let buffer = data[..length].to_vec();
    for _ in 0..length { data.remove(0); }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}
pub fn convert_string(s: &str) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s.as_bytes().to_vec());
    data
}

pub fn get_byte_array(data: &mut Vec<u8>) -> Result<Vec<u8>> {
    let length = get_var_int(data)? as usize;
    let buffer = data[..length].to_vec();
    for _ in 0..length { data.remove(0); }
    Ok(buffer)
}
pub fn convert_byte_array(mut s: &mut Vec<u8>) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s);
    data
}
