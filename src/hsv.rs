use std::fmt;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0, space1},
    combinator::{all_consuming, opt},
    sequence::preceded,
    IResult,
};

use crate::{
    colorspace::ColorSpace,
    helper::{clamp, interpolate, interpolate_angle, MaxPrecision},
    parser::{
        css_color_function, hue_angle, legacy_alpha, legacy_separator, modern_alpha,
        number_or_percentage, percentage,
    },
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

pub(crate) fn parse_hsv_color(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_css_hsv),
        all_consuming(parse_legacy_hsv),
        all_consuming(parse_hsv_color_space),
    ))(input.trim())
}

fn parse_legacy_hsv(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag("hsv("), tag("hsva(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, s) = percentage(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, v) = percentage(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsva(h, s, v, alpha);
    Ok((input, c))
}

// For HSV colors, the `hsv()` function mirrors the modern syntax of the CSS `hsl()` function.  The
// legacy syntax is still handled by the original `pastel` HSV parser.

fn parse_css_hsv(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("hsv(")(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = space1(input)?;
    // Percent reference range for S and V: 0% = 0, 100% = 1
    let (input, s) = number_or_percentage(input, 1.0)?;
    let (input, _) = space1(input)?;
    let (input, v) = number_or_percentage(input, 1.0)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsva(h, s, v, alpha);
    Ok((input, c))
}

// The "culori" library uses the CSS `color()` function with a custom `--hsv` color space name.
// The "color.js" library also uses a custom color space, but unfortunately it is incompatible
// (at least as of v0.4.5) because it uses S and V component values in the 0 to 100 range instead
// of the 0 to 1 values used by `pastel` and "culori".

fn parse_hsv_color_space(input: &str) -> IResult<&str, Color> {
    fn hsv_components(input: &str) -> IResult<&str, Color> {
        // Optional angle units are allowed because they are not ambiguous.
        let (input, h) = hue_angle(input)?;
        let (input, _) = space1(input)?;
        // Percentages can be used with 0 to 1 values.
        let (input, s) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, v) = number_or_percentage(input, 1.0)?;

        let c = Color::from_hsv(h, s, v);
        Ok((input, c))
    }

    // Custom color space "<dashed-ident>" prefix is optional.
    let hsv_name = preceded(opt(tag("--")), tag_no_case("hsv"));
    css_color_function(hsv_name, hsv_components)(input)
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

    fn parse_color(input: &str) -> Option<Color> {
        parse_hsv_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_legacy_hsv_syntax() {
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280deg,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280°,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.33, 0.123, 0.456)),
            parse_color("hsv(280.33001,12.3%,45.6%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(  280 , 20% , 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(270.0, 0.6, 0.7)),
            parse_color("hsv(270 60% 70%)")
        );

        assert_eq!(
            Some(Color::from_hsv(-140.0, 0.2, 0.5)),
            parse_color("hsv(-140°,20%,50%)")
        );

        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("hsv(100grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.05, 0.2, 0.5)),
            parse_color("hsv(1.5708rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("hsv(0.25turn,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(50grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(0.7854rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(0.125turn,20%,50%)")
        );

        assert_eq!(None, parse_color("hsv(280,20%,50)"));
        assert_eq!(None, parse_color("hsv(280,20,50%)"));
        assert_eq!(None, parse_color("hsv(280%,20%,50%)"));
        assert_eq!(None, parse_color("hsv(280,20%)"));
    }

    #[test]
    fn parse_css_hsv_syntax() {
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.33, 0.123, 0.456)),
            parse_color("hsv(280.33 12.3% 45.6%)")
        );
        assert_eq!(
            Some(Color::from_hsv(270.0, 0.6, 0.7)),
            parse_color("hsv(270 60% 70%)")
        );
        assert_eq!(
            Some(Color::from_hsv(-140.0, 0.2, 0.5)),
            parse_color("hsv(-140 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(220.0, 0.2, 0.5)),
            parse_color("hsv(-140 20% 50%)")
        );

        // S and V can be numbers in the range 0 to 1
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280 0.2 0.5)")
        );
        // numbers and percentages can be mixed
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280 20% 0.5)")
        );

        // hue angle unit identifiers are supported
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(280deg 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("hsv(100grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.05, 0.2, 0.5)),
            parse_color("hsv(1.5708rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("hsv(0.25turn 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(50grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(0.7854rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("hsv(0.125turn 20% 50%)")
        );

        // function names are case-insensitive
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("HSV(280 20% 50%)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.5)),
            parse_color("hsv(280 20% 50% / 0.5)")
        );
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.75)),
            parse_color("hsv(280 20% 50% / 75%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsv(  280   20%   50%)")
        );
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.6)),
            parse_color("hsv(    280   20%    50%      /0.6  )")
        );

        // hue angle cannot be a percentage
        assert_eq!(None, parse_color("hsv(280% 20% 50%)"));
        // not enough parameters
        assert_eq!(None, parse_color("hsv(280 20%)"));
        // too many parameters
        // (the following produces a valid color due to legacy syntax)
        // assert_eq!(None, parse_color("hsv(280 20% 50% 0.75)"));
        assert_eq!(None, parse_color("hsv(280 20% 50% 0.75 0.5)"));
        // comma separators not allowed
        // (the following produces a valid color due to legacy syntax)
        // assert_eq!(None, parse_color("hsv(280, 20%, 50%)"));
    }

    #[test]
    fn parse_css_hsv_color_space_syntax() {
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(hsv 280 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.33, 0.123, 0.456)),
            parse_color("color(hsv 280.33 12.3% 45.6%)")
        );
        assert_eq!(
            Some(Color::from_hsv(270.0, 0.6, 0.7)),
            parse_color("color(hsv 270 60% 70%)")
        );
        assert_eq!(
            Some(Color::from_hsv(-140.0, 0.2, 0.5)),
            parse_color("color(hsv -140 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(220.0, 0.2, 0.5)),
            parse_color("color(hsv -140 20% 50%)")
        );

        // S and V can be numbers in the range 0 to 1
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(hsv 280 0.2 0.5)")
        );
        // numbers and percentages can be mixed
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(hsv 280 20% 0.5)")
        );

        // hue angle unit identifiers are supported
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(hsv 280deg 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("color(hsv 100grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.05, 0.2, 0.5)),
            parse_color("color(hsv 1.5708rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("color(hsv 0.25turn 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("color(hsv 50grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("color(hsv 0.7854rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(45.0, 0.2, 0.5)),
            parse_color("color(hsv 0.125turn 20% 50%)")
        );

        // `--hsv` is equivalent to `hsv`
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(--hsv 280 20% 50%)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(HSV 280 20% 50%)")
        );

        // alpha value is supported as a number or percentage
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.5)),
            parse_color("color(hsv 280 20% 50% / 0.5)")
        );
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.75)),
            parse_color("color(hsv 280 20% 50% / 75%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("color(hsv   280   20%   50%)")
        );
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.6)),
            parse_color("color(hsv     280   20%    50%      /0.6  )")
        );
        assert_eq!(
            Some(Color::from_hsva(280.0, 0.2, 0.5, 0.7)),
            parse_color("color(   hsv 280 20% 50%/    70%)")
        );

        // hue angle cannot be a percentage
        assert_eq!(None, parse_color("color(hsv 280% 20% 50%)"));
        // not enough parameters
        assert_eq!(None, parse_color("color(hsv 280 20%)"));
        // too many parameters
        assert_eq!(None, parse_color("color(hsv 280 20% 50% 0.75)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(hsv 280, 20%, 50%)"));
    }
}
