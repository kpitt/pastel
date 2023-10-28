use std::fmt;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, hex_digit1, space0, space1},
    combinator::{all_consuming, cond, opt},
    error::ErrorKind,
    number::complete::double,
    Err, IResult,
};

use crate::{
    colorspace::ColorSpace,
    convert::gam_srgb,
    helper::{interpolate, mod_positive, MaxPrecision},
    hsl::Hsla,
    parser::{
        css_color_function, legacy_alpha, legacy_separator, modern_alpha, number_or_percentage,
        percentage,
    },
    types::Scalar,
    Color, Format, Fraction,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub alpha: Scalar,
}

impl ColorSpace for Srgba<f64> {
    fn mix(self, other: Self, fraction: Fraction) -> Self {
        Self {
            r: interpolate(self.r, other.r, fraction),
            g: interpolate(self.g, other.g, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<Color> for Srgba<f64> {
    fn from(color: Color) -> Self {
        let h_s = color.hue.value() / 60.0;
        let chr = (1.0 - Scalar::abs(2.0 * color.lightness - 1.0)) * color.saturation;
        let m = color.lightness - chr / 2.0;
        let x = chr * (1.0 - Scalar::abs(h_s % 2.0 - 1.0));

        struct Rgb(Scalar, Scalar, Scalar);

        let col = if h_s < 1.0 {
            Rgb(chr, x, 0.0)
        } else if (1.0..2.0).contains(&h_s) {
            Rgb(x, chr, 0.0)
        } else if (2.0..3.0).contains(&h_s) {
            Rgb(0.0, chr, x)
        } else if (3.0..4.0).contains(&h_s) {
            Rgb(0.0, x, chr)
        } else if (4.0..5.0).contains(&h_s) {
            Rgb(x, 0.0, chr)
        } else {
            Rgb(chr, 0.0, x)
        };

        Srgba {
            r: col.0 + m,
            g: col.1 + m,
            b: col.2 + m,
            alpha: color.alpha,
        }
    }
}

impl From<Color> for Srgba<u8> {
    fn from(color: Color) -> Self {
        let c = Srgba::<f64>::from(color);
        // Tiny rounding errors in `f64` floating point calculations can cause effectively equal
        // values to round to different integers.  We expect `f64` rounding errors to be less than
        // the precision of an `f32` in most cases, so we can eliminate many of these rounding
        // anomalies by first converting the values to `f32` before rounding.
        let r = f32::round((255.0 * c.r) as f32) as u8;
        let g = f32::round((255.0 * c.g) as f32) as u8;
        let b = f32::round((255.0 * c.b) as f32) as u8;

        Srgba {
            r,
            g,
            b,
            alpha: color.alpha,
        }
    }
}

impl From<Srgba<u8>> for Color {
    fn from(color: Srgba<u8>) -> Self {
        let Srgba { r, g, b, alpha } = color;
        Self::from(Srgba::with_alpha(
            (r as f64) / 255.0,
            (g as f64) / 255.0,
            (b as f64) / 255.0,
            alpha,
        ))
    }
}

impl From<Srgba<f64>> for Color {
    fn from(color: Srgba<f64>) -> Self {
        const EPS: f64 = f64::EPSILON * 2.0;

        let Srgba { r, g, b, alpha } = color;
        let max_chroma = f64::max(f64::max(r, g), b);
        let min_chroma = f64::min(f64::min(r, g), b);

        let chroma = max_chroma - min_chroma;

        let hue = 60.0
            * (if chroma.abs() < EPS {
                0.0
            } else if r == max_chroma {
                mod_positive((g - b) / chroma, 6.0)
            } else if g == max_chroma {
                (b - r) / chroma + 2.0
            } else {
                (r - g) / chroma + 4.0
            });

        let lightness = (max_chroma + min_chroma) / 2.0;
        let saturation = if chroma.abs() < EPS {
            0.0
        } else {
            chroma / (1.0 - Scalar::abs(2.0 * lightness - 1.0))
        };
        Self::from(Hsla::with_alpha(hue, saturation, lightness, alpha))
    }
}

impl fmt::Display for Srgba<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl fmt::Display for Srgba<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl<T> Srgba<T> {
    #[inline]
    pub fn new(r: T, g: T, b: T) -> Self {
        Self::with_alpha(r, g, b, 1.0)
    }

    #[inline]
    pub fn with_alpha(r: T, g: T, b: T, alpha: Scalar) -> Self {
        Srgba { r, g, b, alpha }
    }
}

impl Srgba<u8> {
    /// Return the color as an integer in RGB representation (`0xRRGGBB`)
    #[inline]
    pub fn to_u32(&self) -> u32 {
        u32::from(self.r).wrapping_shl(16) + u32::from(self.g).wrapping_shl(8) + u32::from(self.b)
    }

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

impl Srgba<f64> {
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

pub(crate) fn parse_rgb_color(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_rgb_hex),
        all_consuming(parse_css_numeric_rgb),
        all_consuming(parse_css_percentage_rgb),
        all_consuming(parse_numeric_rgb),
        all_consuming(parse_percentage_rgb),
        all_consuming(parse_srgb_color_space),
        all_consuming(parse_srgb_linear_color_space),
    ))(input.trim())
}

fn parse_rgb_hex(input: &str) -> IResult<&str, Color> {
    fn hex_to_u8_unsafe(num: &str) -> u8 {
        u8::from_str_radix(num, 16).unwrap()
    }

    let (input, _) = opt(char('#'))(input)?;
    let (input, hex_chars) = hex_digit1(input)?;
    match hex_chars.len() {
        // RRGGBB
        6 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            Ok((input, Color::from_rgb(r, g, b)))
        }
        // RGB
        3 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..1]);
            let g = hex_to_u8_unsafe(&hex_chars[1..2]);
            let b = hex_to_u8_unsafe(&hex_chars[2..3]);
            let r = r * 16 + r;
            let g = g * 16 + g;
            let b = b * 16 + b;
            Ok((input, Color::from_rgb(r, g, b)))
        }
        // RRGGBBAA
        8 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            let a = hex_to_u8_unsafe(&hex_chars[6..8]) as f64 / 255.0;
            Ok((input, Color::from_rgba(r, g, b, a)))
        }
        // RGBA
        4 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..1]);
            let g = hex_to_u8_unsafe(&hex_chars[1..2]);
            let b = hex_to_u8_unsafe(&hex_chars[2..3]);
            let a = hex_to_u8_unsafe(&hex_chars[3..4]);
            let r = r * 16 + r;
            let g = g * 16 + g;
            let b = b * 16 + b;
            let a = (a * 16 + a) as f64 / 255.0;
            Ok((input, Color::from_rgba(r, g, b, a)))
        }
        _ => Err(Err::Error(nom::error::Error::new(
            "Expected hex string of 3 or 6 characters length",
            ErrorKind::Many1,
        ))),
    }
}

