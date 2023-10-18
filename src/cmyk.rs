use std::fmt;

use nom::{
    bytes::complete::tag_no_case,
    character::complete::{char, space0, space1},
    combinator::{all_consuming, opt},
    IResult,
};

use crate::{
    parser::{modern_alpha, number_or_percentage},
    rgb::Srgba,
    types::Scalar,
    Color, Format,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Cmyk {
    pub c: Scalar,
    pub m: Scalar,
    pub y: Scalar,
    pub k: Scalar,
}

impl From<Color> for Cmyk {
    fn from(color: Color) -> Self {
        let rgba = Srgba::<u8>::from(color);
        let r = (rgba.r as f64) / 255.0;
        let g = (rgba.g as f64) / 255.0;
        let b = (rgba.b as f64) / 255.0;
        let biggest = if r >= g && r >= b {
            r
        } else if g >= r && g >= b {
            g
        } else {
            b
        };
        let out_k = 1.0 - biggest;
        let out_c = (1.0 - r - out_k) / biggest;
        let out_m = (1.0 - g - out_k) / biggest;
        let out_y = (1.0 - b - out_k) / biggest;

        Cmyk {
            c: if out_c.is_nan() { 0.0 } else { out_c },
            m: if out_m.is_nan() { 0.0 } else { out_m },
            y: if out_y.is_nan() { 0.0 } else { out_y },
            k: out_k,
        }
    }
}

impl From<Cmyk> for Color {
    fn from(color: Cmyk) -> Self {
        #![allow(clippy::many_single_char_names)]
        let r = (1.0 - color.c) * (1.0 - color.k);
        let g = (1.0 - color.m) * (1.0 - color.k);
        let b = (1.0 - color.y) * (1.0 - color.k);

        Srgba::new(r, g, b).into()
    }
}

impl fmt::Display for Cmyk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cmyk({c}, {m}, {y}, {k})",
            c = self.c,
            m = self.m,
            y = self.y,
            k = self.k,
        )
    }
}

impl Cmyk {
    #[inline]
    pub fn new(c: Scalar, m: Scalar, y: Scalar, k: Scalar) -> Self {
        Cmyk { c, m, y, k }
    }

    /// Format the color as a CMYK-representation string (`cmyk(0, 50, 100, 100)`).
    pub fn to_color_string(&self, format: Format) -> String {
        format!(
            "cmyk({c},{space}{m},{space}{y},{space}{k})",
            c = (self.c * 100.0).round(),
            m = (self.m * 100.0).round(),
            y = (self.y * 100.0).round(),
            k = (self.k * 100.0).round(),
            space = if format == Format::Spaces { " " } else { "" }
        )
    }
}

pub(crate) fn parse_cmyk_color(input: &str) -> IResult<&str, Color> {
    all_consuming(parse_css_device_cmyk)(input.trim())
}

// Parse CMYK colors as the `device-cmyk()` function defined in CSS Color 5.  The simpler `cmyk()`
// is also accepted.  We have no color profile info here, so all CMYK colors are represented as
// uncalibrated colors using the naive RGB conversion.

