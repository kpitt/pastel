//! Various color conversion functions ported from the sample code in the
//! W3C CSS Color 4 draft.

use std::f64::consts::PI;

use crate::{
    matrix::{mat3_dot, vec3_elem_mul},
    types::{Mat3, Scalar, Vec3},
};

// standard white points, defined by 4-figure CIE x,y chromaticities
pub const D50: Vec3 = [0.3457 / 0.3585, 1.00000, (1.0 - 0.3457 - 0.3585) / 0.3585];
pub const D65: Vec3 = [0.3127 / 0.3290, 1.00000, (1.0 - 0.3127 - 0.3290) / 0.3290];

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

// Chromatic adaptation

/// Transforms an array of D65-adapted XYZ color components to the D50 white
/// point using the Bradford chromatic adaption.
/// (http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html)
pub fn d65_to_d50(xyz: Vec3) -> Vec3 {
    #[rustfmt::skip]
    const M: Mat3 = [
          1.0479298208405488,   0.022946793341019088, -0.05019222954313557,
          0.029627815688159344, 0.990434484573249,    -0.01707382502938514,
         -0.009243058152591178, 0.015055144896577895,  0.7518742899580008
    ];

    mat3_dot(M, xyz)
}

/// Transforms an array of D50-adapted XYZ color components to the D65 white
/// point using the Bradford chromatic adaption.
/// (http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html)
pub fn d50_to_d65(xyz: Vec3) -> Vec3 {
    #[rustfmt::skip]
    const M: Mat3 = [
         0.9554734527042182,   -0.023098536874261423, 0.0632593086610217,
        -0.028369706963208136,  1.0099954580058226,   0.021041398966943008,
         0.012314001688319899, -0.020507696433477912, 1.3303659366080753
    ];

    mat3_dot(M, xyz)
}

// CIE Lab and LCH

// from CIE standard, which now defines these as a rational fraction
const LAB_EPSILON: Scalar = 216. / 24389.; // 6^3/29^3
const LAB_KAPPA: Scalar = 24389. / 27.; // 29^3/3^3

/// Converts an array of XYZ values to D50-adapted CIE Lab, assuming XYZ is
/// relative to D50.
pub fn xyz_to_lab(xyz: Vec3) -> Vec3 {
    let f = |v: Scalar| {
        if v > LAB_EPSILON {
            v.cbrt()
        } else {
            (LAB_KAPPA * v + 16.) / 116.
        }
    };

    // compute f values from xyz values scaled relative to reference white
    let [x, y, z] = xyz;
    let fx = f(x / D50[0]);
    let fy = f(y / D50[1]);
    let fz = f(z / D50[2]);

    let l = 116. * fy - 16.;
    let a = 500. * (fx - fy);
    let b = 200. * (fy - fz);

    [l, a, b]
}

/// Converts an array of D65-adapted Lab channel values to XYZ relative to D50.
/// (http://www.brucelindbloom.com/index.html?Eqn_Lab_to_XYZ.html)
pub fn lab_to_xyz(lab: Vec3) -> Vec3 {
    // compute f values, starting with the luminance-related term
    let [l, a, b] = lab;
    let fy = (l + 16.) / 116.;
    let fx = (a / 500.) + fy;
    let fz = fy - (b / 200.);

    // compute reference-relative xyz
    let xr = if fx.powi(3) > LAB_EPSILON {
        fx.powi(3)
    } else {
        (116. * fx - 16.) / LAB_KAPPA
    };
    let yr = if l > LAB_KAPPA * LAB_EPSILON {
        Scalar::powi((l + 16.) / 116., 3)
    } else {
        l / LAB_KAPPA
    };
    let zr = if fz.powi(3) > LAB_EPSILON {
        fz.powi(3)
    } else {
        (116. * fz - 16.) / LAB_KAPPA
    };

    // compute XYZ by scaling by reference white
    vec3_elem_mul([xr, yr, zr], D50)
}

/// Converts an array of Cartesian Lab coordinates to polar LCh form.  This is a
/// simple coordinate system conversion that can be used with either CIELAB or
/// Oklab color values.
pub fn lab_to_lch(lab: Vec3) -> Vec3 {
    let [l, a, b] = lab;
    let c = Scalar::sqrt(a.powi(2) + b.powi(2));
    let h = Scalar::atan2(b, a) * 180.0 / PI;

    [l, c, normalize_hue(h)]
}

/// Converts an array of polar LCh coordinates to Cartesian Lab form.  This is a
/// simple coordinate system conversion that can be used with either CIELAB or
/// Oklab color values.
pub fn lch_to_lab(lch: Vec3) -> Vec3 {
    let [l, c, h] = lch;
    let a = c * Scalar::cos(h * PI / 180.0);
    let b = c * Scalar::sin(h * PI / 180.0);

    [l, a, b]
}

