use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::number::complete::double;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::{
    error::{Error, ErrorKind},
    Err, IResult,
};

use crate::{
    cmyk::parse_cmyk_color, convert::gam_srgb, hsl::parse_hsl_color, hsv::parse_hsv_color,
    hwb::parse_hwb_color, lab::parse_lab_color, lch::parse_lch_color, named::NAMED_COLORS,
    rgb::parse_rgb_color, Color,
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

fn parse_named(input: &str) -> IResult<&str, Color> {
    let (input, color) = all_consuming(alpha1)(input)?;
    let nc = NAMED_COLORS
        .iter()
        .find(|nc| color.to_lowercase() == nc.name);

    match nc {
        None => Err(Err::Error(nom::error::Error::new(
            "Couldn't find matching named color",
            ErrorKind::Alpha,
        ))),
        Some(nc) => Ok((input, nc.color.clone())),
    }
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

fn parse_css_color_fn(input: &str) -> IResult<&str, Color> {
    alt((
        all_consuming(parse_srgb_color_space),
        all_consuming(parse_lin_srgb_color_space),
        all_consuming(parse_cie_xyz65_color_space),
        all_consuming(parse_cie_lab65_color_space),
        all_consuming(parse_cie_lch65_color_space),
        all_consuming(parse_css_hsv_color_space),
    ))(input.trim())
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

fn parse_lin_srgb_color_space(input: &str) -> IResult<&str, Color> {
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

// CSS Color 4 defines separate D65-adapted (`xyz-d65`, or just `xyz`) and D5-adapted (`xyz-d50`)
// color spaces.  Currently, `pastel` does not support chromatic adaptation, and only uses the D65
// illuminant, so we only support the `xyz-d65` color space here.
fn parse_cie_xyz65_color_space(input: &str) -> IResult<&str, Color> {
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

// The "culori" library uses custom `--lab-d65` and `--lch-d65` color space names, consistent with
// the CSS Color 5 draft.  Percentage values are not supported here because the CSS `color()`
// function defines that 100% = 1.0 for all component values, so percentages would produce
// inconsistent and confusing values.  Where there is no ambiguity, however, we try to be lenient
// for broader compatibility.

fn parse_cie_lab65_color_space(input: &str) -> IResult<&str, Color> {
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

fn parse_cie_lch65_color_space(input: &str) -> IResult<&str, Color> {
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

// The "culori" library uses the CSS `color()` function with a custom `--hsv` color space name.
// The "color.js" library also uses a custom color space, but unfortunately it is incompatible
// (at least as of v0.4.5) because it uses S and V component values in the 0 to 100 range instead
// of the 0 to 1 values used by `pastel` and "culori".

fn parse_css_hsv_color_space(input: &str) -> IResult<&str, Color> {
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

pub fn parse_color(input: &str) -> Option<Color> {
    alt((
        parse_rgb_color,
        parse_hsl_color,
        parse_css_color_fn,
        parse_hsv_color,
        parse_hwb_color,
        all_consuming(parse_gray),
        parse_lab_color,
        parse_lch_color,
        parse_cmyk_color,
        all_consuming(parse_named),
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
fn parse_named_syntax() {
    assert_eq!(Some(Color::black()), parse_color("black"));
    assert_eq!(Some(Color::blue()), parse_color("blue"));
    assert_eq!(Some(Color::blue()), parse_color("Blue"));
    assert_eq!(Some(Color::blue()), parse_color("BLUE"));
    assert_eq!(Some(rgb(255, 20, 147)), parse_color("deeppink"));
    assert_eq!(None, parse_color("whatever"));
    assert_eq!(None, parse_color("red blue"));
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
