use std::str;
use std::usize;

use itertools::Itertools;
use num::Num;

/// Create an &str from a null-terminated string.
///
/// Any failures will result in None being returned.
pub fn trimmed_str(contents: &[u8]) -> Option<&str> {
    if contents.is_empty() {
        None
    } else {
        match contents.iter().find_position(|&&x| x == 0u8) {
            Some((pos, _)) => str::from_utf8(&contents[..pos]).ok(),
            None => None,
        }
    }
}

pub fn parse_octal<T>(size: &[u8]) -> Result<T, T::FromStrRadixErr>
    where T: Num {
    T::from_str_radix(trimmed_str(size).unwrap(), 8)
}

pub fn parse_size(size: &[u8]) -> usize {
    debug_assert!(size.len() == 12);
    // TODO: implement the extension format.
    parse_octal(size).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trimmed_str() {
        assert_eq!("foo", trimmed_str(&[b'f', b'o', b'o', 0u8, b'b', 0u8]).unwrap());
    }
}