fn parse_css_device_cmyk(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("device-"))(input)?;
    let (input, _) = tag_no_case("cmyk(")(input)?;
    let (input, _) = space0(input)?;
    let (input, c) = number_or_percentage(input, 1.0)?;
    let (input, _) = space1(input)?;
    let (input, m) = number_or_percentage(input, 1.0)?;
    let (input, _) = space1(input)?;
    let (input, y) = number_or_percentage(input, 1.0)?;
    let (input, _) = space1(input)?;
    let (input, k) = number_or_percentage(input, 1.0)?;
    // accept alpha component for compatibility, but not currently supported for CMYK
    let (input, _alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_cmyk(c, m, y, k);

    Ok((input, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_conversion() {
        assert_eq!(Color::aqua(), Color::from_cmyk(1.0, 0.0, 0.0, 0.0));
        assert_eq!(Color::fuchsia(), Color::from_cmyk(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Color::yellow(), Color::from_cmyk(0.0, 0.0, 1.0, 0.0));
        assert_eq!(Color::black(), Color::from_cmyk(0.0, 0.0, 0.0, 1.0));

        assert_eq!(Color::red(), Color::from_cmyk(0.0, 1.0, 1.0, 0.0));
        assert_eq!(Color::lime(), Color::from_cmyk(1.0, 0.0, 1.0, 0.0));
        assert_eq!(Color::blue(), Color::from_cmyk(1.0, 1.0, 0.0, 0.0));

        assert_eq!(Color::green(), Color::from_cmyk(1.0, 0.0, 1.0, 0.5));
    }

    #[test]
    fn to_cmyk_string() {
        let white = Cmyk::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!("cmyk(0, 0, 0, 0)", white.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,0,0,0)", white.to_color_string(Format::NoSpaces));

        let black = Cmyk::new(0.0, 0.0, 0.0, 1.0);
        assert_eq!("cmyk(0, 0, 0, 100)", black.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,0,0,100)", black.to_color_string(Format::NoSpaces));

        let gray = Cmyk::new(0.0, 0.0, 0.0, 0.75);
        assert_eq!("cmyk(0, 0, 0, 75)", gray.to_color_string(Format::Spaces));

        let c1 = Cmyk::new(0.0, 0.0, 0.95, 0.9);
        assert_eq!("cmyk(0, 0, 95, 90)", c1.to_color_string(Format::Spaces));

        let c2 = Cmyk::new(0.0, 0.14, 0.43, 0.47);
        assert_eq!("cmyk(0, 14, 43, 47)", c2.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,14,43,47)", c2.to_color_string(Format::NoSpaces));

        let c3 = Cmyk::new(0.25, 0.1, 0.0, 0.5);
        assert_eq!("cmyk(25, 10, 0, 50)", c3.to_color_string(Format::Spaces));
    }

    fn parse_color(input: &str) -> Option<Color> {
        parse_cmyk_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_css_device_cmyk_syntax() {
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(80% 20% 60% 40%)")
        );
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(0.8 0.2 0.6 0.4)")
        );
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(80% 0.2 0.6 40%)")
        );

        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(80.0% 20.000% 60% 40.%)")
        );
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(0.800 0.2 .6 0.40)")
        );

        assert_eq!(
            Some(Color::red()),
            parse_color("device-cmyk(0% 100% 100% 0%)")
        );
        assert_eq!(
            Some(Color::green()),
            parse_color("device-cmyk(100% 0% 100% 50%)")
        );
        assert_eq!(
            Some(Color::blue()),
            parse_color("device-cmyk(100% 100% 0% 0%)")
        );
        assert_eq!(Some(Color::yellow()), parse_color("device-cmyk(0 0 1 0)"));
        assert_eq!(
            Some(Color::from_rgb(255, 165, 0)), // orange
            parse_color("device-cmyk(0 0.353 1 0)")
        );
        assert_eq!(Some(Color::purple()), parse_color("device-cmyk(0 1 0 0.5)"));

        // `cmyk` is equivalent to `device-cmyk`
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("cmyk(0.8 0.2 0.6 0.4)")
        );

        // function names are case-insensitive
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("Device-CMYK(80% 20% 60% 40%)")
        );

        // alpha value is allowed for compatibility, but is ignored because it
        // isn't currently supported by the color library
        assert_eq!(
            Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
            parse_color("device-cmyk(80% 20% 60% 40% / 0.5)")
        );

        assert_eq!(None, parse_color("device-cmyk(0,1,1,0)"));
        assert_eq!(None, parse_color("device-cmyk(0 1 1)"));
        assert_eq!(None, parse_color("device-cmyk(0 1)"));
        assert_eq!(None, parse_color("device-cmyk(50%)"));
        assert_eq!(None, parse_color("device-cmyk(0 1 0.5 1 0)"));
    }
}
