//! Various color conversion functions ported from the sample code in the
//! W3C CSS Color 4 draft.

use std::f64::consts::PI;

use crate::{
    matrix::mat3_dot,
    types::{Mat3, Scalar, Vec3},
};

// sRGB-related functions

/// Converts an array of sRGB values where in-gamut values are in the range
/// [0 - 1] to linear light (un-companded) form.
/// (https://en.wikipedia.org/wiki/SRGB)
///
/// Extended transfer function:
/// For negative values, linear portion is extended on reflection of axis,
/// then reflected power function is used.
pub fn lin_srgb(rgb: Vec3) -> Vec3 {
    let finv = |val: Scalar| {
        let abs = val.abs();

        if abs < 0.04045 {
            val / 12.92
        } else {
            val.signum() * Scalar::powf((abs + 0.055) / 1.055, 2.4)
        }
    };

    let [r, g, b] = rgb;
    [finv(r), finv(g), finv(b)]
}

/// Converts an array of linear-light sRGB values in the range 0.0-1.0
/// to gamma corrected form.
/// (https://en.wikipedia.org/wiki/SRGB)
///
/// Extended transfer function:
/// For negative values, linear portion is extended on reflection of axis,
/// then reflected power function is used.
pub fn gam_srgb(rgb: Vec3) -> Vec3 {
    let f = |val: Scalar| {
        let abs = val.abs();

        if abs > 0.0031308 {
            val.signum() * (1.055 * Scalar::powf(abs, 1.0 / 2.4) - 0.055)
        } else {
            12.92 * val
        }
    };

    let [r, g, b] = rgb;
    [f(r), f(g), f(b)]
}

// OKLab and OKLCH
// https://bottosson.github.io/posts/oklab/

/// Converts an array of D65-adapted XYZ values to Oklab.
#[allow(clippy::excessive_precision)]
pub fn xyz_to_oklab(xyz: Vec3) -> Vec3 {
    #[rustfmt::skip]
    const M1: Mat3 = [
        0.8190224432164319,   0.3619062562801221,  -0.12887378261216414,
        0.0329836671980271,   0.9292868468965546,   0.03614466816999844,
        0.048177199566046255, 0.26423952494422764,  0.6335478258136937
    ];
    #[rustfmt::skip]
    const M2: Mat3 = [
        0.2104542553,  0.7936177850, -0.0040720468,
        1.9779984951, -2.4285922050,  0.4505937099,
        0.0259040371,  0.7827717662, -0.8086757660
    ];

    let [l, m, s] = mat3_dot(M1, xyz);
    mat3_dot(M2, [l.cbrt(), m.cbrt(), s.cbrt()])
}

/// Converts an array of Oklab channel values to D65-adapted XYZ.
#[allow(clippy::excessive_precision)]
pub fn oklab_to_xyz(lab: Vec3) -> Vec3 {
    #[rustfmt::skip]
    const M2_: Mat3 = [
        0.99999999845051981432,  0.39633779217376785678,   0.21580375806075880339,
        1.0000000088817607767,  -0.1055613423236563494,   -0.063854174771705903402,
        1.0000000546724109177,  -0.089484182094965759684, -1.2914855378640917399
    ];
    #[rustfmt::skip]
    const M1_: Mat3 = [
         1.2268798733741557,  -0.5578149965554813,  0.28139105017721583,
        -0.04057576262431372,  1.1122868293970594, -0.07171106666151701,
        -0.07637294974672142, -0.4214933239627914,  1.5869240244272418
    ];

    let [l_, m_, s_] = mat3_dot(M2_, lab);
    mat3_dot(M1_, [l_.powi(3), m_.powi(3), s_.powi(3)])
}

/// Ensures that hue, in degrees, is in the range [0..360)
fn normalize_hue(hue: Scalar) -> Scalar {
    hue - 360.0 * (hue / 360.0).floor()
}

/// Converts an array of Cartesian Oklab coordinates to an array of polar Oklch coordinates.
pub fn oklab_to_oklch(oklab: Vec3) -> Vec3 {
    let [ok_l, ok_a, ok_b] = oklab;
    let ok_c = Scalar::sqrt(ok_a.powi(2) + ok_b.powi(2));
    let ok_h = Scalar::atan2(ok_b, ok_a) * 180.0 / PI;

    [ok_l, ok_c, normalize_hue(ok_h)]
}

/// Converts an array of polar Oklch coordinates to an array of Cartesian Oklab coordinates.
pub fn oklch_to_oklab(oklch: Vec3) -> Vec3 {
    let [ok_l, ok_c, ok_h] = oklch;
    let ok_a = ok_c * Scalar::cos(ok_h * PI / 180.0);
    let ok_b = ok_c * Scalar::sin(ok_h * PI / 180.0);

    [ok_l, ok_a, ok_b]
}
