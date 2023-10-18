use std::fmt;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0, space1},
    combinator::all_consuming,
    IResult,
};

use crate::{
    colorspace::ColorSpace,
    helper::{clamp, interpolate, interpolate_angle, MaxPrecision},
    parser::{hue_angle, legacy_alpha, legacy_separator, modern_alpha, percentage},
    types::{Hue, Scalar},
    Color, Format, Fraction,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsla {
    pub h: Scalar,
    pub s: Scalar,
    pub l: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for Hsla {
    fn mix(self, other: Self, fraction: Fraction) -> Self {
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

impl From<Color> for Hsla {
    fn from(color: Color) -> Self {
        Hsla {
            h: color.hue.value(),
            s: color.saturation,
            l: color.lightness,
            alpha: color.alpha,
        }
    }
}

impl From<Hsla> for Color {
    fn from(color: Hsla) -> Self {
        Color {
            hue: Hue::from(color.h),
            saturation: clamp(0.0, 1.0, color.s),
            lightness: clamp(0.0, 1.0, color.l),
            alpha: clamp(0.0, 1.0, color.alpha),
        }
    }
}

impl fmt::Display for Hsla {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsl({h}, {s}, {l})", h = self.h, s = self.s, l = self.l,)
    }
}

impl Hsla {
    #[inline]
    pub fn new(h: Scalar, s: Scalar, l: Scalar) -> Self {
        Self::with_alpha(h, s, l, 1.0)
    }

    #[inline]
    pub fn with_alpha(h: Scalar, s: Scalar, l: Scalar, alpha: Scalar) -> Self {
        Hsla { h, s, l, alpha }
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

pub(crate) fn parse_hsl_color(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_css_hsl),
        all_consuming(parse_legacy_hsl),
    ))(input.trim())
}

fn parse_legacy_hsl(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag("hsl("), tag("hsla(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, s) = percentage(input)?;
    let (input, _) = legacy_separator(input)?;
    let (input, l) = percentage(input)?;
    let (input, alpha) = legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsla(h, s, l, alpha);
    Ok((input, c))
}

fn parse_css_hsl(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("hsl("), tag_no_case("hsla(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = space1(input)?;
    let (input, s) = percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, l) = percentage(input)?;
    let (input, alpha) = modern_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsla(h, s, l, alpha);
    Ok((input, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_color_string() {
        let c = Hsla {
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
        let base = Hsla::new(hue, 0.5, 0.5);

        let hue_after_mixing = |other| base.mix(Hsla::from(other), Fraction::from(0.5)).h;

        assert_eq!(hue, hue_after_mixing(Color::black()));
        assert_eq!(hue, hue_after_mixing(Color::graytone(0.2)));
        assert_eq!(hue, hue_after_mixing(Color::graytone(0.7)));
        assert_eq!(hue, hue_after_mixing(Color::white()));
    }

    fn parse_color(input: &str) -> Option<Color> {
        parse_hsl_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_legacy_hsl_syntax() {
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280deg,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280°,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.33, 0.123, 0.456)),
            parse_color("hsl(280.33001,12.3%,45.6%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(  280 , 20% , 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("hsl(270 60% 70%)")
        );

        assert_eq!(
            Some(Color::from_hsl(-140.0, 0.2, 0.5)),
            parse_color("hsl(-140°,20%,50%)")
        );

        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(100grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.05, 0.2, 0.5)),
            parse_color("hsl(1.5708rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(0.25turn,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(50grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(0.7854rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(0.125turn,20%,50%)")
        );

        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10,50%,50%,1)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10,50%,50%,1.0)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsla(10,50%,50%,1)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsla(10,50%,50%,1.0)")
        );
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.75)),
            parse_color("hsla(280, 20%, 50%, 75%)")
        );

        assert_eq!(None, parse_color("hsl(280,20%,50)"));
        assert_eq!(None, parse_color("hsl(280,20,50%)"));
        assert_eq!(None, parse_color("hsl(280%,20%,50%)"));
        assert_eq!(None, parse_color("hsl(280,20%)"));
    }

    #[test]
    fn parse_legacy_hsl_with_alpha() {
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10,50%,50%,1)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10,50%,50%,1.0)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsla(10,50%,50%,1)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsla(10,50%,50%,1.0)")
        );
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.75)),
            parse_color("hsla(280, 20%, 50%, 75%)")
        );
    }

    #[test]
    fn parse_css_hsl_syntax() {
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280deg 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(280° 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.33, 0.123, 0.456)),
            parse_color("hsl(280.33001 12.3% 45.6%)")
        );
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("hsl(  280  20%  50%)")
        );
        // `hsla` is equivalent to `hsl`
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("hsla(270 60% 70%)")
        );

        assert_eq!(
            Some(Color::from_hsl(-140.0, 0.2, 0.5)),
            parse_color("hsl(-140° 20% 50%)")
        );

        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(100grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.05, 0.2, 0.5)),
            parse_color("hsl(1.5708rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(0.25turn 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(50grad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(0.7854rad 20% 50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(45.0, 0.2, 0.5)),
            parse_color("hsl(0.125turn 20% 50%)")
        );

        // function names are case-insensitive
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("HSL(270 60% 70%)")
        );
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("HslA(270 60% 70%)")
        );

        assert_eq!(None, parse_color("hsl(280 20%,50)"));
        assert_eq!(None, parse_color("hsl(280 20 50%)"));
        assert_eq!(None, parse_color("hsl(280% 20% 50%)"));
        assert_eq!(None, parse_color("hsl(280 20%)"));
    }

    #[test]
    fn parse_css_hsl_alpha() {
        // alpha can be specified as a number from 0.0 to 1.0, or as a percentage
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsl(10 50% 50% / 0.7)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsl(10 50% 50% / 70%)")
        );

        // `hsla` is equivalent to `hsl`
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsla(10 50% 50% / 0.7)")
        );

        // spaces are not required around the '/' separator
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsl(10 50% 50%/0.7)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsl(10 50% 50%/70%)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 0.7)),
            parse_color("hsl(10   50%   50%  /  0.7)")
        );

        // an explicit 100% (or 1.0) alpha is valid
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10 50% 50% / 1.0)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10 50% 50% / 1)")
        );
        assert_eq!(
            Some(Color::from_hsla(10.0, 0.5, 0.5, 1.0)),
            parse_color("hsl(10 50% 50% / 100%)")
        );
    }
}
