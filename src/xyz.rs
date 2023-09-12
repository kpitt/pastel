use std::fmt;

use crate::{
    convert::{gam_srgb, lin_srgb},
    matrix::mat3_dot,
    types::{Mat3, Scalar},
    Color, RGBA,
};

#[derive(Debug, Clone, PartialEq)]
pub struct XYZ {
    pub x: Scalar,
    pub y: Scalar,
    pub z: Scalar,
    pub alpha: Scalar,
}

impl From<&Color> for XYZ {
    fn from(color: &Color) -> Self {
        #[rustfmt::skip]
        const M: Mat3 = [
            0.4124, 0.3576, 0.1805,
            0.2126, 0.7152, 0.0722,
            0.0193, 0.1192, 0.9505,
        ];

        let rec = RGBA::from(color);
        let r_g_b_ = lin_srgb([rec.r, rec.g, rec.b]);
        let [x, y, z] = mat3_dot(M, r_g_b_);

        XYZ::with_alpha(x, y, z, color.alpha)
    }
}

impl From<&XYZ> for Color {
    fn from(color: &XYZ) -> Self {
        #[rustfmt::skip]
        const M_: Mat3 = [
              3.2406, -1.5372, -0.4986,
             -0.9689,  1.8758,  0.0415,
              0.0557, -0.2040,  1.0570,
        ];

        let r_g_b_ = mat3_dot(M_, [color.x, color.y, color.z]);
        let [r, g, b] = gam_srgb(r_g_b_);
        Self::from(&RGBA::<f64> {
            r,
            g,
            b,
            alpha: color.alpha,
        })
    }
}

impl fmt::Display for XYZ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XYZ({x}, {y}, {z})", x = self.x, y = self.y, z = self.z,)
    }
}

impl XYZ {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self::with_alpha(x, y, z, 1.0)
    }

    #[inline]
    pub fn with_alpha(x: Scalar, y: Scalar, z: Scalar, alpha: Scalar) -> Self {
        XYZ { x, y, z, alpha }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

    #[test]
    fn xyz_conversion() {
        assert_eq!(Color::white(), Color::from_xyz(0.9505, 1.0, 1.0890, 1.0));
        assert_eq!(Color::red(), Color::from_xyz(0.4123, 0.2126, 0.01933, 1.0));
        assert_eq!(
            Color::from_hsl(109.999, 0.08654, 0.407843),
            Color::from_xyz(0.13123, 0.15372, 0.13174, 1.0)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let xyz1 = color1.to_xyz();
            let color2 = Color::from_xyz(xyz1.x, xyz1.y, xyz1.z, 1.0);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }
}
