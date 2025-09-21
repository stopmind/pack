use crate::core::BlockType::{Directory, File};
use crate::core::{BlockHeader, BlockType, DirectoryEntry, Header, HEADER_MAGIC, SUPPORTED_VERSION};
use anyhow::Result;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::SeekFrom::{Current, Start};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use zerocopy::IntoBytes;

struct Writer<T: Write + Seek> {
    base: T,
    buffer: Vec<u8>,
}

impl<T: Write + Seek> Writer<T> {
    fn new(mut base: T) -> Result<Self> {
        base.seek(Start(size_of::<Header>() as u64))?;
        Ok(Self {
            base,
            buffer: vec![0; 5 * 1024 * 1024]
        })
    }

    fn write_block(&mut self, block_type: BlockType, data: impl AsRef<[u8]>) -> Result<u32> {
        let data = data.as_ref();
        let offset = self.base.stream_position()? as u32;

        let header = BlockHeader {
            block_type,
            size: (data.len() as u32).into()
        };

        self.base.write(header.as_bytes())?;
        self.base.write(data)?;

        Ok(offset)
    }

    fn write_block_from_read(&mut self, block_type: BlockType, mut file: impl Read) -> Result<u32> {
        let offset = self.base.stream_position()?;

        let mut size = 0usize;

        self.base.seek(Current(size_of::<BlockHeader>() as i64))?;
        loop {
            let chunk_size = file.read(self.buffer.as_mut_slice())?;
            if chunk_size == 0 {
                break;
            }

            size += chunk_size;
            self.base.write(&self.buffer[..chunk_size])?;
        }

        let header = BlockHeader {
            block_type,
            size: (size as u32).into()
        };

        self.base.seek(Start(offset))?;
        self.base.write(header.as_bytes())?;
        self.base.seek(Current(size as i64))?;

        Ok(offset as u32)
    }

    fn write_header(&mut self, header: &Header) -> Result<()> {
        let offset = self.base.stream_position()?;

        self.base.seek(Start(0))?;
        self.base.write(header.as_bytes())?;
        self.base.seek(Start(offset))?;

        Ok(())
    }
}

fn write_tree<T: Write + Seek>(writer: &mut Writer<T>, directory: impl AsRef<Path>) -> Result<u32> {
    struct UnfinishedDirectory {
        name: String,
        map: HashMap<String, u32>,
        sub_directories: Vec<PathBuf>,
    }

    fn begin_directory<T: Write + Seek>(writer: &mut Writer<T>, directory: impl AsRef<Path>) -> Result<UnfinishedDirectory> {
        let directory = directory.as_ref();
        let mut unfinished_directory = UnfinishedDirectory {
            name: directory.file_name().unwrap().to_string_lossy().into_owned(),
            map: HashMap::new(),
            sub_directories: Vec::new()
        };

        for entry in directory.read_dir()? {
            if let Ok(entry) = entry {
                if entry.metadata()?.is_dir() {
                    unfinished_directory.sub_directories.push(entry.path());
                } else {
                    let file = OpenOptions::new()
                        .read(true)
                        .open(entry.path())?;

                    let offset = writer.write_block_from_read(File, file)?;

                    unfinished_directory.map.insert(entry.path().file_name().unwrap().to_string_lossy().into_owned(), offset);
                }
            }
        }

        Ok(unfinished_directory)
    }

    fn end_directory<T: Write + Seek>(writer: &mut Writer<T>, directory: UnfinishedDirectory) -> Result<(String, u32)> {
        let mut buffer = Vec::<u8>::with_capacity(1024);

        for (name, offset) in directory.map {
            let entry = DirectoryEntry {
                offset: offset.into(),
                name_size: (name.len() as u16).into(),
            };

            buffer.write(entry.as_bytes())?;
            buffer.write(name.as_bytes())?;
        }

        let offset = writer.write_block(Directory, buffer)?;
        Ok((directory.name, offset))
    }


    let mut directories = vec![begin_directory(writer, directory)?];

    loop {
        let directory = directories.last_mut().unwrap();

        if let Some(sub_directory) = directory.sub_directories.pop() {
            directories.push(begin_directory(writer, sub_directory)?);
            continue;
        }

        let directory = directories.pop().unwrap();
        let (name, offset) = end_directory(writer, directory)?;

        if let Some(parent) = directories.last_mut() {
            parent.map.insert(name, offset);
        } else {
            return Ok(offset);
        }
    }
}

pub fn pack(file: impl AsRef<Path>, directory: impl AsRef<Path>) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file)?;

    let mut writer = Writer::new(file)?;

    let root_offset = write_tree(&mut writer, directory)?;
    let header = Header {
        magic: HEADER_MAGIC.into(),
        version: SUPPORTED_VERSION,
        root_offset: root_offset.into(),
    };

    writer.write_header(&header)?;

    Ok(())
}