fn parse_numeric_rgb(input: &str) -> IResult<&str, Color> {
    let (input, prefixed) = opt(alt((tag("rgb("), tag("rgba("))))(input)?;
    let is_prefixed = prefixed.is_some();
    let (input, _) = space0(input)?;
    let (input, r) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, g) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = cond(is_prefixed, char(')'))(input)?;

    let r = r / 255.;
    let g = g / 255.;
    let b = b / 255.;

    let c = Color::from_rgba_float(r, g, b, alpha);
    Ok((input, c))
}

fn parse_css_numeric_rgb(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("rgb("), tag_no_case("rgba(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, r) = double(input)?;
    let (input, _) = space1(input)?;
    let (input, g) = double(input)?;
    let (input, _) = space1(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let r = r / 255.;
    let g = g / 255.;
    let b = b / 255.;

    let c = Color::from_rgba_float(r, g, b, alpha);
    Ok((input, c))
}

fn parse_percentage_rgb(input: &str) -> IResult<&str, Color> {
    let (input, prefixed) = opt(alt((tag("rgb("), tag("rgba("))))(input)?;
    let is_prefixed = prefixed.is_some();
    let (input, _) = space0(input)?;
    let (input, r) = percentage(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, g) = percentage(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, b) = percentage(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = cond(is_prefixed, char(')'))(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);
    Ok((input, c))
}

fn parse_css_percentage_rgb(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("rgb("), tag_no_case("rgba(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, r) = percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, g) = percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, b) = percentage(input)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);
    Ok((input, c))
}

fn parse_srgb_color_space(input: &str) -> IResult<&str, Color> {
    fn srgb_components(input: &str) -> IResult<&str, Color> {
        let (input, r) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, g) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, b) = number_or_percentage(input, 1.0)?;

        let c = Color::from_rgb_float(r, g, b);
        Ok((input, c))
    }

    css_color_function(tag_no_case("srgb"), srgb_components)(input)
}

fn parse_srgb_linear_color_space(input: &str) -> IResult<&str, Color> {
    fn lin_srgb_components(input: &str) -> IResult<&str, Color> {
        let (input, r_) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, g_) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, b_) = number_or_percentage(input, 1.0)?;

        let [r, g, b] = gam_srgb([r_, g_, b_]);
        let c = Color::from_rgb_float(r, g, b);
        Ok((input, c))
    }

    css_color_function(tag_no_case("srgb-linear"), lin_srgb_components)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_rgb_almost_equal;

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
    fn roundtrip_out_of_gamut_srgb() {
        // Display-P3 primaries and secondaries are out-of-gamut in sRGB, but
        // these colors should still round-trip through `Color`.
        let roundtrip = |r, g, b| {
            let rgb1 = Srgba::new(r, g, b);
            let rgb2 = Srgba::from(Color::from(rgb1));
            assert_rgb_almost_equal(&rgb1, &rgb2);
        };

        roundtrip(1.0931, -0.2267, -0.1501); // red primary
        roundtrip(-0.5116, 1.0183, -0.3107); // green primary
        roundtrip(0.0, 0.0, 1.042); // blue primary

        roundtrip(1.0, 1.0, -0.3463); // yellow secondary
        roundtrip(-0.5116, 1.0183, 1.0086); // cyan secondary
        roundtrip(1.0931, -0.2267, 1.0338); // magenta secondary
    }

    #[test]
    fn rgb_to_u32_conversion() {
        assert_eq!(0, rgb(0, 0, 0).to_u32());
        assert_eq!(0xff0000, rgb(255, 0, 0).to_u32());
        assert_eq!(0xffff00, Color::yellow().to_u32());
        assert_eq!(0xff00ff, Color::fuchsia().to_u32());
        assert_eq!(0x00ffff, Color::aqua().to_u32());
        assert_eq!(0xffffff, rgb(255, 255, 255).to_u32());
        assert_eq!(0xf4230f, rgb(0xf4, 0x23, 0x0f).to_u32());
    }

    #[test]
    fn to_color_string_u8() {
        let c = Srgba::new(255, 127, 4);
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

        let c = Srgba::new(0.12, 0.45, 0.78);
        assert_eq!(
            "rgb(0.120, 0.450, 0.780)",
            c.to_color_string(Format::Spaces)
        );
    }

    #[test]
    fn to_rgb_hex_string() {
        let c = Srgba::new(255, 127, 4);
        assert_eq!("ff7f04", c.to_hex_string(false));
        assert_eq!("#ff7f04", c.to_hex_string(true));
    }

    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgb(r, g, b)
    }

    fn rgba(r: u8, g: u8, b: u8, a: f64) -> Color {
        Color::from_rgba(r, g, b, a)
    }

    fn parse_color(input: &str) -> Option<Color> {
        parse_rgb_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_rgb_hex_syntax() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("f09"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#f09"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#F09"));

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#ff0099"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#FF0099"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("ff0099"));

        assert_eq!(Some(rgb(87, 166, 206)), parse_color("57A6CE"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("  #ff0077  "));

        assert_eq!(None, parse_color("#1"));
        assert_eq!(None, parse_color("#12"));
        assert_eq!(None, parse_color("#12345"));
        assert_eq!(None, parse_color("#1234567"));
        assert_eq!(None, parse_color("#hh0033"));
        assert_eq!(None, parse_color("#h03"));
    }

    #[test]
    fn parse_rgb_hex_alpha() {
        assert_eq!(Some(rgba(17, 51, 85, 0.6)), parse_color("#11335599"));
        assert_eq!(Some(rgba(17, 51, 85, 0.6)), parse_color("11335599"));
        assert_eq!(Some(rgba(17, 51, 85, 0.6)), parse_color("#1359"));

        assert_eq!(Some(rgb(255, 0, 0)), parse_color("ff0000ff"));
        assert_eq!(Some(rgb(255, 0, 0)), parse_color("#ff0000ff"));
    }

    #[test]
    fn parse_rgb_functional_syntax() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255,0,153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255, 0, 153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb( 255 , 0 , 153 )"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255, 0, 153.0)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153)"));

        assert_eq!(
            Some(rgb(255, 8, 119)),
            parse_color("  rgb( 255  ,  8  ,  119 )  ")
        );

        assert_eq!(Some(rgb(255, 0, 127)), parse_color("rgb(100%,0%,49.8%)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%,0%,60%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(100%,0%,46.7%)"));
        assert_eq!(Some(rgb(3, 54, 119)), parse_color("rgb(1%,21.2%,46.7%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(255 0 119)"));
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("rgb(    255      0      119)")
        );

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%,0%,60%)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%, 0%, 60%)"));
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("rgb( 100% , 0% , 60% )")
        );
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100% 0% 60%)"));

        assert_eq!(Some(rgb(100, 5, 1)), parse_color("rgb(1e2, .5e1, .5e0)"));
        assert_eq!(Some(rgb(140, 0, 153)), parse_color("rgb(55% 0% 60%)"));
        assert_eq!(Some(rgb(142, 0, 153)), parse_color("rgb(55.5% 0% 60%)"));
        assert_eq!(Some(rgb(255, 0, 0)), parse_color("rgb(256,0,0)"));
        assert_eq!(Some(rgb(255, 255, 0)), parse_color("rgb(100%,100%,-45%)"));

        assert_eq!(None, parse_color("rgb(255,0)"));
        assert_eq!(None, parse_color("rgb(255,0,0"));
        assert_eq!(None, parse_color("rgb (256,0,0)"));
        assert_eq!(None, parse_color("rgb(100%,0,0)"));
        assert_eq!(None, parse_color("rgb(2550119)"));
    }

    #[test]
    fn parse_legacy_rgb_alpha() {
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgb(10,0,0,1)"));
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgb(10,0,0, 1)"));
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0,1)"));
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0, 1)"));
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0,1.0)"));
        assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0, 1.0)"));

        assert_eq!(
            Some(rgba(10, 0, 0, 0.5)),
            parse_color("rgba(10, 0, 0, 0.5)")
        );
        assert_eq!(
            Some(rgba(10, 0, 0, 0.5)),
            parse_color("rgba(10, 0, 0, 50%)")
        );
        assert_eq!(
            Some(rgba(10, 0, 0, 0.33)),
            parse_color("rgba(10, 0, 0, 0.33)")
        );
        assert_eq!(
            Some(rgba(10, 0, 0, 0.33)),
            parse_color("rgba(10, 0, 0, 33%)")
        );
    }

    #[test]
    fn parse_css_rgb_syntax() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb( 255  0  153 )"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153.0)"));
        // `rgba` is equivalent to `rgb`
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgba(255 0 153)"));

        assert_eq!(
            Some(rgb(255, 8, 119)),
            parse_color("  rgb( 255    8    119 )  ")
        );

        assert_eq!(Some(rgb(255, 0, 127)), parse_color("rgb(100% 0% 49.8%)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100% 0% 60%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(100% 0% 46.7%)"));
        assert_eq!(Some(rgb(3, 54, 119)), parse_color("rgb(1% 21.2% 46.7%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(255 0 119)"));
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("rgb(    255      0      119)")
        );
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb( 100%   0%  60% )"));

        assert_eq!(Some(rgb(100, 5, 1)), parse_color("rgb(1e2 .5E1 .5e0)"));
        assert_eq!(Some(rgb(140, 0, 153)), parse_color("rgb(55% 0% 60%)"));
        assert_eq!(Some(rgb(142, 0, 153)), parse_color("rgb(55.5% 0% 60%)"));
        assert_eq!(Some(rgb(255, 0, 0)), parse_color("rgb(256,0,0)"));
        assert_eq!(Some(rgb(255, 255, 0)), parse_color("rgb(100%,100%,-45%)"));

        // function names are case-insensitive
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("RGB(255 0 153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("RgbA(255 0 153)"));

        assert_eq!(None, parse_color("rgb(255 0)"));
        assert_eq!(None, parse_color("rgb(255 0 0"));
        assert_eq!(None, parse_color("rgb (256 0 0)"));
        assert_eq!(None, parse_color("rgb(100% 0 0)"));
        assert_eq!(None, parse_color("rgb(2550119)"));
    }

    #[test]
    fn parse_css_rgb_alpha() {
        // alpha can be specified as a number from 0.0 to 1.0, or as a percentage
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgb(10 20 30 / 0.7)")
        );
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgb(10 20 30 / 70%)")
        );

        // `rgba` is equivalent to `rgb`
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgba(10 20 30 / 0.7)")
        );

        // spaces are not required around the '/' separator
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgb(10 20 30/0.7)")
        );
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgb(10 20 30/70%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(rgba(10, 20, 30, 0.7)),
            parse_color("rgb(10   20   30  /  0.7)")
        );

        // alpha can also be combined with percentage channel values
        assert_eq!(
            Some(rgba(255, 0, 128, 0.7)),
            parse_color("rgb(100% 0% 50% / 0.7)")
        );

        // an explicit 100% (or 1.0) alpha is valid
        assert_eq!(
            Some(rgba(10, 20, 30, 1.0)),
            parse_color("rgb(10 20 30 / 1.0)")
        );
        assert_eq!(
            Some(rgba(10, 20, 30, 1.0)),
            parse_color("rgb(10 20 30 / 1)")
        );
        assert_eq!(
            Some(rgba(10, 20, 30, 1.0)),
            parse_color("rgb(10 20 30 / 100%)")
        );
    }

    #[test]
    fn parse_rgb_standalone_syntax() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("255,0,153"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("255, 0, 153"));
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("  255  ,  0  ,  153   ")
        );
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("255 0 153"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("255 0 153.0"));

        assert_eq!(Some(rgb(1, 2, 3)), parse_color("1,2,3"));
    }

    #[test]
    fn parse_srgb_color_space_syntax() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("color(srgb 1 0 0.6)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("color(srgb 1.0 0 0.6)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("color(srgb 1 0 0.467)"));

        assert_eq!(
            Some(rgb(255, 0, 127)),
            parse_color("color(srgb 100% 0% 49.8%)")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb 100% 0% 60%)")
        );
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("color(srgb 100% 0% 46.7%)")
        );
        assert_eq!(
            Some(rgb(3, 54, 119)),
            parse_color("color(srgb 1% 21.2% 46.7%)")
        );
        assert_eq!(
            Some(rgb(140, 0, 153)),
            parse_color("color(srgb 55% 0% 60%)")
        );
        assert_eq!(
            Some(rgb(142, 0, 153)),
            parse_color("color(srgb 55.5% 0% 60%)")
        );

        // numbers and percentages can be mixed
        assert_eq!(Some(rgb(140, 0, 153)), parse_color("color(srgb 55% 0 0.6)"));

        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb  1  0  0.6 )")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb  100%   0%  60% )")
        );
        assert_eq!(
            Some(rgb(255, 8, 119)),
            parse_color("  color(srgb 1    0.031    0.467 )  ")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(   srgb   1 0 0.6)")
        );

        assert_eq!(Some(rgb(255, 0, 0)), parse_color("color(srgb 1.1 0 0)"));
        assert_eq!(
            Some(rgb(255, 255, 0)),
            parse_color("color(srgb 100% 100% -45%)")
        );

        // color space name is case-insensitive
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("color(SRGB 1 0 0.6)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("color(sRgb 1 0 0.6)"));

        // alpha is supported
        assert_eq!(
            Some(rgba(255, 0, 153, 0.9)),
            parse_color("color(srgb 1 0 0.6 / 0.9)")
        );
        assert_eq!(
            Some(rgba(255, 0, 153, 0.9)),
            parse_color("color(srgb 1 0 0.6 / 90%)")
        );

        assert_eq!(None, parse_color("color(srgb 1 0)"));
        assert_eq!(None, parse_color("color(srgb 1 0 0 1)"));
        assert_eq!(None, parse_color("color(srgb 1 0 0"));
        assert_eq!(None, parse_color("color (srgb 1.01 0 0)"));
        assert_eq!(None, parse_color("color(srgb 2550119)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(srgb 0.3, 0.5, 0.7)"));
    }

    #[test]
    fn parse_lin_srgb_color_space_syntax() {
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb-linear 1 0 0.31855)")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb-linear 1.0 0 0.31855)")
        );
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("color(srgb-linear 1 0 0.18447)")
        );

        assert_eq!(
            Some(rgb(255, 0, 127)),
            parse_color("color(srgb-linear 100% 0% 21.223%)")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb-linear 100% 0% 31.855%)")
        );
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("color(srgb-linear 100% 0% 18.447%)")
        );
        assert_eq!(
            Some(rgb(3, 54, 119)),
            parse_color("color(srgb-linear 0.09106% 3.6889% 18.447%)")
        );
        assert_eq!(
            Some(rgb(140, 0, 153)),
            parse_color("color(srgb-linear 26.225% 0% 31.855%)")
        );
        assert_eq!(
            Some(rgb(142, 0, 153)),
            parse_color("color(srgb-linear 27.05% 0% 31.855%)")
        );

        // numbers and percentages can be mixed
        assert_eq!(
            Some(rgb(140, 0, 153)),
            parse_color("color(srgb-linear 26.225% 0 0.31855)")
        );

        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb-linear  1  0  0.31855 )")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(srgb-linear  100%   0%  31.855% )")
        );
        assert_eq!(
            Some(rgb(255, 8, 119)),
            parse_color("  color(srgb-linear 1    0.0024282    0.18447 )  ")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(   srgb-linear   1 0 0.31855)")
        );

        assert_eq!(
            Some(rgb(255, 0, 0)),
            parse_color("color(srgb-linear 1.1 0 0)")
        );
        assert_eq!(
            Some(rgb(255, 255, 0)),
            parse_color("color(srgb-linear 100% 100% -45%)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(SRGB-linear 1 0 0.31855)")
        );
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("color(sRgb-Linear 1 0 0.31855)")
        );

        // alpha is supported
        assert_eq!(
            Some(rgba(255, 0, 153, 0.9)),
            parse_color("color(srgb-linear 1 0 0.31855 / 0.9)")
        );
        assert_eq!(
            Some(rgba(255, 0, 153, 0.9)),
            parse_color("color(srgb-linear 1 0 0.31855 / 90%)")
        );

        assert_eq!(None, parse_color("color(srgb-linear 1 0)"));
        assert_eq!(None, parse_color("color(srgb-linear 1 0 0 1)"));
        assert_eq!(None, parse_color("color(srgb-linear 1 0 0"));
        assert_eq!(None, parse_color("color (srgb 1.01 0 0)"));
        assert_eq!(None, parse_color("color(srgb-linear 2550119)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(srgb-linear 0.3, 0.5, 0.7)"));
    }
}
