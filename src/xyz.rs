use std::fmt;

use nom::{
    branch::alt, bytes::complete::tag_no_case, character::complete::space1,
    combinator::all_consuming, IResult,
};

use crate::{
    convert::{gam_srgb, lin_srgb},
    matrix::mat3_dot,
    parser::{css_color_function, number_or_percentage},
    types::{Mat3, Scalar},
    Color, Srgba,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Xyz {
    pub x: Scalar,
    pub y: Scalar,
    pub z: Scalar,
    pub alpha: Scalar,
}

impl From<&Color> for Xyz {
    fn from(color: &Color) -> Self {
        #[rustfmt::skip]
        const M: Mat3 = [
            0.4124, 0.3576, 0.1805,
            0.2126, 0.7152, 0.0722,
            0.0193, 0.1192, 0.9505,
        ];

        let rec = Srgba::from(color);
        let r_g_b_ = lin_srgb([rec.r, rec.g, rec.b]);
        let [x, y, z] = mat3_dot(M, r_g_b_);

        Xyz::with_alpha(x, y, z, color.alpha)
    }
}

impl From<&Xyz> for Color {
    fn from(color: &Xyz) -> Self {
        #[rustfmt::skip]
        const M_: Mat3 = [
              3.2406, -1.5372, -0.4986,
             -0.9689,  1.8758,  0.0415,
              0.0557, -0.2040,  1.0570,
        ];

        let r_g_b_ = mat3_dot(M_, [color.x, color.y, color.z]);
        let [r, g, b] = gam_srgb(r_g_b_);
        Self::from(&Srgba::with_alpha(r, g, b, color.alpha))
    }
}

impl fmt::Display for Xyz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XYZ({x}, {y}, {z})", x = self.x, y = self.y, z = self.z,)
    }
}

impl Xyz {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self::with_alpha(x, y, z, 1.0)
    }

    #[inline]
    pub fn with_alpha(x: Scalar, y: Scalar, z: Scalar, alpha: Scalar) -> Self {
        Xyz { x, y, z, alpha }
    }
}

pub(crate) fn parse_xyz_color(input: &str) -> IResult<&str, Color> {
    alt((all_consuming(parse_xyz_d65_color_space),))(input.trim())
}

// CSS Color 4 defines separate D65-adapted (`xyz-d65`, or just `xyz`) and D5-adapted (`xyz-d50`)
// color spaces.  Currently, `pastel` does not support chromatic adaptation, and only uses the D65
// illuminant, so we only support the `xyz-d65` color space here.
fn parse_xyz_d65_color_space(input: &str) -> IResult<&str, Color> {
    fn xyz_components(input: &str) -> IResult<&str, Color> {
        let (input, x) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, y) = number_or_percentage(input, 1.0)?;
        let (input, _) = space1(input)?;
        let (input, z) = number_or_percentage(input, 1.0)?;

        let c = Color::from_xyz(x, y, z, 1.0);
        Ok((input, c))
    }

    let xyz_name = alt((tag_no_case("xyz-d65"), tag_no_case("xyz")));
    css_color_function(xyz_name, xyz_components)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::assert_almost_equal;

    #[test]
    fn xyz_conversion() {
        assert_eq!(Color::white(), Color::from_xyz(0.9505, 1.0, 1.0890, 1.0));
        assert_eq!(Color::red(), Color::from_xyz(0.4123, 0.2126, 0.01933, 1.0));
        assert_eq!(
            Color::from_hsl(109.999, 0.08654, 0.407843),
            Color::from_xyz(0.13123, 0.15372, 0.13174, 1.0)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let xyz1 = color1.to_xyz();
            let color2 = Color::from_xyz(xyz1.x, xyz1.y, xyz1.z, 1.0);
            assert_almost_equal(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    fn parse_color(input: &str) -> Option<Color> {
        parse_xyz_color(input).ok().map(|(_, c)| c)
    }

    #[test]
    fn parse_xyz65_color_space_syntax() {
        fn xyz(x: f64, y: f64, z: f64) -> Color {
            Color::from_xyz(x, y, z, 1.0)
        }

        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz-d65 0.3 0.5 0.7)")
        );

        assert_eq!(
            Some(xyz(0.950_470, 1.0, 1.088_830)),
            parse_color("color(xyz-d65 0.950470 1 1.088830)")
        );

        assert_eq!(
            Some(xyz(-0.004, 1.007, -1.2222)),
            parse_color("color(xyz-d65 -0.004 1.007000 -1.2222)")
        );

        // percentages are allowed
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz-d65 30% 50% 70%)")
        );
        // numbers and percentages can be mixed
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz-d65 0.3 50% 0.7)")
        );

        // `xyz` is equivalent to `xyz-d65`
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz 0.3 0.5 0.7)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz-d65  0.3   0.5    0.7)")
        );
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(   xyz-d65   0.3 0.5 0.7)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(XYZ 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(Xyz 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz-D65 0.3 0.5 0.7)")
        );

        // alpha is supported
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz-d65 0.3 0.5 0.7 / 0.9)")
        );

        // not enough parameters
        assert_eq!(None, parse_color("color(xyz-d65 0.3 0.5)"));
        // too many parameters
        assert_eq!(None, parse_color("color(xyz-d65 0.3 0.5 0.7 1.0)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(xyz-d65 0.3, 0.5, 0.7)"));
    }

    #[test]
    fn parse_xyz50_color_space_syntax() {
        // The D50-adapted `xyz-d50` color space is not currently supported.
        assert_eq!(None, parse_color("color(xyz-d50 0.3 0.5 0.7)"));
    }
}
