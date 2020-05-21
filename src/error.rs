use std::{error::Error, fmt::Display};

/// The error type of the [`Mpls::from`] method.
///
/// [`Mpls::from`]: ../types/struct.Mpls.html#method.from
#[derive(Debug)]
pub enum MplsError {
    /// An I/O error occurred during parsing.
    IoError(std::io::Error),
    /// Failed to parse the byte stream as valid MPLS.
    ParseError,
}

impl Error for MplsError {}

impl Display for MplsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MplsError::IoError(e) => write!(f, "{}", e),
            MplsError::ParseError => write!(f, "failed to parse byte stream as valid MPLS"),
        }
    }
}

impl From<std::io::Error> for MplsError {
    fn from(err: std::io::Error) -> Self {
        MplsError::IoError(err)
    }
}
