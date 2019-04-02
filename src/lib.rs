use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::num::ParseIntError;

use utils::parse_octal;
use utils::parse_size;
use crate::utils::trimmed_osstr;

mod utils;

type TarBlock = [u8; 512];

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
    pub fn from_v7_header(block: &TarBlock) -> Result<TarHeader, TarError> {
        Ok(TarHeader {
            name: trimmed_osstr(&block[0..100]).ok_or(TarError::EncodingError)?.to_owned(),
            mode: parse_octal(&block[100..108])?,
            owner: parse_octal(&block[108..116])?,
            group: parse_octal(&block[108..116])?,
            size: parse_size(&block[124..136])?,
            mtime: parse_octal(&block[136..148])?,
            link: LinkType::from_byte(block[156]),
            link_name: trimmed_osstr(&block[157..257]).map(|x| x.to_owned()),
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
}
