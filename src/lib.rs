//! Rust implementation of the tar file format.
//!
//! This crate contains classes and methods to efficiently read tar files.
use std::{fmt, io};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::{Read, Seek, SeekFrom};
use std::num::ParseIntError;

use utils::parse_octal;
use utils::parse_size;

use crate::constants::{BLOCK_SIZE, TarBlock};
use crate::constants::header::{
    CHECKSUM_RANGE, GROUP_RANGE, LINK_NAME_RANGE, LINK_TYPE_OFFSET, MODE_RANGE, MTIME_RANGE,
    NAME_RANGE, OWNER_RANGE, SIZE_RANGE,
};
use crate::utils::{compute_checksum, trimmed_osstr};

pub mod constants;
mod utils;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum LinkType {
    Normal,
    Hard,
    Symbolic,
    Other(char),
}

impl From<u8> for LinkType {
    fn from(b: u8) -> LinkType {
        use LinkType::*;

        match b {
            0 | b'0' => Normal,
            b'1' => Hard,
            b'2' => Symbolic,
            c => Other(c as char),
        }
    }
}

#[derive(Debug)]
pub enum TarError {
    CheckSum,
    EncodingError,
    EmptyName,
    ParseError(ParseIntError),
    IOError(io::Error),
    FileEnd,
}

impl PartialEq for TarError {
    fn eq(&self, other: &Self) -> bool {
        use TarError::*;

        match (self, other) {
            (CheckSum, CheckSum) => true,
            (EncodingError, EncodingError) => true,
            (EmptyName, EmptyName) => true,
            (ParseError(_), ParseError(_)) => true,
            (IOError(_), IOError(_)) => true,
            (FileEnd, FileEnd) => true,
            _ => false,
        }
    }
}

impl Display for TarError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl Error for TarError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TarError::ParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ParseIntError> for TarError {
    fn from(p: ParseIntError) -> Self {
        TarError::ParseError(p)
    }
}

impl From<io::Error> for TarError {
    fn from(err: io::Error) -> Self {
        TarError::IOError(err)
    }
}

#[derive(Debug)]
pub struct TarHeader<'a> {
    pub name: &'a OsStr,
    pub mode: u32,
    pub owner: u32,
    pub group: u32,
    pub size: usize,
    pub mtime: u64,
    pub link: LinkType,
    pub link_name: Option<&'a OsStr>,
}

impl<'a> TarHeader<'a> {
    /// Check to see if the header checksum is correct.
    ///
    /// This method checks both the signed and unsigned checksum, and accepts either.
    pub fn validate_checksum(block: &TarBlock) -> bool {
        let checksum: i32 = if let Ok(n) = parse_octal(&block[CHECKSUM_RANGE]) {
            n
        } else {
            return false;
        };

        if checksum == compute_checksum(block) {
            true
        } else {
            // Some implementations have the checksum signed, so just try that as well.
            let signed_block = unsafe { &*(block as *const TarBlock as *const [i8; BLOCK_SIZE]) };
            checksum == compute_checksum(signed_block)
        }
    }

    pub fn from_v7_header(block: &TarBlock) -> Result<TarHeader, TarError> {
        Ok(TarHeader {
            name: trimmed_osstr(&block[NAME_RANGE])
                .ok_or(TarError::EmptyName)?,
            mode: parse_octal(&block[MODE_RANGE])?,
            owner: parse_octal(&block[OWNER_RANGE])?,
            group: parse_octal(&block[GROUP_RANGE])?,
            size: parse_size(&block[SIZE_RANGE])?,
            mtime: parse_octal(&block[MTIME_RANGE])?,
            link: block[LINK_TYPE_OFFSET].into(),
            link_name: trimmed_osstr(&block[LINK_NAME_RANGE]),
        })
    }

    pub fn from_block(block: &TarBlock) -> Result<TarHeader, TarError> {
        if !Self::validate_checksum(block) {
            Err(TarError::CheckSum)
        } else {
            Self::from_v7_header(block)
        }
    }

    pub fn block_size(&self) -> usize {
        let size = self.size / BLOCK_SIZE;
        if self.size % BLOCK_SIZE != 0 {
            size + 1
        } else {
            size
        }
    }
}

pub struct TarEntry<'a> {
    header: TarHeader<'a>,
    to_advance: &'a mut usize,
    to_read: usize,
    handle: &'a mut Read,
}

impl<'a> TarEntry<'a> {
    fn new(header: TarHeader<'a>, handle: &'a mut Read, to_advance: &'a mut usize) -> Self {
        Self {
            to_read: header.size,
            header,
            handle,
            to_advance,
        }
    }

    pub fn get_header(&self) -> &TarHeader {
        &self.header
    }
}

impl<'a> Read for TarEntry<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        println!("{} {:?}", self.to_read, buf);
        let max_len = buf.len().min(self.to_read);
        let read = self.handle.read(&mut buf[..max_len])?;
        println!("{}, {}", max_len, read);
        *self.to_advance -= read;
        self.to_read -= read;
        Ok(read)
    }
}

pub struct TarReader<T>
where T: Read + Seek {
    handle: T,
    buf: TarBlock,
    to_advance: usize
}

impl<T> TarReader<T>
where T: Read + Seek {

    pub fn new(handle: T) -> TarReader<T> {
        Self {
            handle,
            buf: [0u8; 512],
            to_advance: 0,
        }
    }

    pub fn next_entry(&mut self) -> Result<TarEntry, TarError> {
        if self.to_advance > 0 {
            self.handle.seek(SeekFrom::Current(self.to_advance as i64))?;
        }

        let read = self.handle.read(&mut self.buf)?;
        if read != BLOCK_SIZE {
            return Err(TarError::FileEnd);
        }

        if self.buf.iter().all(|&x| x == 0) {
            // A block of all null bytes indicates that we are past the end of the tar file.
            return Err(TarError::FileEnd);
        }

        let header = TarHeader::from_block(&self.buf)?;
        self.to_advance = header.block_size() * BLOCK_SIZE;

        Ok(TarEntry::new(header, &mut self.handle, &mut self.to_advance))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    // Sample archive that contains a version of the Cargo.toml
    const SAMPLE_DATA: &[u8] = include_bytes!("samples/test.tar");

    #[test]
    fn test_v7_parse() {
        let mut block = [0u8; 512];
        block.copy_from_slice(&SAMPLE_DATA[..512]);

        let header = TarHeader::from_v7_header(&block).unwrap();
        assert_eq!(LinkType::Normal, header.link);
    }

    #[test]
    fn test_checksum() {
        let mut block = [0u8; 512];
        block.copy_from_slice(&SAMPLE_DATA[..512]);

        assert_eq!(true, TarHeader::validate_checksum(&block));
    }

    #[test]
    fn test_reader() {
        let handle = Cursor::new(SAMPLE_DATA.as_ref());

        let mut reader = TarReader::new(handle);
        {
            let mut entry = reader.next_entry().unwrap();
            let header = entry.get_header();
            assert_eq!("Cargo.toml", header.name.to_str().unwrap());

            let mut buf = String::new();
            entry.read_to_string(&mut buf).unwrap();
            assert_eq!(include_str!("../Cargo.toml"), buf);
        }

        let entry = reader.next_entry();
        let err = entry.err().unwrap();
        assert_eq!(TarError::FileEnd, err);
    }
}
