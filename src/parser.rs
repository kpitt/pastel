use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::ErrorKind;
use nom::number::complete::double;
use nom::Err;
use nom::IResult;

use crate::named::NAMED_COLORS;
use crate::Color;

fn hex_to_u8_unsafe(num: &str) -> u8 {
    u8::from_str_radix(num, 16).unwrap()
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

fn rgba(r: u8, g: u8, b: u8, a: f64) -> Color {
    Color::from_rgba(r, g, b, a)
}

fn comma_separated(input: &str) -> IResult<&str, &str> {
    let (input, _) = space0(input)?;
    let (input, _) = char(',')(input)?;
    space0(input)
}

fn parse_separator(input: &str) -> IResult<&str, &str> {
    alt((comma_separated, space1))(input)
}

fn opt_hash_char(s: &str) -> IResult<&str, Option<char>> {
    opt(char('#'))(s)
}

fn parse_percentage(input: &str) -> IResult<&str, f64> {
    let (input, percent) = double(input)?;
    let (input, _) = char('%')(input)?;
    Ok((input, percent / 100.))
}

fn parse_number_or_percentage(input: &str) -> IResult<&str, f64> {
    let (input, value) = alt((parse_percentage, double))(input)?;
    Ok((input, value))
}

fn parse_degrees(input: &str) -> IResult<&str, f64> {
    let (input, d) = double(input)?;
    let (input, _) = alt((tag("°"), tag("deg"), tag("")))(input)?;
    Ok((input, d))
}

fn parse_rads(input: &str) -> IResult<&str, f64> {
    let (input, rads) = double(input)?;
    let (input, _) = tag("rad")(input)?;
    Ok((input, rads * 180. / std::f64::consts::PI))
}

fn parse_grads(input: &str) -> IResult<&str, f64> {
    let (input, grads) = double(input)?;
    let (input, _) = tag("grad")(input)?;
    Ok((input, grads * 360. / 400.))
}

fn parse_turns(input: &str) -> IResult<&str, f64> {
    let (input, turns) = double(input)?;
    let (input, _) = tag("turn")(input)?;
    Ok((input, turns * 360.))
}

fn parse_angle(input: &str) -> IResult<&str, f64> {
    alt((parse_turns, parse_grads, parse_rads, parse_degrees))(input)
}

fn parse_alpha<'a>(input: &'a str) -> IResult<&'a str, f64> {
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        alt((parse_percentage, double))(input)
    })(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

fn parse_css_alpha<'a>(input: &'a str) -> IResult<&'a str, f64> {
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = space0(input)?;
        let (input, _) = char('/')(input)?;
        let (input, _) = space0(input)?;
        alt((parse_percentage, double))(input)
    })(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

fn parse_hex(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt_hash_char(input)?;
    let (input, hex_chars) = hex_digit1(input)?;
    match hex_chars.len() {
        // RRGGBB
        6 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            Ok((input, rgb(r, g, b)))
        }
        // RGB
        3 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..1]);
            let g = hex_to_u8_unsafe(&hex_chars[1..2]);
            let b = hex_to_u8_unsafe(&hex_chars[2..3]);
            let r = r * 16 + r;
            let g = g * 16 + g;
            let b = b * 16 + b;
            Ok((input, rgb(r, g, b)))
        }
        // RRGGBBAA
        8 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            let a = hex_to_u8_unsafe(&hex_chars[6..8]) as f64 / 255.0;
            Ok((input, rgba(r, g, b, a)))
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
            Ok((input, rgba(r, g, b, a)))
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
    let (input, _) = parse_separator(input)?;
    let (input, g) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = parse_alpha(input)?;
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
    let (input, alpha) = parse_css_alpha(input)?;
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
    let (input, r) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, g) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = parse_percentage(input)?;
    let (input, alpha) = parse_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = cond(is_prefixed, char(')'))(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);

    Ok((input, c))
}

fn parse_css_percentage_rgb(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("rgb("), tag_no_case("rgba(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, r) = parse_percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, g) = parse_percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, b) = parse_percentage(input)?;
    let (input, alpha) = parse_css_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);

    Ok((input, c))
}

