/// Errors possible during decoding.
use std::fmt::{Debug, Formatter};

use zune_core::colorspace::ColorSpace;

/// Possible Errors that may occur during decoding
pub enum QoiErrors
{
    /// The image does not start with QOI magic bytes `qoif`
    ///
    /// Indicates that image is not a qoi file
    WrongMagicBytes,
    /// The input buffer doesn't have enough bytes to fully
    /// reconstruct the image
    ///
    /// # Arguments
    /// - 1st argument is the number of bytes we expected
    /// - 2nd argument is number of bytes actually left
    InsufficientData(usize, usize),
    /// The header contains an invalid channel number
    ///
    /// The only supported types are `3` and `4`
    UnknownChannels(u8),
    /// The header contains an invalid colorspace value
    ///
    /// The should be `0` or `1`
    /// but this can be ignored if strict is set to false
    UnknownColorspace(u8),
    /// Generic message
    Generic(String),
    /// Generic message does not need heap allocation
    GenericStatic(&'static str)
}

impl Debug for QoiErrors
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            QoiErrors::WrongMagicBytes =>
            {
                writeln!(f, "Wrong magic bytes, expected `qoif` as image start")
            }
            QoiErrors::InsufficientData(expected, found) =>
            {
                writeln!(
                    f,
                    "Insufficient data required {expected} but remaining stream has {found}"
                )
            }
            QoiErrors::UnknownChannels(channel) =>
            {
                writeln!(
                    f,
                    "Unknown channel number {channel}, expected either 3 or 4"
                )
            }
            QoiErrors::UnknownColorspace(colorspace) =>
            {
                writeln!(
                    f,
                    "Unknown colorspace number {colorspace}, expected either 0 or 1"
                )
            }
            QoiErrors::Generic(val) =>
            {
                writeln!(f, "{val}")
            }
            QoiErrors::GenericStatic(val) =>
            {
                writeln!(f, "{val}")
            }
        }
    }
}

impl From<&'static str> for QoiErrors
{
    fn from(r: &'static str) -> Self
    {
        Self::GenericStatic(r)
    }
}

pub enum QoiEncodeErrors
{
    /// Unsupported colorspace
    ///
    /// The first argument is the colorspace encountered
    /// The second argument is list of supported colorspaces
    UnsupportedColorspace(ColorSpace, &'static [ColorSpace]),

    /// Too large dimensions
    /// The dimensions cannot be correctly encoded to a width
    TooLargeDimensions(usize)
}
