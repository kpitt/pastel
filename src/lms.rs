use std::fmt;

use crate::{
    matrix::mat3_dot,
    types::{Mat3, Scalar},
    xyz::Xyz,
    Color,
};

/// A color space whose axes correspond to the responsivity spectra of the long-, medium-, and
/// short-wavelength cone cells in the human eye. More info
/// [here](https://en.wikipedia.org/wiki/LMS_color_space).
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Lms {
    pub l: Scalar,
    pub m: Scalar,
    pub s: Scalar,
    pub alpha: Scalar,
}

impl From<&Color> for Lms {
    fn from(color: &Color) -> Self {
        #[rustfmt::skip]
        const M: Mat3 = [
             0.38971, 0.68898, -0.07868,
            -0.22981, 1.18340,  0.04641,
             0.00000, 0.00000,  1.00000,
        ];

        let Xyz { x, y, z, alpha } = Xyz::from(color);
        let [l, m, s] = mat3_dot(M, [x, y, z]);

        Lms { l, m, s, alpha }
    }
}

impl From<&Lms> for Color {
    fn from(color: &Lms) -> Self {
        #[rustfmt::skip]
        const M: Mat3 = [
            1.91020, -1.112_120, 0.201_908,
            0.37095,  0.629_054, 0.000_000,
            0.00000,  0.000_000, 1.000_000
        ];

        let [x, y, z] = mat3_dot(M, [color.l, color.m, color.s]);
        Self::from(&Xyz {
            x,
            y,
            z,
            alpha: color.alpha,
        })
    }
}

impl fmt::Display for Lms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LMS({l}, {m}, {s})", l = self.l, m = self.m, s = self.s,)
    }
}

impl Lms {
    #[inline]
    pub fn new(l: Scalar, m: Scalar, s: Scalar) -> Self {
        Self::with_alpha(l, m, s, 1.0)
    }

    #[inline]
    pub fn with_alpha(l: Scalar, m: Scalar, s: Scalar, alpha: Scalar) -> Self {
        Lms { l, m, s, alpha }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

    #[test]
    fn lms_conversion() {
        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lms1 = color1.to_lms();
            let color2 = Color::from_lms(lms1.l, lms1.m, lms1.s, 1.0);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }
}
