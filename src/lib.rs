use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::mem::{size_of, transmute};
use std::num::ParseIntError;

use utils::parse_octal;
use utils::parse_size;

use crate::constants::header::{CHECKSUM_RANGE, NAME_RANGE, MODE_RANGE, OWNER_RANGE, GROUP_RANGE, SIZE_RANGE, MTIME_RANGE, LINK_TYPE_OFFSET, LINK_NAME_RANGE};
use crate::constants::TarBlock;
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

impl LinkType {
    pub fn from_byte(b: u8) -> LinkType {
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

#[derive(Debug)]
pub struct TarHeader {
    name: OsString,
    mode: u32,
    owner: u32,
    group: u32,
    size: usize,
    mtime: u64,
    link: LinkType,
    link_name: Option<OsString>,
}

impl TarHeader {
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
            let signed_block = unsafe { transmute::<_, &[i8; size_of::<TarBlock>()]>(block) };
            checksum == compute_checksum(signed_block)
        }
    }

    pub fn from_v7_header(block: &TarBlock) -> Result<TarHeader, TarError> {
        Ok(TarHeader {
            name: trimmed_osstr(&block[NAME_RANGE]).ok_or(TarError::EmptyName)?.to_owned(),
            mode: parse_octal(&block[MODE_RANGE])?,
            owner: parse_octal(&block[OWNER_RANGE])?,
            group: parse_octal(&block[GROUP_RANGE])?,
            size: parse_size(&block[SIZE_RANGE])?,
            mtime: parse_octal(&block[MTIME_RANGE])?,
            link: LinkType::from_byte(block[LINK_TYPE_OFFSET]),
            link_name: trimmed_osstr(&block[LINK_NAME_RANGE]).map(|x| x.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
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
}
