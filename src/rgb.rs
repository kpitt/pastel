use std::fmt;

use crate::{
    colorspace::ColorSpace,
    helper::{clamp, interpolate, mod_positive, MaxPrecision},
    hsl::HSLA,
    types::Scalar,
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RGBA<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub alpha: Scalar,
}

impl ColorSpace for RGBA<f64> {
    fn from_color(c: &Color) -> Self {
        c.to_rgba_float()
    }

    fn into_color(self) -> Color {
        Color::from_rgba_float(self.r, self.g, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            r: interpolate(self.r, other.r, fraction),
            g: interpolate(self.g, other.g, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for RGBA<f64> {
    fn from(color: &Color) -> Self {
        let h_s = color.hue.value() / 60.0;
        let chr = (1.0 - Scalar::abs(2.0 * color.lightness - 1.0)) * color.saturation;
        let m = color.lightness - chr / 2.0;
        let x = chr * (1.0 - Scalar::abs(h_s % 2.0 - 1.0));

        #[allow(clippy::upper_case_acronyms)]
        struct RGB(Scalar, Scalar, Scalar);

        let col = if h_s < 1.0 {
            RGB(chr, x, 0.0)
        } else if (1.0..2.0).contains(&h_s) {
            RGB(x, chr, 0.0)
        } else if (2.0..3.0).contains(&h_s) {
            RGB(0.0, chr, x)
        } else if (3.0..4.0).contains(&h_s) {
            RGB(0.0, x, chr)
        } else if (4.0..5.0).contains(&h_s) {
            RGB(x, 0.0, chr)
        } else {
            RGB(chr, 0.0, x)
        };

        RGBA {
            r: col.0 + m,
            g: col.1 + m,
            b: col.2 + m,
            alpha: color.alpha,
        }
    }
}

impl From<&Color> for RGBA<u8> {
    fn from(color: &Color) -> Self {
        let c = RGBA::<f64>::from(color);
        // Tiny rounding errors in `f64` floating point calculations can cause effectively equal
        // values to round to different integers.  We expect `f64` rounding errors to be less than
        // the precision of an `f32` in most cases, so we can eliminate many of these rounding
        // anomalies by first converting the values to `f32` before rounding.
        let r = f32::round((255.0 * c.r) as f32) as u8;
        let g = f32::round((255.0 * c.g) as f32) as u8;
        let b = f32::round((255.0 * c.b) as f32) as u8;

        RGBA {
            r,
            g,
            b,
            alpha: color.alpha,
        }
    }
}

impl From<&RGBA<u8>> for Color {
    fn from(color: &RGBA<u8>) -> Self {
        let max_chroma = u8::max(u8::max(color.r, color.g), color.b);
        let min_chroma = u8::min(u8::min(color.r, color.g), color.b);

        let chroma = max_chroma - min_chroma;
        let chroma_s = Scalar::from(chroma) / 255.0;

        let r_s = Scalar::from(color.r) / 255.0;
        let g_s = Scalar::from(color.g) / 255.0;
        let b_s = Scalar::from(color.b) / 255.0;

        let hue = 60.0
            * (if chroma == 0 {
                0.0
            } else if color.r == max_chroma {
                mod_positive((g_s - b_s) / chroma_s, 6.0)
            } else if color.g == max_chroma {
                (b_s - r_s) / chroma_s + 2.0
            } else {
                (r_s - g_s) / chroma_s + 4.0
            });

        let lightness = (Scalar::from(max_chroma) + Scalar::from(min_chroma)) / (255.0 * 2.0);
        let saturation = if chroma == 0 {
            0.0
        } else {
            chroma_s / (1.0 - Scalar::abs(2.0 * lightness - 1.0))
        };
        Self::from(&HSLA::with_alpha(hue, saturation, lightness, color.alpha))
    }
}

impl From<&RGBA<f64>> for Color {
    fn from(color: &RGBA<f64>) -> Self {
        let r = Scalar::round(clamp(0.0, 255.0, 255.0 * color.r)) as u8;
        let g = Scalar::round(clamp(0.0, 255.0, 255.0 * color.g)) as u8;
        let b = Scalar::round(clamp(0.0, 255.0, 255.0 * color.b)) as u8;
        Self::from(&RGBA::with_alpha(r, g, b, color.alpha))
    }
}

impl fmt::Display for RGBA<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl fmt::Display for RGBA<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl<T> RGBA<T> {
    #[inline]
    pub fn new(r: T, g: T, b: T) -> Self {
        Self::with_alpha(r, g, b, 1.0)
    }

    #[inline]
    pub fn with_alpha(r: T, g: T, b: T, alpha: Scalar) -> Self {
        RGBA { r, g, b, alpha }
    }
}

impl RGBA<u8> {
    /// Format the color as a RGB-representation string (`rgba(255, 127, 0, 0.5)`). If the alpha
    /// channel is `1.0`, the simplified `rgb()` format will be used instead.
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
            "rgb{a_prefix}({r},{space}{g},{space}{b}{a})",
            space = space,
            a_prefix = a_prefix,
            r = self.r,
            g = self.g,
            b = self.b,
            a = a,
        )
    }

    /// Format the color as a RGB-representation string (`#fc0070`). The output will contain 6 hex
    /// digits if the alpha channel is `1.0`, or 8 hex digits otherwise.
    pub fn to_hex_string(&self, leading_hash: bool) -> String {
        format!(
            "{}{:02x}{:02x}{:02x}{}",
            if leading_hash { "#" } else { "" },
            self.r,
            self.g,
            self.b,
            if self.alpha == 1.0 {
                "".to_string()
            } else {
                format!("{:02x}", (self.alpha * 255.).round() as u8)
            }
        )
    }
}

impl RGBA<f64> {
    /// Format the color as a floating point RGB-representation string (`rgb(1.0, 0.5, 0)`). If the
    /// alpha channel is `1.0`, the simplified `rgb()` format will be used instead.
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
            "rgb{a_prefix}({r:.3},{space}{g:.3},{space}{b:.3}{a})",
            space = space,
            a_prefix = a_prefix,
            r = self.r,
            g = self.g,
            b = self.b,
            a = a,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_hsl_conversion() {
        assert_eq!(Color::white(), Color::from_rgb_float(1.0, 1.0, 1.0));
        assert_eq!(Color::gray(), Color::from_rgb_float(0.5, 0.5, 0.5));
        assert_eq!(Color::black(), Color::from_rgb_float(0.0, 0.0, 0.0));
        assert_eq!(Color::red(), Color::from_rgb_float(1.0, 0.0, 0.0));
        assert_eq!(
            Color::from_hsl(60.0, 1.0, 0.375),
            Color::from_rgb_float(0.75, 0.75, 0.0)
        ); //yellow-green
        assert_eq!(Color::green(), Color::from_rgb_float(0.0, 0.5, 0.0));
        assert_eq!(
            Color::from_hsl(240.0, 1.0, 0.75),
            Color::from_rgb_float(0.5, 0.5, 1.0)
        ); // blue-ish
        assert_eq!(
            Color::from_hsl(49.5, 0.893, 0.497),
            Color::from_rgb_float(0.941, 0.785, 0.053)
        ); // yellow
        assert_eq!(
            Color::from_hsl(162.4, 0.779, 0.447),
            Color::from_rgb_float(0.099, 0.795, 0.591)
        ); // cyan 2
    }

    #[test]
    fn rgb_u8_roundtrip_conversion() {
        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let rgb = color1.to_rgba();
            let color2 = Color::from_rgb(rgb.r, rgb.g, rgb.b);
            assert_eq!(color1, color2);
        };

        roundtrip(0.0, 0.0, 1.0);
        roundtrip(0.0, 0.0, 0.5);
        roundtrip(0.0, 0.0, 0.0);
        roundtrip(60.0, 1.0, 0.375);
        roundtrip(120.0, 1.0, 0.25);
        roundtrip(240.0, 1.0, 0.75);
        roundtrip(49.5, 0.893, 0.497);
        roundtrip(162.4, 0.779, 0.447);

        for degree in 0..360 {
            roundtrip(Scalar::from(degree), 0.5, 0.8);
        }
    }

    #[test]
    fn to_color_string_u8() {
        let c = RGBA::new(255, 127, 4);
        assert_eq!("rgb(255, 127, 4)", c.to_color_string(Format::Spaces));
        assert_eq!("rgb(255,127,4)", c.to_color_string(Format::NoSpaces));
    }

    #[test]
    fn to_color_string_f64() {
        let cb = Color::black().to_rgba_float();
        assert_eq!(
            "rgb(0.000, 0.000, 0.000)",
            cb.to_color_string(Format::Spaces)
        );
        assert_eq!(
            "rgb(0.000,0.000,0.000)",
            cb.to_color_string(Format::NoSpaces)
        );

        let cw = Color::white().to_rgba_float();
        assert_eq!(
            "rgb(1.000, 1.000, 1.000)",
            cw.to_color_string(Format::Spaces)
        );
        assert_eq!(
            "rgb(1.000,1.000,1.000)",
            cw.to_color_string(Format::NoSpaces)
        );

        let c = RGBA::new(0.12, 0.45, 0.78);
        assert_eq!(
            "rgb(0.120, 0.450, 0.780)",
            c.to_color_string(Format::Spaces)
        );
    }

    #[test]
    fn to_rgb_hex_string() {
        let c = RGBA::new(255, 127, 4);
        assert_eq!("ff7f04", c.to_hex_string(false));
        assert_eq!("#ff7f04", c.to_hex_string(true));
    }
}
