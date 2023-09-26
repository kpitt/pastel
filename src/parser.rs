use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::character::complete::{char, space0, space1};
use nom::combinator::{all_consuming, opt, verify};
use nom::number::complete::double;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::{error::Error, IResult};

use crate::Color;
use crate::{
    cmyk::parse_cmyk_color, hsl::parse_hsl_color, hsv::parse_hsv_color, hwb::parse_hwb_color,
    lab::parse_lab_color, lch::parse_lch_color, named::parse_named_color, rgb::parse_rgb_color,
    xyz::parse_xyz_color,
};

fn comma_separator(input: &str) -> IResult<&str, &str> {
    let (input, _) = space0(input)?;
    let (input, _) = char(',')(input)?;
    space0(input)
}

pub(crate) fn legacy_separator(input: &str) -> IResult<&str, &str> {
    alt((comma_separator, space1))(input)
}

pub(crate) fn percentage(input: &str) -> IResult<&str, f64> {
    let (input, percent) = double(input)?;
    let (input, _) = char('%')(input)?;
    Ok((input, percent / 100.))
}

pub(crate) fn number_or_percentage(input: &str, scale: f64) -> IResult<&str, f64> {
    let (input, num) = double(input)?;
    let (input, percent_sign) = opt(char('%'))(input)?;
    let is_percentage = percent_sign.is_some();

    let value = if is_percentage {
        num * scale / 100.0
    } else {
        num
    };
    Ok((input, value))
}

fn angle_in_degrees(input: &str) -> IResult<&str, f64> {
    let (input, d) = double(input)?;
    let (input, _) = alt((tag("°"), tag("deg"), tag("")))(input)?;
    Ok((input, d))
}

fn angle_in_rads(input: &str) -> IResult<&str, f64> {
    let (input, rads) = double(input)?;
    let (input, _) = tag("rad")(input)?;
    Ok((input, rads * 180. / std::f64::consts::PI))
}

fn angle_in_grads(input: &str) -> IResult<&str, f64> {
    let (input, grads) = double(input)?;
    let (input, _) = tag("grad")(input)?;
    Ok((input, grads * 360. / 400.))
}

fn angle_in_turns(input: &str) -> IResult<&str, f64> {
    let (input, turns) = double(input)?;
    let (input, _) = tag("turn")(input)?;
    Ok((input, turns * 360.))
}

pub(crate) fn hue_angle(input: &str) -> IResult<&str, f64> {
    alt((
        angle_in_turns,
        angle_in_grads,
        angle_in_rads,
        angle_in_degrees,
    ))(input)
}

