use std::ops::Range;

/// Size of a single tar block.
pub const BLOCK_SIZE: usize = 512;

/// Byte array representing a single block in a tar file.
pub type TarBlock = [u8; BLOCK_SIZE];

/// Definitions for offsets within the tar header format.
///
/// Numbers are stored in null-terminated octal, unless specified
/// otherwise.
pub mod header {
    use super::*;

    // Below are various valid ranges in the header.

    /// Range for the file name
    pub const NAME_RANGE: Range<usize> = 0..100;
    /// Mode of the file as a number
    pub const MODE_RANGE: Range<usize> = 100..108;
    /// Owner UID of the file
    pub const OWNER_RANGE: Range<usize> = 108..116;
    /// Group ID of the file
    pub const GROUP_RANGE: Range<usize> = 116..124;
    /// Size of the file, as an octal null-terminated string.
    ///
    /// There are some cases in which this field is actually base 256
    /// (i.e. binary) encoded, so be careful.
    pub const SIZE_RANGE: Range<usize> = 124..136;
    /// Modification time in unix timestamp.
    pub const MTIME_RANGE: Range<usize> = 136..148;
    /// Range within a block that contains the checksum
    pub const CHECKSUM_RANGE: Range<usize> = 148..156;
    /// Type of the link. See the LinkType enum for the implementation.
    pub const LINK_TYPE_OFFSET: usize = 156;
    /// Contents of the link if the file is either a symlink or a hard
    /// link. Otherwise empty.
    pub const LINK_NAME_RANGE: Range<usize> = 157..257;
}
