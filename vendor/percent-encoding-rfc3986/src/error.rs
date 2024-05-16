use core::fmt;

#[derive(Debug, Copy, Clone)]
pub enum PercentDecodeError {
    InvalidHexDigit { position: usize, byte: u8, },
    MissingDigits { single: bool, },
}

impl fmt::Display for PercentDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PercentDecodeError::InvalidHexDigit { position, byte, } if byte.is_ascii() && !byte.is_ascii_control() => {
                write!(f, "The character '{}' at position {} is not a valid hex digit", *byte as char, position)
            },
            PercentDecodeError::InvalidHexDigit { position, byte, } => {
                write!(f, "The byte 0x{:02x} at position {} is not a valid hex digit", byte, position)
            },
            PercentDecodeError::MissingDigits { single: true } => {
                write!(f, "A hex digit is missing at the end of URL")
            },
            PercentDecodeError::MissingDigits { single: false } => {
                write!(f, "Two hex digits are missing at the end of URL")
            },
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl std::error::Error for PercentDecodeError {}
