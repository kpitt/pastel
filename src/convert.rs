// Various color conversion functions ported from the sample code in the W3C CSS Color 4.

use crate::types::Scalar;

type ChannelTuple = (Scalar, Scalar, Scalar);

// sRGB-related functions

/// Converts a tuple of sRGB values where in-gamut values are in the range [0 - 1]
/// to linear light (un-companded) form.
/// (https://en.wikipedia.org/wiki/SRGB)
///
/// Extended transfer function:
/// For negative values, linear portion is extended on reflection of axis,
/// then reflected power function is used.
pub fn lin_srgb(rgb: ChannelTuple) -> ChannelTuple {
    let finv = |val: Scalar| {
        let abs = val.abs();

        if abs < 0.04045 {
            val / 12.92
        } else {
            val.signum() * Scalar::powf((abs + 0.055) / 1.055, 2.4)
        }
    };

    let (r, g, b) = rgb;
    (finv(r), finv(g), finv(b))
}

/// Converts a tuple of linear-light sRGB values in the range 0.0-1.0
/// to gamma corrected form.
/// (https://en.wikipedia.org/wiki/SRGB)
///
/// Extended transfer function:
/// For negative values, linear portion is extended on reflection of axis,
/// then reflected power function is used.
pub fn gam_srgb(rgb: ChannelTuple) -> ChannelTuple {
    let f = |val: Scalar| {
        let abs = val.abs();

        if abs > 0.0031308 {
            val.signum() * (1.055 * Scalar::powf(abs, 1.0 / 2.4) - 0.055)
        } else {
            12.92 * val
        }
    };

    let (r, g, b) = rgb;
    (f(r), f(g), f(b))
}
