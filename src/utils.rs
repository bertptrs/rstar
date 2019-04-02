use std::ffi::OsStr;
use std::num::ParseIntError;
use std::os::unix::ffi::OsStrExt;
use std::str;
use std::usize;

use itertools::Itertools;
use num::Num;

use crate::constants::header::CHECKSUM_RANGE;

/// Create an &str from a null-terminated string.
///
/// Any failures will result in None being returned.
pub fn trimmed_str(contents: &[u8]) -> Option<&str> {
    if contents.is_empty() {
        None
    } else {
        match contents.iter().find_position(|&&x| x == 0u8) {
            Some((0, _)) => None,
            Some((pos, _)) => str::from_utf8(&contents[..pos]).ok(),
            None => None,
        }
    }
}

pub fn trimmed_osstr(contents: &[u8]) -> Option<&OsStr> {
    if contents.is_empty() {
        None
    } else {
        match contents.iter().find_position(|&&x| x == 0u8) {
            Some((pos, _)) if pos != 0 => Some(OsStr::from_bytes(&contents[..pos])),
            _ => None
        }
    }
}

pub fn parse_octal<T>(size: &[u8]) -> Result<T, T::FromStrRadixErr>
    where T: Num {
    T::from_str_radix(trimmed_str(size).unwrap_or(""), 8)
}

pub fn parse_size(size: &[u8]) -> Result<usize, ParseIntError> {
    debug_assert!(size.len() == 12);
    // TODO: implement the extension format.
    parse_octal(size)
}

/// Compute the checksum for a given block.
///
/// The checksum is simply the sum of all the bytes in the header, where the
/// checksum bytes are masked with space characters (ASCII 32).
///
/// While most programs agree that this should be unsigned, it isn't always,
/// so this function is generic in order to do both.
pub fn compute_checksum<T>(block: &[T; 512]) -> i32
    where T: Into<i32> + Copy {
    let checksum_bytes = &block[CHECKSUM_RANGE];
    let checksum_sum: i32 = checksum_bytes.iter().cloned().map_into().sum::<i32>();

    block.iter().cloned().map_into().sum::<i32>()
        // Remove the actual checksum of the checksum bytes
        - checksum_sum
        // And add the 8 spaces, ASCII 32
        + 32 * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trimmed_str() {
        assert_eq!("foo", trimmed_str(b"foo\0bar\0").unwrap());
    }
}
