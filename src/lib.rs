#![no_std]

use core::{fmt, result};
use embedded_io::{Read, Seek, SeekFrom, Write};

pub type Result<T> = result::Result<T, Error>;

fn err_parsing<T>(pos: u64, description: &'static str) -> Result<T> {
    Err(Error::parsing(pos, description))
}

fn err_describe<T>(description: &'static str) -> Result<T> {
    Err(Error::describe(description))
}

trait OrError<T> {
    fn or_describe(self, description: &'static str) -> Result<T>;
    fn or_parsing(self, pos: u64, description: &'static str) -> Result<T>;
}

impl<T, E> OrError<T> for result::Result<T, E> {
    fn or_describe(self, description: &'static str) -> Result<T> {
        self.or_else(|_| err_describe(description))
    }
    fn or_parsing(self, pos: u64, description: &'static str) -> Result<T> {
        self.or_else(|_| err_parsing(pos, description))
    }
}

#[derive(Debug)]
pub struct Error {
    pub position: Option<u64>,
    pub description: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.position {
            Some(pos) => write!(f, "at {}: {}", pos, self.description),
            None => write!(f, "{}", self.description),
        }
    }
}

impl Error {
    fn new(position: Option<u64>, description: &'static str) -> Error {
        Error { position, description }
    }

    fn parsing(pos: u64, description: &'static str) -> Error {
        Error::new(Some(pos), description)
    }

    fn describe(description: &'static str) -> Error {
        Error::new(None, description)
    }
}

pub fn merge_mp3s<'a, T>(rs: impl Iterator<Item = &'a mut T>, w: &mut impl Write, cb: fn(usize)) -> Result<()>
    where T: Read + Seek + 'a {

    for (i, r) in rs.enumerate() {
        read_to_first_header(r)?;
        copy(r, w)?;
        cb(i);
    }
    Ok(())
}

fn read_to_first_header(reader: &mut (impl Read + Seek)) -> Result<u64> {
    let mut header = [0; 3];
    let pos = reader.stream_position()
        .or_describe("failed to get stream position at beginning of parsing")?;
    reader.read_exact(&mut header)
        .or_parsing(pos, "failed to read first header bytes")?;

    let id3_header = *b"ID3";
    // compare header and id3_header
    if header != id3_header {
        return err_parsing(pos, "no id3v2 tag: we need to handle this");
    }

    let pos = reader.seek(SeekFrom::Current(3))
        .or_describe("failed to seek to id3v2 tag size")?;
    let mut sz = [0; 4];
    reader.read_exact(&mut sz)
        .or_parsing(pos, "failed to read id3v2 tag size")?;
    let mut tag_size = u32::from_be_bytes(sz);
    // tag size encoding scheme
    tag_size = tag_size & 0xFF | (tag_size & 0xFF00) >> 1 | (tag_size & 0xFF_0000) >> 2 | (tag_size & 0xFF00_0000) >> 3;

    let pos = reader.seek(SeekFrom::Current(tag_size as i64))
        .or_describe("failed to seek to end of id3v2 tag")?;
    let mut header = [0; 4];
    reader.read_exact(&mut header)
        .or_parsing(pos, "failed to read first header bytes")?;

    Ok(pos)
}

fn copy(r: &mut impl Read, w: &mut impl Write) -> Result<()> {
    let mut buf = [0; 1024];
    loop {
        let n = r.read(&mut buf)
            .or_describe("failed to read from input")?;
        if n == 0 {
            break;
        }
        w.write_all(&buf[..n])
            .or_describe("failed to write to output")?;
    }
    Ok(())
}