fn parse_hsl(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag("hsl("), tag("hsla(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, s) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = parse_percentage(input)?;
    let (input, alpha) = parse_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsla(h, s, l, alpha);

    Ok((input, c))
}

fn parse_css_hsl(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("hsl("), tag_no_case("hsla(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = space1(input)?;
    let (input, s) = parse_percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, l) = parse_percentage(input)?;
    let (input, alpha) = parse_css_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsla(h, s, l, alpha);

    Ok((input, c))
}

fn parse_hsv(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag("hsv("), tag("hsva(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, s) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, v) = parse_percentage(input)?;
    let (input, alpha) = parse_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsva(h, s, v, alpha);

    Ok((input, c))
}

fn parse_gray(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag("gray(")(input)?;
    let (input, _) = space0(input)?;
    let (input, g) = verify(alt((parse_percentage, double)), |&d| d >= 0.)(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_rgb_float(g, g, g);

    Ok((input, c))
}

fn parse_lab(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lab(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, a) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = parse_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lab(l, a, b, alpha);

    Ok((input, c))
}

fn parse_lch(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lch(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, alpha) = parse_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lch(l, c, h, alpha);

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

fn parse_css_color_fn<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = tag_no_case("color(")(input)?;

    let (input, color) = parse_cie_xyz65_color_space(input)?;

    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, color))
}

// CSS Color 4 defines separate D65-adapted (`xyz-d65`, or just `xyz`) and D5-adapted (`xyz-d50`)
// color spaces.  Currently, `pastel` does not support chromatic adaptation, and only uses the D65
// illuminant, so we only support the `xyz-d65` color space here.
fn parse_cie_xyz65_color_space<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = alt((tag_no_case("xyz-d65"), tag_no_case("xyz")))(input)?;
    let (input, _) = space1(input)?;
    let (input, x) = parse_number_or_percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, y) = parse_number_or_percentage(input)?;
    let (input, _) = space1(input)?;
    let (input, z) = parse_number_or_percentage(input)?;
    let (input, alpha) = parse_css_alpha(input)?;
    let (input, _) = space0(input)?;

    let c = Color::from_xyz(x, y, z, alpha);

    Ok((input, c))
}

pub fn parse_color(input: &str) -> Option<Color> {
    alt((
        all_consuming(parse_hex),
        all_consuming(parse_css_numeric_rgb),
        all_consuming(parse_css_percentage_rgb),
        all_consuming(parse_numeric_rgb),
        all_consuming(parse_percentage_rgb),
        all_consuming(parse_css_hsl),
        all_consuming(parse_hsl),
        all_consuming(parse_css_color_fn),
        all_consuming(parse_hsv),
        all_consuming(parse_gray),
        all_consuming(parse_lab),
        all_consuming(parse_lch),
        all_consuming(parse_named),
    ))(input.trim())
    .ok()
    .map(|(_, c)| c)
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
fn parse_rgb_standalone_syntax() {
    assert_eq!(
        Some(rgb(255, 8, 119)),
        parse_color("  rgb( 255  ,  8  ,  119 )  ")
    );

    assert_eq!(rgb(255, 0, 153), parse_color("255,0,153").unwrap());
    assert_eq!(rgb(255, 0, 153), parse_color("255, 0, 153").unwrap());
    assert_eq!(
        rgb(255, 0, 153),
        parse_color("  255  ,  0  ,  153   ").unwrap()
    );
    assert_eq!(rgb(255, 0, 153), parse_color("255 0 153").unwrap());
    assert_eq!(rgb(255, 0, 153), parse_color("255 0 153.0").unwrap());

    assert_eq!(Some(rgb(1, 2, 3)), parse_color("1,2,3"));
}

#[test]
fn parse_hsl_syntax() {
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

    assert_eq!(None, parse_color("hsl(280,20%,50)"));
    assert_eq!(None, parse_color("hsl(280,20,50%)"));
    assert_eq!(None, parse_color("hsl(280%,20%,50%)"));
    assert_eq!(None, parse_color("hsl(280,20%)"));
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
fn parse_hsv_syntax() {
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
fn parse_alpha_syntax() {
    // hex
    assert_eq!(Some(rgba(255, 0, 0, 1.0)), parse_color("ff0000ff"));
    assert_eq!(Some(rgba(255, 0, 0, 1.0)), parse_color("#ff0000ff"));

    // rgb/rgba
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgb(10,0,0,1)"));
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgb(10,0,0, 1)"));
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0,1)"));
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0, 1)"));
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0,1.0)"));
    assert_eq!(Some(rgba(10, 0, 0, 1.0)), parse_color("rgba(10,0,0, 1.0)"));

    // hsl/hsla
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

    // lab
    assert_eq!(
        Some(Color::from_lab(10.0, 30.0, 50.0, 1.0)),
        parse_color("lab(10,30,50,1)")
    );
    assert_eq!(
        Some(Color::from_lab(10.0, 30.0, 50.0, 1.0)),
        parse_color("lab(10,30,50,1.0)")
    );

    // alpha parsing
    assert_eq!(Some(rgba(10, 0, 0, 0.5)), parse_color("rgba(10,0,0,0.5)"));
    assert_eq!(Some(rgba(10, 0, 0, 0.5)), parse_color("rgba(10,0,0,50%)"));
    assert_eq!(Some(rgba(10, 0, 0, 0.33)), parse_color("rgba(10,0,0,0.33)"));
    assert_eq!(Some(rgba(10, 0, 0, 0.33)), parse_color("rgba(10,0,0,33%)"));

    // hex alpha doesn't line up nicely with decimal precision,
    // so just compare the debug output (3 digit precision)
    assert_eq!(
        format!("{:?}", Some(rgba(10, 0, 0, 0.502))),
        format!("{:?}", parse_color("0a000080"))
    );
    assert_eq!(
        format!("{:?}", Some(rgba(10, 0, 0, 0.329))),
        format!("{:?}", parse_color("0a000054"))
    );
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