pub(crate) fn legacy_alpha<'a>(input: &'a str) -> IResult<&'a str, f64> {
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = legacy_separator(input)?;
        alt((percentage, double))(input)
    })(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

pub(crate) fn modern_alpha(input: &str) -> IResult<&str, f64> {
    let (input, alpha) = opt(preceded(
        delimited(space0, char('/'), space0),
        alt((percentage, double)),
    ))(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

fn parse_gray(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag("gray(")(input)?;
    let (input, _) = space0(input)?;
    let (input, g) = verify(alt((percentage, double)), |&d| d >= 0.)(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_rgb_float(g, g, g);

    Ok((input, c))
}

pub(crate) fn css_color_function<'a, O1, F, G>(
    mut color_name: F,
    mut color: G,
) -> impl FnMut(&'a str) -> IResult<&'a str, Color>
where
    F: Parser<&'a str, O1, Error<&'a str>>,
    G: Parser<&'a str, Color, Error<&'a str>>,
{
    move |input: &'a str| {
        let (input, _) = tag_no_case("color(")(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = color_name.parse(input)?;
        let (input, _) = space1(input)?;

        let (input, c) = color.parse(input)?;
        let (input, alpha) = modern_alpha(input)?;

        let (input, _) = space0(input)?;
        let (input, _) = char(')')(input)?;

        let c = if alpha == 1.0 { c } else { c.with_alpha(alpha) };
        Ok((input, c))
    }
}

pub fn parse_color(input: &str) -> Option<Color> {
    alt((
        parse_rgb_color,
        parse_hsl_color,
        parse_hsv_color,
        parse_hwb_color,
        all_consuming(parse_gray),
        parse_xyz_color,
        parse_lab_color,
        parse_lch_color,
        parse_cmyk_color,
        parse_named_color,
    ))(input.trim())
    .ok()
    .map(|(_, c)| c)
}

#[cfg(test)]
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

#[cfg(test)]
fn rgba(r: u8, g: u8, b: u8, a: f64) -> Color {
    Color::from_rgba(r, g, b, a)
}

#[test]
fn parse_hsl_string() {
    assert_eq!(
        Some(Color::from_hsl(280.0, 0.2, 0.5)),
        parse_color("hsl(280, 20%, 50%)")
    );
    assert_eq!(
        Some(Color::from_hsla(280.0, 0.2, 0.5, 0.75)),
        parse_color("hsla(280, 20%, 50%, 75%)")
    );

    assert_eq!(
        Some(Color::from_hsl(280.0, 0.2, 0.5)),
        parse_color("hsl(280 20% 50%)")
    );
    assert_eq!(
        Some(Color::from_hsla(280.0, 0.2, 0.5, 0.25)),
        parse_color("hsl(280 20% 50% / 25%)")
    );
}

#[test]
fn parse_rgb_string() {
    assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255, 0, 153)"));
    assert_eq!(
        Some(rgba(10, 0, 0, 0.5)),
        parse_color("rgba(10, 0, 0, 0.5)")
    );

    assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153)"));
    assert_eq!(
        Some(rgba(10, 20, 30, 0.7)),
        parse_color("rgb(10 20 30 / 0.7)")
    );

    assert_eq!(Some(rgb(140, 0, 153)), parse_color("rgb(55% 0% 60%)"));
    assert_eq!(
        Some(rgba(255, 0, 128, 0.7)),
        parse_color("rgb(100% 0% 50% / 0.7)")
    );

    assert_eq!(Some(rgb(255, 0, 153)), parse_color("#ff0099"));
    assert_eq!(Some(rgba(17, 51, 85, 0.6)), parse_color("#11335599"));
}

#[test]
fn parse_hsv_string() {
    assert_eq!(
        Some(Color::from_hsv(280.0, 0.2, 0.5)),
        parse_color("hsv(280, 20%, 50%)")
    );

    assert_eq!(
        Some(Color::from_hsv(280.0, 0.2, 0.5)),
        parse_color("hsv(280 20% 50%)")
    );
    assert_eq!(
        Some(Color::from_hsva(280.0, 0.2, 0.5, 0.5)),
        parse_color("hsv(280 20% 50% / 0.5)")
    );
}

#[test]
fn parse_hwb_string() {
    assert_eq!(
        Some(Color::from_hwb(280.0, 0.2, 0.5)),
        parse_color("hwb(280 20% 50%)")
    );

    assert_eq!(
        Some(Color::from_hwba(220.0, 0.25, 0.5, 0.2)),
        parse_color("hwb(220 25% 50% / 0.2)")
    );
}

#[test]
fn parse_gray_syntax() {
    assert_eq!(Some(Color::graytone(0.2)), parse_color("gray(0.2)"));
    assert_eq!(Some(Color::black()), parse_color("gray(0.0)"));
    assert_eq!(Some(Color::black()), parse_color("gray(0)"));
    assert_eq!(Some(Color::white()), parse_color("gray(1.0)"));
    assert_eq!(Some(Color::white()), parse_color("gray(1)"));
    assert_eq!(Some(Color::white()), parse_color("gray(7.3)"));

    assert_eq!(Some(Color::graytone(0.32)), parse_color("gray(.32)"));

    assert_eq!(
        Some(Color::graytone(0.41)),
        parse_color("  gray(  0.41   ) ")
    );

    assert_eq!(Some(Color::graytone(0.2)), parse_color("gray(20%)"));
    assert_eq!(Some(Color::black()), parse_color("gray(0%)"));
    assert_eq!(Some(Color::black()), parse_color("gray(0.0%)"));
    assert_eq!(Some(Color::white()), parse_color("gray(100%)"));
    assert_eq!(Some(Color::graytone(0.5)), parse_color("gray(50%)"));

    assert_eq!(None, parse_color("gray(-1)"));
    assert_eq!(None, parse_color("gray(-1%)"));
    assert_eq!(None, parse_color("gray(-4.%)"));
}

#[test]
fn parse_lab_string() {
    assert_eq!(
        Some(Color::from_lab(15.0, -23.0, 43.0, 1.0)),
        parse_color("lab(15, -23, 43)")
    );
    assert_eq!(
        Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
        parse_color("lab(15, -23, 43, 0.5)")
    );

    assert_eq!(
        Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
        parse_color("lab65(15 23 -43)")
    );
    assert_eq!(
        Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
        parse_color("lab65(15 -23 43 / 0.5)")
    );
}

#[test]
fn parse_lch_string() {
    assert_eq!(
        Some(Color::from_lch(15.0, 23.0, 45.0, 1.0)),
        parse_color("lch(15, 23, 45)")
    );
    assert_eq!(
        Some(Color::from_lch(75.0, 40.0, 220.0, 0.5)),
        parse_color("lch(75, 40, 220, 0.5)")
    );

    assert_eq!(
        Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
        parse_color("lch65(15 25 90)")
    );
    assert_eq!(
        Some(Color::from_lch(15.0, 23.0, 43.0, 0.5)),
        parse_color("lch65(15 23 43 / 0.5)")
    );
}

#[test]
fn parse_named_color_string() {
    assert_eq!(Some(Color::blue()), parse_color("blue"));
    assert_eq!(Some(rgb(255, 20, 147)), parse_color("deeppink"));
}

#[test]
fn parse_color_srgb_string() {
    assert_eq!(Some(rgb(255, 0, 153)), parse_color("color(srgb 1 0 0.6)"));
    assert_eq!(
        Some(rgba(255, 0, 153, 0.9)),
        parse_color("color(srgb 1 0 0.6 / 0.9)")
    );

    assert_eq!(
        Some(rgb(255, 0, 153)),
        parse_color("color(srgb-linear 1 0 0.31855)")
    );
    assert_eq!(
        Some(rgba(255, 0, 153, 0.9)),
        parse_color("color(srgb-linear 1 0 0.31855 / 0.9)")
    );
}

#[test]
fn parse_color_xyz_d65_string() {
    fn xyz(x: f64, y: f64, z: f64) -> Color {
        Color::from_xyz(x, y, z, 1.0)
    }

    assert_eq!(
        Some(xyz(0.3, 0.5, 0.7)),
        parse_color("color(xyz-d65 0.3 0.5 0.7)")
    );
    assert_eq!(
        Some(xyz(0.3, 0.5, 0.7)),
        parse_color("color(xyz 0.3 0.5 0.7)")
    );
    assert_eq!(
        Some(Color::from_xyz(0.3, 0.5, 0.7, 0.9)),
        parse_color("color(xyz-d65 0.3 0.5 0.7 / 0.9)")
    );
}

#[test]
fn parse_color_lab_d65_string() {
    assert_eq!(
        Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
        parse_color("color(lab-d65 15 23 -43)")
    );
    assert_eq!(
        Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
        parse_color("color(lab-d65 15 -23 43 / 0.5)")
    );
}

#[test]
fn parse_color_lch_d65_string() {
    assert_eq!(
        Some(Color::from_lch(15.0, 25.0, 90.0, 1.0)),
        parse_color("color(lch-d65 15 25 90)")
    );
    assert_eq!(
        Some(Color::from_lch(15.0, 23.0, 43.0, 0.5)),
        parse_color("color(lch-d65 15 23 43 / 0.5)")
    );
}

#[test]
fn parse_color_hsv_string() {
    assert_eq!(
        Some(Color::from_hsv(280.0, 0.2, 0.5)),
        parse_color("color(hsv 280 20% 50%)")
    );
    assert_eq!(
        Some(Color::from_hsva(280.0, 0.2, 0.5, 0.5)),
        parse_color("color(hsv 280 20% 50% / 0.5)")
    );
}

#[test]
fn parse_colorspace_ci() {
    // This tests case-insensitivity for the outer `color()` function only.
    // Case-insensitivity for each color space name should be tested separately.

    fn xyz(x: f64, y: f64, z: f64) -> Color {
        Color::from_xyz(x, y, z, 1.0)
    }

    assert_eq!(
        Some(xyz(0.3, 0.5, 0.7)),
        parse_color("Color(xyz 0.3 0.5 0.7)")
    );
    assert_eq!(
        Some(xyz(0.3, 0.5, 0.7)),
        parse_color("COLOR(xyz 0.3 0.5 0.7)")
    );
    assert_eq!(
        Some(xyz(0.3, 0.5, 0.7)),
        parse_color("cOLOr(xyz 0.3 0.5 0.7)")
    );
}

#[test]
fn parse_undefined_colorspace() {
    assert_eq!(None, parse_color("color(qqqq 0.1 0.2 0.3 0.4)"));
}

#[test]
fn parse_device_cmyk_string() {
    assert_eq!(
        Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
        parse_color("device-cmyk(80% 20% 60% 40%)")
    );
    assert_eq!(
        Some(Color::from_cmyk(0.8, 0.2, 0.6, 0.4)),
        parse_color("device-cmyk(0.8 0.2 0.6 0.4)")
    );
}
