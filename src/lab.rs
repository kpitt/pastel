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
    helper::{interpolate, MaxPrecision},
    parser::{
        css_color_function, legacy_alpha, legacy_separator, modern_alpha, number_or_percentage,
    },
    types::Scalar,
    xyz::Xyz,
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
        let rec = Xyz::from(color);

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

        Self::from(&Xyz::with_alpha(x, y, z, color.alpha))
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

pub(crate) fn parse_lab_color(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_css_lab65),
        all_consuming(parse_legacy_lab),
        all_consuming(parse_lab_d65_color_space),
    ))(input.trim())
}

fn parse_legacy_lab(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lab(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, a) = double(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lab(l, a, b, alpha);

    Ok((input, c))
}

// CSS Color 4 only defines `lab()` and `lch()` color functions for the D50 illuminant.  We support
// alternate functions for the D65 illuminent used by `pastel` that are consistent with the
// CSS-defined functions, and that provide compatibility with the serialization formats of two
// popular and extensive JavaScript color libraries:
//
//   - color.js (https://colorjs.io) by CSS Color 4/5 co-editors Lea Verou and Chris Lilley
//   - culori (https://culorijs.org)

// The `lab-d65()` and `lch-d65()` function names are used by the "color.js" library.  The `lab65()`
// and `lch65()` provide simpler alternatives.

fn parse_css_lab65(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("lab65("), tag_no_case("lab-d65(")))(input)?;
    let (input, _) = space0(input)?;
    // Percent reference range for L: 0% = 0, 100% = 100
    let (input, l) = number_or_percentage(input, 100.0)?;
    let (input, _) = space1(input)?;
    // Percent reference range for a and b: -100% = -125, 100% = 125
    let (input, a) = number_or_percentage(input, 125.0)?;
    let (input, _) = space1(input)?;
    let (input, b) = number_or_percentage(input, 125.0)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lab(l, a, b, alpha);

    Ok((input, c))
}

// The "culori" library uses custom `--lab-d65` and `--lch-d65` color space names, consistent with
// the CSS Color 5 draft.  Percentage values are not supported here because the CSS `color()`
// function defines that 100% = 1.0 for all component values, so percentages would produce
// inconsistent and confusing values.

fn parse_lab_d65_color_space(input: &str) -> IResult<&str, Color> {
    fn lab_components(input: &str) -> IResult<&str, Color> {
        // Don't allow percentages because 100% = 1.0 for color space components.
        let (input, l) = double(input)?;
        let (input, _) = space1(input)?;
        let (input, a) = double(input)?;
        let (input, _) = space1(input)?;
        let (input, b) = double(input)?;

        let c = Color::from_lab(l, a, b, 1.0);
        Ok((input, c))
    }

    // Custom color space "<dashed-ident>" prefix is optional.
    let lab_name = preceded(opt(tag("--")), tag_no_case("lab-d65"));
    css_color_function(lab_name, lab_components)(input)
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

    fn parse_color(input: &str) -> Option<Color> {
        parse_lab_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_lab_syntax() {
        assert_eq!(
            Some(Color::from_lab(12.43, -35.5, 43.4, 1.0)),
            parse_color("Lab(12.43,-35.5,43.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
            parse_color("lab(15,-23,43,0.5)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELab(15,23,-43)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 35.5, -43.4, 1.0)),
            parse_color("CIELab(15,35.5,-43.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("cieLab(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("Lab(        15,  23,-43   )")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("CieLab(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELab(        15,  23,-43   )")
        );

        assert_eq!(
            Some(Color::from_lab(10.0, 30.0, 50.0, 1.0)),
            parse_color("lab(10,30,50,1)")
        );
        assert_eq!(
            Some(Color::from_lab(10.0, 30.0, 50.0, 1.0)),
            parse_color("lab(10,30,50,1.0)")
        );
    }

    #[test]
    fn parse_css_lab65_syntax() {
        assert_eq!(
            Some(Color::from_lab(12.43, -35.5, 43.4, 1.0)),
            parse_color("lab65(12.43 -35.5 43.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab65(15 23 -43)")
        );

        // lightness can be a percentage
        assert_eq!(
            Some(Color::from_lab(15.0, 25.0, 90.0, 1.0)),
            parse_color("lab65(15% 25 90)")
        );

        // a and b can be percentages, but 100% represents a value of 125
        assert_eq!(
            Some(Color::from_lab(15.0, 125.0, 0.0, 1.0)),
            parse_color("lab65(15% 100% 0%)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 0.0, -125.0, 1.0)),
            parse_color("lab65(15% 0% -100%)")
        );
        // numbers and percentages can be mixed
        assert_eq!(
            Some(Color::from_lab(15.0, 100.0, 80.0, 1.0)),
            parse_color("lab65(15% 80% 80.0)")
        );

        // `lab-d65` is equivalent to `lab65`
        assert_eq!(
            Some(Color::from_lab(15.0, 25.0, 90.0, 1.0)),
            parse_color("lab-d65(15% 25 90)")
        );

        // function names are case-insensitive
        assert_eq!(
            Some(Color::from_lab(15.0, 25.0, 90.0, 1.0)),
            parse_color("LAB65(15% 25 90)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 25.0, 90.0, 1.0)),
            parse_color("Lab-D65(15% 25 90)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
            parse_color("lab65(15% -23 43 / 0.5)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("lab65(15% -35.5 -43.4 / 40%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab65(    15%    23      -43)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 0.6)),
            parse_color("lab65(15%     23    -43/     60%   )")
        );

        // not enough parameters
        assert_eq!(None, parse_color("lab65(15 25)"));
        // too many parameters
        assert_eq!(None, parse_color("lab65(15% 25 90 120)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("lab65(15%, 25, 90)"));
    }

    #[test]
    fn parse_lab65_color_space_syntax() {
        assert_eq!(
            Some(Color::from_lab(12.43, -35.5, 43.4, 1.0)),
            parse_color("color(lab-d65 12.43 -35.5 43.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("color(lab-d65 15 23 -43)")
        );

        // `--lab-d65` is equivalent to `lab-d65`
        assert_eq!(
            Some(Color::from_lab(15.0, 25.0, 90.0, 1.0)),
            parse_color("color(--lab-d65 15 25 90)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(Color::from_lab(50.0, 10.0, 20.0, 1.0)),
            parse_color("color(Lab-D65 50 10 20)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
            parse_color("color(lab-d65 15 -23 43 / 0.5)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("color(lab-d65 15 -35.5 -43.4 / 40%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("color(lab-d65     15    23      -43)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("color(   lab-d65   15 23 -43)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 0.6)),
            parse_color("color(lab-d65   15     23    -43/     60%)")
        );

        // percentage values are not allowed
        assert_eq!(None, parse_color("color(lab-d65 15% 25 90)"));
        assert_eq!(None, parse_color("color(lab-d65 15% 100% 0%)"));
        assert_eq!(None, parse_color("color(lab-d65 15% 0% -100%)"));
        assert_eq!(None, parse_color("color(lab-d65 15% 80% 80.0)"));

        // not enough parameters
        assert_eq!(None, parse_color("color(lab-d65 15 25)"));
        // too many parameters
        assert_eq!(None, parse_color("color(lab-d65 15 25 90 120)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(lab-d65 15, 25, 90)"));
    }
}
