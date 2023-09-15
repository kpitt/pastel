use std::fmt;

use crate::{
    colorspace::ColorSpace,
    helper::{clamp, interpolate, interpolate_angle, MaxPrecision},
    types::{Hue, Scalar},
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HSLA {
    pub h: Scalar,
    pub s: Scalar,
    pub l: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for HSLA {
    fn from_color(c: &Color) -> Self {
        c.to_hsla()
    }

    fn into_color(self) -> Color {
        Color::from_hsla(self.h, self.s, self.l, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.s < 0.0001 { other.h } else { self.h };
        let other_hue = if other.s < 0.0001 { self.h } else { other.h };

        Self {
            h: interpolate_angle(self_hue, other_hue, fraction),
            s: interpolate(self.s, other.s, fraction),
            l: interpolate(self.l, other.l, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for HSLA {
    fn from(color: &Color) -> Self {
        HSLA {
            h: color.hue.value(),
            s: color.saturation,
            l: color.lightness,
            alpha: color.alpha,
        }
    }
}

impl From<&HSLA> for Color {
    fn from(color: &HSLA) -> Self {
        Color {
            hue: Hue::from(color.h),
            saturation: clamp(0.0, 1.0, color.s),
            lightness: clamp(0.0, 1.0, color.l),
            alpha: clamp(0.0, 1.0, color.alpha),
        }
    }
}

impl fmt::Display for HSLA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsl({h}, {s}, {l})", h = self.h, s = self.s, l = self.l,)
    }
}

impl HSLA {
    #[inline]
    pub fn new(h: Scalar, s: Scalar, l: Scalar) -> Self {
        Self::with_alpha(h, s, l, 1.0)
    }

    #[inline]
    pub fn with_alpha(h: Scalar, s: Scalar, l: Scalar, alpha: Scalar) -> Self {
        HSLA { h, s, l, alpha }
    }

    /// Format the color as a HSL-representation string (`hsla(123, 50.3%, 80.1%, 0.4)`). If the
    /// alpha channel is `1.0`, the simplified `hsl()` format will be used instead.
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
            "hsl{a_prefix}({h:.0},{space}{s:.1}%,{space}{l:.1}%{a})",
            space = space,
            a_prefix = a_prefix,
            h = self.h,
            s = 100.0 * self.s,
            l = 100.0 * self.l,
            a = a,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_color_string() {
        let c = HSLA {
            h: 91.3,
            s: 0.541,
            l: 0.983,
            alpha: 1.0,
        };
        assert_eq!("hsl(91, 54.1%, 98.3%)", c.to_color_string(Format::Spaces));
        assert_eq!("hsl(91,54.1%,98.3%)", c.to_color_string(Format::NoSpaces));
    }

    #[test]
    fn mixing_with_gray_preserves_hue() {
        let hue = 123.0;
        let base = HSLA::new(hue, 0.5, 0.5);

        let hue_after_mixing = |other| base.mix(&HSLA::from(&other), Fraction::from(0.5)).h;

        assert_eq!(hue, hue_after_mixing(Color::black()));
        assert_eq!(hue, hue_after_mixing(Color::graytone(0.2)));
        assert_eq!(hue, hue_after_mixing(Color::graytone(0.7)));
        assert_eq!(hue, hue_after_mixing(Color::white()));
    }
}
