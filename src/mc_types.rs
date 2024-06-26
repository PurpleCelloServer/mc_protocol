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
use crypto::digest::Digest;
use sha1::Sha1;
use num_bigint::BigInt;

use crate::login;
use crate::encrypt::{self, McCipher};
use crate::play::Play;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const VERSION_NAME: &str = "1.21";
pub const VERSION_PROTOCOL: i32 = 767;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Debug)]
pub enum PacketError {
    ValueTooLarge,
    RanOutOfBytes,
    InvalidPacketId,
    InvalidUUIDString,
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
            PacketError::InvalidUUIDString =>
                write!(f, "Invalid UUID format"),
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

#[async_trait]
pub trait ProtocolRead {
    async fn read_data(&mut self) -> Result<Vec<u8>>;
}

#[async_trait]
pub trait ProtocolWrite {
    async fn write_data(&mut self, data: &mut Vec<u8>) -> Result<()>;
}

pub struct ProtocolConnection<'a> {
    pub stream_read: &'a mut OwnedReadHalf,
    pub stream_write: &'a mut OwnedWriteHalf,
    rsa_private_key: Option<RsaPrivateKey>,
    rsa_public_key: Option<RsaPublicKey>,
    aes_cipher: Option<McCipher>,
    verify_token: Option<[u8; 16]>,
    server_id: String
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
            aes_cipher: None,
            verify_token: None,
            server_id: "".to_string(),
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
                            server_id: self.server_id.clone(),
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
        self.server_id = request.server_id;
        self.rsa_public_key = Some(
            RsaPublicKey::from_public_key_der(&request.public_key)?);
        self.aes_cipher = Some(McCipher::create());
        match &self.aes_cipher {
            Some(aes_cipher) => {
                match &self.rsa_public_key {
                    Some(public_key) => {
                        Ok(login::serverbound::EncryptionResponse {
                            shared_secret: aes_cipher
                                .get_encrypted_key(public_key)?,
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
                            self.aes_cipher =
                                Some(McCipher::create_with_encrypted_key(
                                    private_key,
                                    response.shared_secret.as_slice(),
                                )?);
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

    pub async fn forward_play(
        &mut self,
        other: &mut ProtocolConnection<'_>,
    ) -> Result<()> {
        loop {
            let packet = Play::read(self).await?;
            match packet {
                Play::PlayPacket(packet) => packet.write(other).await?,
            };
        }
    }

    pub fn split_conn(
        &mut self
    ) -> Result<(WriteHaftProtocolConnection, ReadHaftProtocolConnection)> {
        Ok((WriteHaftProtocolConnection {
            stream_write: &mut self.stream_write,
            aes_cipher: self.aes_cipher.clone(),
        },
        ReadHaftProtocolConnection {
            stream_read: &mut self.stream_read,
            aes_cipher: self.aes_cipher.clone(),
        }))
    }

    pub async fn server_id_hash(&self) -> Result<String> {
        let hash_data = match &self.aes_cipher {
            Some(aes_cipher) => match &self.rsa_public_key {
                Some(key) => [
                    self.server_id.as_bytes(),
                    &aes_cipher.key,
                    key.to_public_key_der()?.as_ref(),
                ].concat(),
                None => return Err(Box::new(PacketError::EncryptionError))
            },
            None => return Err(Box::new(PacketError::EncryptionError))
        };
        let hash = BigInt::from_signed_bytes_be(
            &Sha1::digest(hash_data)).to_str_radix(16);
        Ok(hash)
    }
}

unsafe impl<'a> Send for ProtocolConnection<'a> {}

#[async_trait]
impl<'a> ProtocolRead for ProtocolConnection<'a> {
    async fn read_data(&mut self) -> Result<Vec<u8>> {
        match &mut self.aes_cipher {
            Some(aes_cipher) => {
                let length = read_var_int_stream_encrypted(
                    self.stream_read, aes_cipher).await? as usize;

                let mut buffer: Vec<u8> = vec![0; length];
                self.stream_read.read_exact(&mut buffer).await?;
                Ok(aes_cipher.decrypt_aes(buffer))
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
}

#[async_trait]
impl<'a> ProtocolWrite for ProtocolConnection<'a> {
    async fn write_data(&mut self, data: &mut Vec<u8>) -> Result<()> {
        let mut out_data = convert_var_int(data.len() as i32);
        out_data.append(data);
        match &mut self.aes_cipher {
            Some(aes_cipher) => {
                self.stream_write.write_all(
                    &aes_cipher.encrypt_aes(out_data)).await?;

                Ok(())
            },
            None => {
                self.stream_write.write_all(&out_data).await?;

                Ok(())
            }
        }
    }
}

pub struct WriteHaftProtocolConnection<'a> {
    pub stream_write: &'a mut OwnedWriteHalf,
    aes_cipher: Option<McCipher>,
}

impl<'a> WriteHaftProtocolConnection<'a> {
    pub fn new(
        stream_write: &'a mut OwnedWriteHalf,
    ) -> Self {
        WriteHaftProtocolConnection {
            stream_write,
            aes_cipher: None,
        }
    }
}

unsafe impl<'a> Send for WriteHaftProtocolConnection<'a> {}

#[async_trait]
impl<'a> ProtocolWrite for WriteHaftProtocolConnection<'a> {
    async fn write_data(&mut self, data: &mut Vec<u8>) -> Result<()> {
        let mut out_data = convert_var_int(data.len() as i32);
        out_data.append(data);
        match &mut self.aes_cipher {
            Some(aes_cipher) => {
                self.stream_write.write_all(
                    &aes_cipher.encrypt_aes(out_data)).await?;

                Ok(())
            },
            None => {
                self.stream_write.write_all(&out_data).await?;

                Ok(())
            }
        }
    }
}

pub struct ReadHaftProtocolConnection<'a> {
    pub stream_read: &'a mut OwnedReadHalf,
    aes_cipher: Option<McCipher>,
}

impl<'a> ReadHaftProtocolConnection<'a> {
    pub fn new(
        stream_read: &'a mut OwnedReadHalf,
    ) -> Self {
        ReadHaftProtocolConnection {
            stream_read,
            aes_cipher: None,
        }
    }

    pub async fn forward_play<T: ProtocolWrite + Send>(
        &mut self,
        other: &mut T,
    ) -> Result<()> {
        loop {
            let packet = Play::read(self).await?;
            match packet {
                Play::PlayPacket(packet) => packet.write(other).await?,
            };
        }
    }
}

unsafe impl<'a> Send for ReadHaftProtocolConnection<'a> {}

#[async_trait]
impl<'a> ProtocolRead for ReadHaftProtocolConnection<'a> {
    async fn read_data(&mut self) -> Result<Vec<u8>> {
        match &mut self.aes_cipher {
            Some(aes_cipher) => {
                let length = read_var_int_stream_encrypted(
                    self.stream_read, aes_cipher).await? as usize;

                let mut buffer: Vec<u8> = vec![0; length];
                self.stream_read.read_exact(&mut buffer).await?;
                Ok(aes_cipher.decrypt_aes(buffer))
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
}

#[async_trait]
pub trait Packet: Sized {
    fn packet_id() -> i32;
    fn get(data: &mut Vec<u8>) -> Result<Self>;
    fn convert(&self) -> Vec<u8>;

    async fn read<T: ProtocolRead + Send>(conn: &mut T) -> Result<Self> {
        let mut data = conn.read_data().await?;
        let packet_id = get_var_int(&mut data)?;
        if packet_id == Self::packet_id() {
            return Ok(Self::get(&mut data)?)
        } else {
            return Err(Box::new(PacketError::InvalidPacketId))
        }
    }

    async fn write<T: ProtocolWrite + Send>(&self, conn: &mut T) -> Result<()> {
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
async fn read_var_int_stream_encrypted(
    stream: &mut OwnedReadHalf,
    cipher: &mut McCipher,
) -> Result<i32> {
    let mut data: Vec<u8> = vec![];

    loop {
        let encrypted_byte = stream.read_u8().await?;
        let current_byte = cipher.decrypt_aes(vec![encrypted_byte])[0];

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
pub fn uuid_u128_to_string(uuid: u128) -> String {
    let uuid_bytes = convert_uuid(uuid);
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        get_u32(&mut vec![
            uuid_bytes[0],
            uuid_bytes[1],
            uuid_bytes[2],
            uuid_bytes[3],
        ]),
        get_u16(&mut vec![uuid_bytes[4], uuid_bytes[5]]),
        get_u16(&mut vec![uuid_bytes[6], uuid_bytes[7]]),
        get_u16(&mut vec![uuid_bytes[8], uuid_bytes[9]]),
        get_u64(&mut vec![
            0,
            0,
            uuid_bytes[10],
            uuid_bytes[11],
            uuid_bytes[12],
            uuid_bytes[13],
            uuid_bytes[14],
            uuid_bytes[15],
        ]),
    )
}
pub fn uuid_string_to_u128(uuid: &str) -> Result<u128> {
    let cleaned_uuid = uuid.replace("-", "");
    if cleaned_uuid.len() != 32 {
        return Err(Box::new(PacketError::InvalidUUIDString));
    }
    Ok(u128::from_str_radix(&cleaned_uuid, 16)?)
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