/// Ensures that hue, in degrees, is in the range [0..360)
fn normalize_hue(hue: Scalar) -> Scalar {
    hue - 360.0 * (hue / 360.0).floor()
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

#[cfg(test)]
mod tests {
    use super::*;

    // more precise D65 white point, defined by 4-figure CIE x,y chromaticities
    const D65: Vec3 = [0.3127 / 0.3290, 1.00000, (1.0 - 0.3127 - 0.3290) / 0.3290];

    fn round(v: Scalar, places: i32) -> Scalar {
        let scale: Scalar = 10f64.powi(places);
        (v * scale).round() / scale
    }

    macro_rules! assert_rounded_eq {
        ($expected:expr, $given:expr, $places:expr) => {
            let rd = |v| round(v, $places);
            match (&($expected), &($given)) {
                (expected, given) => {
                    let rounded_expected = match (*expected) {
                        [a, b, c] => [rd(a), rd(b), rd(c)],
                    };
                    let rounded_given = match (*given) {
                        [a, b, c] => [rd(a), rd(b), rd(c)],
                    };
                    assert!(
                        rounded_given == rounded_expected,
                        "assert_rounded_eq!({}, {})

    left  = {:?}
    right = {:?}

",
                        stringify!($expected),
                        stringify!($given),
                        rounded_expected,
                        rounded_given
                    );
                }
            }
        };
        ($expected:expr, $given:expr) => {
            assert_rounded_eq!($expected, $given, 6);
        };
    }

    #[test]
    fn test_xyz_to_oklab() {
        // Unit vectors and white point are considered to be exact values.
        // Conversions of exact values should be accurate to at least 6 places.
        assert_rounded_eq!([1.000, 0.000, 0.000], xyz_to_oklab(D65));
        assert_rounded_eq!(
            [0.449_937, 1.235_758, -0.018_982],
            xyz_to_oklab([1.000, 0.000, 0.000])
        );
        assert_rounded_eq!(
            [0.921_816, -0.671_211, 0.263_400],
            xyz_to_oklab([0.000, 1.000, 0.000])
        );
        assert_rounded_eq!(
            [0.152_597, -1.415_088, -0.448_819],
            xyz_to_oklab([0.000, 0.000, 1.000])
        );
    }

    #[test]
    fn test_oklab_to_xyz() {
        assert_rounded_eq!(D65, oklab_to_xyz([1.000, 0.000, 0.000]));
        // Rounding loses some precision, so conversions of 6-place rounded
        // inputs may only be accurate to 5 places.
        assert_rounded_eq!(
            [1.000, 0.000, 0.000],
            oklab_to_xyz([0.449_937, 1.235_758, -0.018_982]),
            5
        );
        assert_rounded_eq!(
            [0.000, 1.000, 0.000],
            oklab_to_xyz([0.921_816, -0.671_211, 0.263_400]),
            5
        );
        assert_rounded_eq!(
            [0.000, 0.000, 1.000],
            oklab_to_xyz([0.152_597, -1.415_088, -0.448_819]),
            5
        );
    }

    #[test]
    fn test_xyz_oklab_xyz_roundtrip() {
        // round trips with no intermediate rounding are accurate to at least 7 places
        let roundtrip = |x, y, z| {
            let xyz = [x, y, z];
            let xyz_as_lab = xyz_to_oklab(xyz);
            let lab_as_xyz = oklab_to_xyz(xyz_as_lab);
            assert_rounded_eq!(xyz, lab_as_xyz, 7);
        };

        roundtrip(0.950, 1.000, 1.089);
        roundtrip(1.000, 0.000, 0.000);
        roundtrip(0.000, 1.000, 0.000);
        roundtrip(0.000, 0.000, 1.000);
    }

    #[test]
    fn test_oklab_xyz_oklab_roundtrip() {
        // round trips with no intermediate rounding are accurate to at least 7 places
        let roundtrip = |l, a, b| {
            let lab = [l, a, b];
            let lab_as_xyz = oklab_to_xyz(lab);
            let xyz_as_lab = xyz_to_oklab(lab_as_xyz);
            assert_rounded_eq!(lab, xyz_as_lab, 7);
        };

        roundtrip(1.000, 0.000, 0.000);
        roundtrip(1.000, 1.000, 0.000);
        roundtrip(1.000, 0.000, 1.000);
        roundtrip(1.000, 0.000, -1.000);
        roundtrip(0.450, 1.236, -0.019);
        roundtrip(0.922, -0.671, 0.263);
        roundtrip(0.153, -1.415, -0.449);
    }
}
