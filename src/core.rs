use zerocopy::big_endian::{U16, U32};
use zerocopy::{FromBytes, Immutable, IntoBytes, TryFromBytes};

pub const HEADER_MAGIC: u16 = 0x10FA;
pub const SUPPORTED_VERSION: u8 = 0xEE;

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable)]
pub struct Header {
    pub magic: U16,
    pub version: u8,
    pub root_offset: U32
}

#[repr(u8)]
#[derive(TryFromBytes, IntoBytes, Immutable, Eq, PartialEq)]
pub enum BlockType {
    Directory = 0xDD,
    File = 0xFF,
}

#[repr(C)]
#[derive(TryFromBytes, IntoBytes, Immutable)]
pub struct BlockHeader {
    pub block_type: BlockType,
    pub size: U32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable)]
pub struct DirectoryEntry {
    pub offset: U32,
    pub name_size: U16,
}