use std::fmt;

use crate::{
    colorspace::ColorSpace,
    helper::{clamp, interpolate, interpolate_angle, MaxPrecision},
    types::{Hue, Scalar},
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HSVA {
    pub h: Scalar,
    pub s: Scalar,
    pub v: Scalar,
    pub alpha: Scalar,
}

impl From<&HSVA> for Color {
    fn from(color: &HSVA) -> Self {
        let lightness = color.v * (1.0 - color.s / 2.0);
        let saturation = if lightness > 0.0 && lightness < 1.0 {
            (color.v - lightness) / lightness.min(1.0 - lightness)
        } else {
            0.0
        };

        Color {
            hue: Hue::from(color.h),
            saturation: clamp(0.0, 1.0, saturation),
            lightness: clamp(0.0, 1.0, lightness),
            alpha: clamp(0.0, 1.0, color.alpha),
        }
    }
}

impl From<&Color> for HSVA {
    fn from(color: &Color) -> Self {
        let lightness = color.lightness;

        let value = lightness + color.saturation * lightness.min(1.0 - lightness);
        let saturation = if value > 0.0 {
            2.0 * (1.0 - lightness / value)
        } else {
            0.0
        };

        HSVA {
            h: color.hue.value(),
            s: saturation,
            v: value,
            alpha: color.alpha,
        }
    }
}

impl ColorSpace for HSVA {
    fn from_color(c: &Color) -> Self {
        c.to_hsva()
    }

    fn into_color(self) -> Color {
        Color::from_hsva(self.h, self.s, self.v, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.s < 0.0001 { other.h } else { self.h };
        let other_hue = if other.s < 0.0001 { self.h } else { other.h };

        Self {
            h: interpolate_angle(self_hue, other_hue, fraction),
            s: interpolate(self.s, other.s, fraction),
            v: interpolate(self.v, other.v, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl fmt::Display for HSVA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsv({h}, {s}, {v})", h = self.h, s = self.s, v = self.v)
    }
}

impl HSVA {
    #[inline]
    pub fn new(h: Scalar, s: Scalar, v: Scalar) -> Self {
        Self::with_alpha(h, s, v, 1.0)
    }

    #[inline]
    pub fn with_alpha(h: Scalar, s: Scalar, v: Scalar, alpha: Scalar) -> Self {
        HSVA { h, s, v, alpha }
    }

    /// Format the color as a HSV-representation string (`hsva(123, 50.3%, 80.1%, 0.4)`). If the
    /// alpha channel is `1.0`, the simplified `hsv()` format will be used instead.
    pub fn to_color_string(&self, format: Format) -> String {
        let space = if format == Format::Spaces { " " } else { "" };
        let (a_prefix, a) = if self.alpha == 1.0 {
            ("", "".to_string())
        } else {
            (
                "a",
                format!(
                    ",{space}{alpha}",
                    alpha = MaxPrecision::wrap(3, self.alpha),
                    space = space
                ),
            )
        };
        format!(
            "hsv{a_prefix}({h:.0},{space}{s:.1}%,{space}{v:.1}%{a})",
            space = space,
            a_prefix = a_prefix,
            h = self.h,
            s = 100.0 * self.s,
            v = 100.0 * self.v,
            a = a,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

    #[test]
    fn hsva_conversion() {
        assert_eq!(
            Color::from_hsla(0.0, 1.0, 0.5, 0.5),
            Color::from_hsva(0.0, 1.0, 1.0, 0.5)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let hsva1 = color1.to_hsva();
            let color2 = Color::from_hsva(hsva1.h, hsva1.s, hsva1.v, hsva1.alpha);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }
}
