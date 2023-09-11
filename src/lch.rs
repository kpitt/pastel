use std::fmt;

use crate::{
    colorspace::ColorSpace,
    helper::{interpolate, interpolate_angle, mod_positive, MaxPrecision},
    lab::Lab,
    types::Scalar,
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LCh {
    pub l: Scalar,
    pub c: Scalar,
    pub h: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for LCh {
    fn from_color(c: &Color) -> Self {
        c.to_lch()
    }

    fn into_color(self) -> Color {
        Color::from_lch(self.l, self.c, self.h, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.c < 0.1 { other.h } else { self.h };
        let other_hue = if other.c < 0.1 { self.h } else { other.h };

        Self {
            l: interpolate(self.l, other.l, fraction),
            c: interpolate(self.c, other.c, fraction),
            h: interpolate_angle(self_hue, other_hue, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for LCh {
    fn from(color: &Color) -> Self {
        let Lab { l, a, b, alpha } = Lab::from(color);

        const RAD2DEG: Scalar = 180.0 / std::f64::consts::PI;

        let c = Scalar::sqrt(a * a + b * b);
        let h = mod_positive(Scalar::atan2(b, a) * RAD2DEG, 360.0);

        LCh::with_alpha(l, c, h, alpha)
    }
}

impl From<&LCh> for Color {
    fn from(color: &LCh) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DEG2RAD: Scalar = std::f64::consts::PI / 180.0;

        let a = color.c * Scalar::cos(color.h * DEG2RAD);
        let b = color.c * Scalar::sin(color.h * DEG2RAD);

        Self::from(&Lab::with_alpha(color.l, a, b, color.alpha))
    }
}

impl fmt::Display for LCh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LCh({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

impl LCh {
    #[inline]
    pub fn new(l: Scalar, c: Scalar, h: Scalar) -> Self {
        Self::with_alpha(l, c, h, 1.0)
    }

    #[inline]
    pub fn with_alpha(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Self {
        LCh { l, c, h, alpha }
    }

    /// Format the color as a Lab-representation string (`Lab(41, 83, -93, 0.5)`). If the alpha channel
    /// is `1.0`, it won't be included in the output.
    pub fn to_color_string(&self, format: Format) -> String {
        let space = if format == Format::Spaces { " " } else { "" };
        format!(
            "LCh({l:.0},{space}{c:.0},{space}{h:.0}{alpha})",
            l = self.l,
            c = self.c,
            h = self.h,
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

    fn assert_almost_equal(c1: &Color, c2: &Color) {
        let c1 = c1.to_rgba();
        let c2 = c2.to_rgba();

        assert!((c1.r as i32 - c2.r as i32).abs() <= 1);
        assert!((c1.g as i32 - c2.g as i32).abs() <= 1);
        assert!((c1.b as i32 - c2.b as i32).abs() <= 1);
    }

    #[test]
    fn lch_conversion() {
        assert_eq!(
            Color::from_hsl(0.0, 1.0, 0.245),
            Color::from_lch(24.829, 60.093, 38.18, 1.0)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lch1 = color1.to_lch();
            let color2 = Color::from_lch(lch1.l, lch1.c, lch1.h, 1.0);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn to_color_string() {
        let c = LCh::new(52.0, 44.0, 271.0);
        assert_eq!("LCh(52, 44, 271)", c.to_color_string(Format::Spaces));
        assert_eq!("LCh(52,44,271)", c.to_color_string(Format::NoSpaces));
    }
}
