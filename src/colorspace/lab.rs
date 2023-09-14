use std::fmt;

use super::{xyz::XYZ, ColorSpace};
use crate::{
    helper::{interpolate, MaxPrecision},
    types::Scalar,
    Color, Format, Fraction, D65_XN, D65_YN, D65_ZN,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Lab {
    pub l: Scalar,
    pub a: Scalar,
    pub b: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for Lab {
    fn from_color(c: &Color) -> Self {
        c.to_lab()
    }

    fn into_color(self) -> Color {
        Color::from_lab(self.l, self.a, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            l: interpolate(self.l, other.l, fraction),
            a: interpolate(self.a, other.a, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for Lab {
    fn from(color: &Color) -> Self {
        let rec = XYZ::from(color);

        let cut = Scalar::powf(6.0 / 29.0, 3.0);
        let f = |t| {
            if t > cut {
                Scalar::powf(t, 1.0 / 3.0)
            } else {
                (1.0 / 3.0) * Scalar::powf(29.0 / 6.0, 2.0) * t + 4.0 / 29.0
            }
        };

        let fy = f(rec.y / D65_YN);

        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (f(rec.x / D65_XN) - fy);
        let b = 200.0 * (fy - f(rec.z / D65_ZN));

        Lab::with_alpha(l, a, b, color.alpha)
    }
}

impl From<&Lab> for Color {
    fn from(color: &Lab) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DELTA: Scalar = 6.0 / 29.0;

        let finv = |t| {
            if t > DELTA {
                Scalar::powf(t, 3.0)
            } else {
                3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
            }
        };

        let l_ = (color.l + 16.0) / 116.0;
        let x = D65_XN * finv(l_ + color.a / 500.0);
        let y = D65_YN * finv(l_);
        let z = D65_ZN * finv(l_ - color.b / 200.0);

        Self::from(&XYZ::with_alpha(x, y, z, color.alpha))
    }
}

impl fmt::Display for Lab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lab({l}, {a}, {b})", l = self.l, a = self.a, b = self.b,)
    }
}

impl Lab {
    #[inline]
    pub fn new(l: Scalar, a: Scalar, b: Scalar) -> Self {
        Self::with_alpha(l, a, b, 1.0)
    }

    #[inline]
    pub fn with_alpha(l: Scalar, a: Scalar, b: Scalar, alpha: Scalar) -> Self {
        Lab { l, a, b, alpha }
    }

    /// Format the color as a Lab-representation string (`Lab(41, 83, -93, 0.5)`). If the alpha channel
    /// is `1.0`, it won't be included in the output.
    pub fn to_color_string(&self, format: Format) -> String {
        let space = if format == Format::Spaces { " " } else { "" };
        format!(
            "Lab({l:.0},{space}{a:.0},{space}{b:.0}{alpha})",
            l = self.l,
            a = self.a,
            b = self.b,
            space = space,
            alpha = if self.alpha == 1.0 {
                "".to_string()
            } else {
                format!(
                    ",{space}{alpha}",
                    alpha = MaxPrecision::wrap(3, self.alpha),
                    space = space
                )
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

    #[test]
    fn lab_conversion() {
        assert_eq!(Color::red(), Color::from_lab(53.233, 80.109, 67.22, 1.0));

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lab1 = color1.to_lab();
            let color2 = Color::from_lab(lab1.l, lab1.a, lab1.b, 1.0);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn to_color_string() {
        let c = Lab::new(41.0, 83.0, -93.0);
        assert_eq!("Lab(41, 83, -93)", c.to_color_string(Format::Spaces));
        assert_eq!("Lab(41,83,-93)", c.to_color_string(Format::NoSpaces));
    }
}
