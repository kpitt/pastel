use std::fmt;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0, space1},
    combinator::{all_consuming, opt},
    number::complete::double,
    sequence::preceded,
    IResult,
};

use crate::{
    colorspace::ColorSpace,
    helper::{interpolate, interpolate_angle, mod_positive, MaxPrecision},
    lab::Lab,
    parser::{
        css_color_function, hue_angle, legacy_alpha, legacy_separator, modern_alpha,
        number_or_percentage,
    },
    types::Scalar,
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Lch {
    pub l: Scalar,
    pub c: Scalar,
    pub h: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for Lch {
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

impl From<Color> for Lch {
    fn from(color: Color) -> Self {
        let Lab { l, a, b, alpha } = Lab::from(color);

        const RAD2DEG: Scalar = 180.0 / std::f64::consts::PI;

        let c = Scalar::sqrt(a * a + b * b);
        let h = mod_positive(Scalar::atan2(b, a) * RAD2DEG, 360.0);

        Lch::with_alpha(l, c, h, alpha)
    }
}

impl From<Lch> for Color {
    fn from(color: Lch) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DEG2RAD: Scalar = std::f64::consts::PI / 180.0;

        let a = color.c * Scalar::cos(color.h * DEG2RAD);
        let b = color.c * Scalar::sin(color.h * DEG2RAD);

        Self::from(Lab::with_alpha(color.l, a, b, color.alpha))
    }
}

impl fmt::Display for Lch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LCh({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

impl Lch {
    #[inline]
    pub fn new(l: Scalar, c: Scalar, h: Scalar) -> Self {
        Self::with_alpha(l, c, h, 1.0)
    }

    #[inline]
    pub fn with_alpha(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Self {
        Lch { l, c, h, alpha }
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

pub(crate) fn parse_lch_color(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_css_lch65),
        all_consuming(parse_legacy_lch),
        all_consuming(parse_lch_d65_color_space),
    ))(input.trim())
}

fn parse_legacy_lch(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lch(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, c) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lch(l, c, h, alpha);
    Ok((input, c))
}

// The `lch-d65()` and `lch65()` function names differentiate these formats from the D50 illuminant
// functions in CSS Color 4.  See comments in the `lab` module for details.

fn parse_css_lch65(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("lch65("), tag_no_case("lch-d65(")))(input)?;
    let (input, _) = space0(input)?;
    // Percent reference range for L: 0% = 0, 100% = 100
    let (input, l) = number_or_percentage(input, 100.0)?;
    let (input, _) = space1(input)?;
    // Percent reference range for C: 0% = 0, 100% = 150
    let (input, c) = number_or_percentage(input, 150.0)?;
    let (input, _) = space1(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lch(l, c, h, alpha);
    Ok((input, c))
}

// The "culori" library uses custom `--lab-d65` and `--lch-d65` color space names, consistent with
// the CSS Color 5 draft.  Percentage values are not supported here because the CSS `color()`
// function defines that 100% = 1.0 for all component values, so percentages would produce
// inconsistent and confusing values.  Where there is no ambiguity, however, we try to be lenient
// for broader compatibility.

fn parse_lch_d65_color_space(input: &str) -> IResult<&str, Color> {
    fn lch_components(input: &str) -> IResult<&str, Color> {
        // Don't allow percentages because 100% = 1.0 for color space components.
        let (input, l) = double(input)?;
        let (input, _) = space1(input)?;
        let (input, c) = double(input)?;
        let (input, _) = space1(input)?;
        // Optional angle units are allowed because they are not ambiguous.
        let (input, h) = hue_angle(input)?;

        let c = Color::from_lch(l, c, h, 1.0);
        Ok((input, c))
    }

    // Custom color space "<dashed-ident>" prefix is optional.
    let lch_name = preceded(opt(tag("--")), tag_no_case("lch-d65"));
    css_color_function(lch_name, lch_components)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

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
        let c = Lch::new(52.0, 44.0, 271.0);
        assert_eq!("LCh(52, 44, 271)", c.to_color_string(Format::Spaces));
        assert_eq!("LCh(52,44,271)", c.to_color_string(Format::NoSpaces));
    }

    fn parse_color(input: &str) -> Option<Color> {
        parse_lch_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_lch_syntax() {
        assert_eq!(
            Some(Color::from_lch(12.43, -35.5, 43.4, 1.0)),
            parse_color("Lch(12.43,-35.5,43.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, -23.0, 43.0, 0.5)),
            parse_color("lch(15,-23,43,0.5)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELch(15,23,-43)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 35.5, -43.4, 1.0)),
            parse_color("CIELch(15,35.5,-43.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, -35.5, -43.4, 0.4)),
            parse_color("cieLch(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, -43.0, 1.0)),
            parse_color("Lch(        15,  23,-43   )")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, -35.5, -43.4, 0.4)),
            parse_color("CieLch(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELch(        15,  23,-43   )")
        );

        assert_eq!(
            Some(Color::from_lch(15.0, -23.0, 43.0, 1.0)),
            parse_color("lch(15,-23,43)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, -23.0, 43.0, 1.0)),
            parse_color("lch(15,-23,43°)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, -23.0, 43.0, 1.0)),
            parse_color("lch(15,-23,43deg)")
        );

        assert_eq!(None, parse_color("lch(15%,-23,43)"));
    }

    #[test]
    fn parse_css_lch65_syntax() {
        assert_eq!(
            Some(Color::from_lch(12.43, 35.5, 43.4, 1.0)),
            parse_color("lch65(12.43 35.5 43.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(15 25 90)")
        );

        // lightness can be a percentage
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(15% 25 90)")
        );

        // chroma can be a percentage, but 100% represents a value of 150
        assert_eq!(
            Some(Color::from_lch(15.0, 150.0, 90.0, 1.0)),
            parse_color("lch65(15% 100% 90)")
        );

        // hue angle can be negative
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, -45.0, 1.0)),
            parse_color("lch65(15% 25 -45)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 315.0, 1.0)),
            parse_color("lch65(15% 25 -45)")
        );

        // hue angle can include a unit identifier
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(15% 25 90deg)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(15% 25 100grad)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(15% 25 0.25turn)")
        );

        // `lch-d65` is equivalent to `lch65`
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch-d65(15% 25 90)")
        );

        // function names are case-insensitive
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("LCh65(15% 25 90)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("Lch-D65(15% 25 90)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, 43.0, 0.5)),
            parse_color("lch65(15 23 43 / 0.5)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, 43.0, 0.75)),
            parse_color("lch65(15 23 43 / 75%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("lch65(    15%    25      90)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 0.6)),
            parse_color("lch65(15%     25    90      /0.6   )")
        );

        // not enough parameters
        assert_eq!(None, parse_color("lch65(15 25)"));
        // too many parameters
        assert_eq!(None, parse_color("lch65(15% 25 90 120)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("lch65(15%, 25, 90)"));
    }

    #[test]
    fn parse_lch65_color_space_syntax() {
        assert_eq!(
            Some(Color::from_lch(12.43, 35.5, 43.4, 1.0)),
            parse_color("color(lch-d65 12.43 35.5 43.4)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(lch-d65 15 25 90)")
        );

        // hue angle can be negative
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, -45.0, 1.0)),
            parse_color("color(lch-d65 15 25 -45)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 315.0, 1.0)),
            parse_color("color(lch-d65 15 25 -45)")
        );

        // hue angle can include a unit identifier
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(lch-d65 15 25 90deg)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(lch-d65 15 25 100grad)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(lch-d65 15 25 0.25turn)")
        );

        // `--lch-d65` is equivalent to `lch-d65`
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(--lch-d65 15 25 90)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(Color::from_lch(50.0, 20.0, 180.0, 1.0)),
            parse_color("color(LCh-D65 50 20 180)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, 43.0, 0.5)),
            parse_color("color(lch-d65 15 23 43 / 0.5)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 23.0, 43.0, 0.75)),
            parse_color("color(lch-d65 15 23 43 / 75%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(lch-d65     15    25      90)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(   lch-d65   15 25 90)")
        );
        assert_eq!(
            Some(Color::from_lch(15.0, 25.0, 90.0, 0.6)),
            parse_color("color(lch-d65   15     25    90      /0.6)")
        );

        // percentage values are not allowed
        assert_eq!(None, parse_color("color(lch-d65 15% 25 90)"));
        assert_eq!(None, parse_color("color(lch-d65 15% 100% 90)"));

        // not enough parameters
        assert_eq!(None, parse_color("color(lch-d65 15 25)"));
        // too many parameters
        assert_eq!(None, parse_color("color(lch-d65 15 25 90 120)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(lch-d65 15, 25, 90)"));
    }
}
