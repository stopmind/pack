use crate::core::BlockType::{Directory, File};
use crate::core::{BlockHeader, BlockType, DirectoryEntry, Header, HEADER_MAGIC, SUPPORTED_VERSION};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::fs::{create_dir, create_dir_all, remove_dir_all, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use zerocopy::{try_transmute, IntoBytes};

struct Frame<T: Read> {
    read: T,
    offset: usize,
    end: usize,
}

impl<'a, T: Read> Frame<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let max_size = self.end - self.offset;

        if max_size == 0 {
            return Ok(0);
        }

        let size = if buf.len() > max_size {
            self.read.read(&mut buf[..max_size])
        } else {
            self.read.read(buf)
        }?;

        self.offset += size;

        Ok(size)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.offset + buf.len() > self.end {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof))
        }

        self.offset += buf.len();
        self.read.read_exact(buf)
    }
}

fn read_block<R: Read + Seek>(mut read: R, offset: u32) -> Result<(BlockType, Frame<R>)> {
    let mut header = [0u8; size_of::<BlockHeader>()];

    read.seek(SeekFrom::Start(offset as u64))?;

    read.read_exact(&mut header)?;
    let header: BlockHeader = try_transmute!(header)
        .map_err(|e| anyhow!("Failed read block header: {:?}", e))?;

    let offset = offset as usize + size_of::<BlockHeader>();
    let frame = Frame {
        read,
        offset,
        end: offset + header.size.get() as usize,
    };

    Ok((header.block_type, frame))
}

fn read_directory(mut data: Frame<impl Read>) -> Result<HashMap<String, u32>> {
    let mut directory = HashMap::new();

    let mut entry = DirectoryEntry {
        offset: 0.into(),
        name_size: 0.into(),
    };

    loop {
        if let Err(_) = data.read_exact(entry.as_mut_bytes()) {
            break;
        }

        let mut string = vec![0u8; entry.name_size.get() as usize];
        data.read_exact(string.as_mut_slice())?;

        directory.insert(String::from_utf8(string)?, entry.offset.into());
    }

    Ok(directory)
}

fn unpack_tree(mut read: impl Read + Seek, root: u32, mut path: PathBuf) -> Result<()> {
    let mut buffer = vec![0u8; 5 * 1024 * 1024];

    let (block_type, content) = read_block(&mut read, root)?;

    if block_type != Directory {
        bail!("Root must be a directory");
    }


    let _ = remove_dir_all(&path);
    create_dir_all(&path)?;

    let mut queue = vec![
        read_directory(content)?
            .into_iter()
            .collect::<Vec<(String, u32)>>()
    ];

    'dirs:
    while let Some(directory) = queue.last_mut() {
        while let Some((name, offset)) = directory.pop() {
            path.push(name);

            let (block_type, mut content) = read_block(&mut read, offset)?;
            match block_type {
                Directory => {
                    create_dir(&path)?;

                    queue.push(
                        read_directory(content)?
                            .into_iter()
                            .collect::<Vec<(String, u32)>>()
                    );

                    continue 'dirs;
                }
                File => {
                    let mut file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(&path)?;

                    loop {
                        let chunk_size = content.read(buffer.as_mut_slice())?;
                        if chunk_size == 0 {
                            break;
                        }

                        file.write(&buffer[..chunk_size])?;
                    }

                    path.pop();
                }
            }
        }

        path.pop();
        queue.pop();
    }

    Ok(())
}

pub fn unpack(file: impl AsRef<Path>, out: impl Into<PathBuf>) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(file)?;

    let mut header = Header {
        magic: 0.into(),
        version: 0,
        root_offset: 0.into(),
    };

    file.read_exact(header.as_mut_bytes())?;

    if header.magic != HEADER_MAGIC {
        bail!("Invalid magic number");
    }
    if header.version != SUPPORTED_VERSION {
        bail!("Unsupported version");
    }

    unpack_tree(file, header.root_offset.get(), out.into())?;

    Ok(())
}
