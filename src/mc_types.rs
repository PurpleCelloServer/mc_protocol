// Yeahbut December 2023

use std::error::Error;
use std::fmt;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const VERSION_NAME: &str = "1.19.4";
pub const VERSION_PROTOCOL: i32 = 762;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

// enum PacketError

#[derive(Debug)]
pub enum PacketError {
    ValueTooLarge,
    RanOutOfBytes,
    InvalidPacketId,
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
        }
    }
}

impl Error for PacketError {}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub text: String,
}

// trait PacketType

// trait Packet

#[async_trait]
pub trait Packet: Sized {
    fn packet_id() -> i32;
    fn get(data: &mut Vec<u8>) -> Result<Self>;
    fn convert(&self) -> Vec<u8>;

    async fn read(stream: &mut OwnedReadHalf) -> Result<Self> {
        let mut data = read_data(stream).await?;
        let packet_id = get_var_int(&mut data)?;
        if packet_id == Self::packet_id() {
            return Ok(Self::get(&mut data)?)
        } else {
            return Err(Box::new(PacketError::InvalidPacketId))
        }
    }

    async fn write(&self, stream: &mut OwnedWriteHalf) -> Result<()> {
        write_data(stream, &mut self.convert()).await
    }
}

pub async fn read_data(stream: &mut OwnedReadHalf) -> Result<Vec<u8>> {
    let length = read_var_int_stream(stream).await? as usize;

    let mut buffer: Vec<u8> = vec![0; length];
    stream.read_exact(&mut buffer).await?;

    Ok(buffer)
}
pub async fn write_data(
    stream: &mut OwnedWriteHalf,
    data: &mut Vec<u8>,
) -> Result<()> {
    let mut out_data = convert_var_int(data.len() as i32);
    out_data.append(data);

    stream.write_all(&out_data).await?;

    Ok(())
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

// enum MCTypes

// pub enum MCTypes {
//     Boolean,
//     Byte,
//     UnsignedByte,
//     Short,
//     UnsignedShort,
//     Int,
//     Long,
//     Float,
//     Double,
//     String,
//     VarInt,
//     VarLong,
//     Uuid,
//     Optional,
//     Array,
//     ByteArray,
// }

pub enum MCTypes {
    Boolean,
    Byte,
    UnsignedByte,
    Short,
    UnsignedShort,
    Int,
    Long,
    Float,
    Double,
    String,
    // Chat,
    Json,
    Identifier,
    VarInt,
    VarLong,
    // EntityMetadata,
    // Slot,
    // NBTTag,
    Position,
    Angle,
    Uuid,
    // Optional,
    // Array,
    // Enum,
    ByteArray,
}

// fn get_boolean
pub fn get_boolean(data: &mut Vec<u8>) -> Result<bool> {
    if data.len() < std::mem::size_of::<u8>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(u8::from_be_bytes(
        data.drain(0..std::mem::size_of::<u8>()).as_slice().try_into()?) != 0)
}
// fn convert_boolean
pub fn convert_boolean(value: bool) -> Vec<u8> {
    (value as u8).to_be_bytes().to_vec()
}

// fn get_byte
pub fn get_byte(data: &mut Vec<u8>) -> Result<i8> {
    if data.len() < std::mem::size_of::<i8>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(i8::from_be_bytes(
        data.drain(0..std::mem::size_of::<i8>()).as_slice().try_into()?))
}
// fn convert_byte
pub fn convert_byte(value: i8) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_unsigned_byte
pub fn get_unsigned_byte(data: &mut Vec<u8>) -> Result<u8> {
    if data.len() < std::mem::size_of::<u8>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(u8::from_be_bytes(
        data.drain(0..std::mem::size_of::<u8>()).as_slice().try_into()?))
}
// fn convert_unsigned_byte
pub fn convert_unsigned_byte(value: u8) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_short
pub fn get_short(data: &mut Vec<u8>) -> Result<i16> {
    if data.len() < std::mem::size_of::<i16>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(i16::from_be_bytes(
        data.drain(0..std::mem::size_of::<i16>()).as_slice().try_into()?))
}
// fn convert_short
pub fn convert_short(value: i16) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_unsigned_short
pub fn get_unsigned_short(data: &mut Vec<u8>) -> Result<u16> {
    if data.len() < std::mem::size_of::<u16>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(u16::from_be_bytes(
        data.drain(0..std::mem::size_of::<u16>()).as_slice().try_into()?))
}
// fn convert_unsigned_short
pub fn convert_unsigned_short(value: u16) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_int
pub fn get_int(data: &mut Vec<u8>) -> Result<i32> {
    if data.len() < std::mem::size_of::<i32>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(i32::from_be_bytes(
        data.drain(0..std::mem::size_of::<i32>()).as_slice().try_into()?))
}
// fn convert_int
pub fn convert_int(value: i32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_long
pub fn get_long(data: &mut Vec<u8>) -> Result<i64> {
    if data.len() < std::mem::size_of::<i64>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(i64::from_be_bytes(
        data.drain(0..std::mem::size_of::<i64>()).as_slice().try_into()?))
}
// fn convert_long
pub fn convert_long(value: i64) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_float
pub fn get_float(data: &mut Vec<u8>) -> Result<f32> {
    if data.len() < std::mem::size_of::<f32>() {
        return Err(Box::new(PacketError::RanOutOfBytes));
    }
    Ok(f32::from_be_bytes(
        data.drain(0..std::mem::size_of::<f32>()).as_slice().try_into()?))
}
// fn convert_float
pub fn convert_float(value: f32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_double
pub fn get_double(data: &mut Vec<u8>) -> Result<f64> {
    if data.len() < std::mem::size_of::<f64>() {
        return Err(Box::new(PacketError::RanOutOfBytes));
    }
    Ok(f64::from_be_bytes(
        data.drain(0..std::mem::size_of::<f64>()).as_slice().try_into()?))
}
// fn convert_double
pub fn convert_double(value: f64) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// fn get_string
pub fn get_string(data: &mut Vec<u8>) -> Result<String> {
    let length = get_var_int(data)? as usize;
    let buffer = data[..length].to_vec();
    for _ in 0..length { data.remove(0); }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}
// fn convert_string
pub fn convert_string(s: &str) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s.as_bytes().to_vec());
    data
}

// // fn get_chat <- nbt
// pub fn get_chat(data: &mut Vec<u8>) -> Result<String> {
//     get_nbt_tag(data)
// }
// // fn convert_chat <- nbt
// pub fn convert_chat(value: String) -> Vec<u8> {
//     convert_nbt_tag(value)
// }

// fn get_var_int
pub fn get_var_int(data: &mut Vec<u8>) -> Result<i32> {
    Ok(get_var(data, 32)? as i32)
}
// fn convert_var_int
pub fn convert_var_int(value: i32) -> Vec<u8> {
    convert_var(value as i64)
}

// fn get_var_long
pub fn get_var_long(data: &mut Vec<u8>) -> Result<i64> {
    get_var(data, 64)
}
// fn convert_var_long
pub fn convert_var_long(value: i64) -> Vec<u8> {
    convert_var(value)
}

// fn get_var
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
// fn convert_var
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

// fn get_entity_metadata

// fn convert_entity_metadata

// struct MCSlot

// fn get_slot

// fn convert_slot

// fn get_nbt_tag

// fn convert_nbt_tag

// struct MCPosition
pub struct MCPosition {
    x: i32,
    z: i32,
    y: i16,
}
// fn get_position
pub fn get_position(data: &mut Vec<u8>) -> Result<MCPosition> {
    let pos = get_long(data)?;
    Ok(MCPosition {
        x: (pos >> 38) as i32,
        z: (pos << 26 >> 38) as i32,
        y: (pos << 52 >> 52) as i16,
    })
}
// fn convert_position
pub fn convert_position(value: MCPosition) -> Vec<u8> {
    let pos: u64 =
        ((value.x as u64 & 0x3FFFFFF) << 38) |
        ((value.z as u64 & 0x3FFFFFF) << 12) |
        (value.y as u64 & 0xFFF);
    convert_long(pos as i64)
}

// fn get_uuid
pub fn get_uuid(data: &mut Vec<u8>) -> Result<u128> {
    if data.len() < std::mem::size_of::<u128>() {
        return Err(Box::new(PacketError::RanOutOfBytes))
    }
    Ok(u128::from_be_bytes(
        data.drain(0..std::mem::size_of::<u128>()).as_slice().try_into()?))
}
// fn convert_uuid
pub fn convert_uuid(value: u128) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

// trait MCTypeOptional

// trait MCTypeArray

pub trait MCTypeArray: Sized {
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

// trait MCTypeEnum

// fn get_byte_array
pub fn get_byte_array(data: &mut Vec<u8>) -> Result<Vec<u8>> {
    let length = get_var_int(data)? as usize;
    let buffer = data[..length].to_vec();
    for _ in 0..length { data.remove(0); }
    Ok(buffer)
}
// fn convert_byte_array
pub fn convert_byte_array(mut s: &mut Vec<u8>) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s);
    data
}
