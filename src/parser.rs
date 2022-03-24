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

fn parse_percentage_or_double(input: &str) -> IResult<&str, f64> {
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

fn parse_legacy_alpha<'a>(input: &'a str) -> IResult<&'a str, f64> {
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        parse_percentage_or_double(input)
    })(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

fn parse_css_alpha<'a>(input: &'a str) -> IResult<&'a str, f64> {
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = space0(input)?;
        let (input, _) = char('/')(input)?;
        let (input, _) = space0(input)?;
        parse_percentage_or_double(input)
    })(input)?;
    Ok((input, alpha.unwrap_or(1.0)))
}

fn parse_hex(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt_hash_char(input)?;
    let (input, hex_chars) = hex_digit1(input)?;
    match hex_chars.len() {
        8 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            let a = hex_to_u8_unsafe(&hex_chars[6..8]);
            Ok((input, rgba(r, g, b, (a as f64) / 255.0)))
        }
        6 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..2]);
            let g = hex_to_u8_unsafe(&hex_chars[2..4]);
            let b = hex_to_u8_unsafe(&hex_chars[4..6]);
            Ok((input, rgb(r, g, b)))
        }
        4 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..1]);
            let g = hex_to_u8_unsafe(&hex_chars[1..2]);
            let b = hex_to_u8_unsafe(&hex_chars[2..3]);
            let a = hex_to_u8_unsafe(&hex_chars[3..4]);
            let r = r * 16 + r;
            let g = g * 16 + g;
            let b = b * 16 + b;
            let a = a * 16 + a;
            Ok((input, rgba(r, g, b, (a as f64) / 255.0)))
        }
        3 => {
            let r = hex_to_u8_unsafe(&hex_chars[0..1]);
            let g = hex_to_u8_unsafe(&hex_chars[1..2]);
            let b = hex_to_u8_unsafe(&hex_chars[2..3]);
            let r = r * 16 + r;
            let g = g * 16 + g;
            let b = b * 16 + b;
            Ok((input, rgb(r, g, b)))
        }
        _ => Err(Err::Error(nom::error::Error::new(
            "Expected hex string of 3 or 6 characters length",
            ErrorKind::Many1,
        ))),
    }
}

fn parse_numeric_rgb(input: &str) -> IResult<&str, Color> {
    let (input, prefixed) = opt(
        alt((tag_no_case("rgb("), tag_no_case("rgba(")))
    )(input)?;
    let is_prefixed = prefixed.is_some();
    let (input, _) = space0(input)?;
    let (input, r) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, g) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = parse_legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = cond(is_prefixed, char(')'))(input)?;

    let r = r / 255.0;
    let g = g / 255.0;
    let b = b / 255.0;
    let c = Color::from_rgba_float(r, g, b, alpha);

    Ok((input, c))
}

fn parse_percentage_rgb(input: &str) -> IResult<&str, Color> {
    let (input, prefixed) = opt(
        alt((tag_no_case("rgb("), tag_no_case("rgba(")))
    )(input)?;
    let is_prefixed = prefixed.is_some();
    let (input, _) = space0(input)?;
    let (input, r) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, g) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = parse_percentage(input)?;
    let (input, alpha) = parse_legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = cond(is_prefixed, char(')'))(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);

    Ok((input, c))
}

fn parse_hsl(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(
        alt((tag_no_case("hsl("), tag_no_case("hsla(")))
    )(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, s) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = parse_percentage(input)?;
    let (input, alpha) = parse_legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsla(h, s, l, alpha);

    Ok((input, c))
}

fn parse_hsv(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("hsv("), tag_no_case("hsb(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, s) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = parse_percentage(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsv(h, s, l);

    Ok((input, c))
}

fn parse_hwb(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("hwb(")(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, w) = parse_percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = parse_percentage(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hwb(h, w, b);

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

fn parse_xyz(input: &str) -> IResult<&str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("xyz(")(input)?;
    let (input, _) = space0(input)?;
    let (input, x) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, y) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, z) = double(input)?;
    let (input, alpha) = parse_legacy_alpha(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_xyza(x, y, z, alpha);

    Ok((input, c))
}

fn parse_lab<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lab(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, a) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = double(input)?;
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        double(input)
    })(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_lab(l, a, b, alpha.unwrap_or(1.0));

    Ok((input, c))
}

fn parse_lch<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lch(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = opt(char('%'))(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        double(input)
    })(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let color = Color::from_lch(l, c, h, alpha.unwrap_or(1.0));

    Ok((input, color))
}

fn parse_luv<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("luv(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, u) = double(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, v) = double(input)?;
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        double(input)
    })(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_luv(l, u, v, alpha.unwrap_or(1.0));

    Ok((input, c))
}

fn parse_lchuv<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lchuv(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = double(input)?;
    let (input, _) = opt(char('%'))(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        double(input)
    })(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let color = Color::from_lchuv(l, c, h, alpha.unwrap_or(1.0));

    Ok((input, color))
}

fn parse_hcl<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = tag_no_case("hcl(")(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = parse_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = double(input)?;
    let (input, _) = opt(char('%'))(input)?;
    let (input, alpha) = opt(|input: &'a str| {
        let (input, _) = parse_separator(input)?;
        double(input)
    })(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let color = Color::from_lchuv(l, c, h, alpha.unwrap_or(1.0));

    Ok((input, color))
}

fn parse_css_colorspace<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = tag_no_case("color(")(input)?;
    let (input, _) = space0(input)?;

    let (input, color) = alt((
        parse_xyz_colorspace,
        parse_srgb_colorspace,
    ))(input)?;

    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, color))
}

fn parse_xyz_colorspace(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("xyz")(input)?;
    let (input, _) = space1(input)?;
    let (input, x) = double(input)?;
    let (input, _) = space1(input)?;
    let (input, y) = double(input)?;
    let (input, _) = space1(input)?;
    let (input, z) = double(input)?;
    let (input, alpha) = parse_css_alpha(input)?;

    let c = Color::from_xyza(x, y, z, alpha);

    Ok((input, c))
}

fn parse_srgb_colorspace(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("srgb")(input)?;
    let (input, _) = space1(input)?;
    let (input, r) = parse_percentage_or_double(input)?;
    let (input, _) = space1(input)?;
    let (input, g) = parse_percentage_or_double(input)?;
    let (input, _) = space1(input)?;
    let (input, b) = parse_percentage_or_double(input)?;
    let (input, alpha) = parse_css_alpha(input)?;

    let c = Color::from_rgba_float(r, g, b, alpha);

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

pub fn parse_color(input: &str) -> Option<Color> {
    alt((
        all_consuming(parse_hex),
        all_consuming(parse_numeric_rgb),
        all_consuming(parse_percentage_rgb),
        all_consuming(parse_hsl),
        all_consuming(parse_hsv),
        all_consuming(parse_hwb),
        all_consuming(parse_gray),
        all_consuming(parse_xyz),
        all_consuming(parse_lab),
        all_consuming(parse_lch),
        all_consuming(parse_luv),
        all_consuming(parse_lchuv),
        all_consuming(parse_hcl),
        all_consuming(parse_css_colorspace),
        all_consuming(parse_named),
    ))(input.trim())
    .ok()
    .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgbf(r: f64, g: f64, b: f64) -> Option<Color> {
        Some(Color::from_rgb_float(r, g, b))
    }

    fn rgbf255(r: f64, g: f64, b: f64) -> Option<Color> {
        rgbf(r / 255.0, g / 255.0, b / 255.0)
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
        assert_eq!(None, parse_color("#123456789"));
        assert_eq!(None, parse_color("#hh0033"));
        assert_eq!(None, parse_color("#h03"));
    }

    #[test]
    fn parse_rgb_hex_syntax_alpha() {
        assert_eq!(Some(rgba(255, 0, 153, 0.4)), parse_color("f096"));
        assert_eq!(Some(rgba(255, 0, 153, 0.4)), parse_color("#f096"));

        assert_eq!(Some(rgba(255, 0, 153, 0.8)), parse_color("#FF0099CC"));
        assert_eq!(Some(rgba(255, 0, 153, 0.8)), parse_color("ff0099cc"));

        assert_eq!(Some(rgba(255, 0, 119, 0.6)), parse_color("  #ff007799  "));
    }

    #[test]
    fn css_rgb_legacy_fn() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255,0,153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255, 0, 153)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb( 255 , 0 , 153 )"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255, 0, 153.0)"));

        assert_eq!(
            rgbf255(255.0, 0.0, 119.085),
            parse_color("rgb(255,0,119.085)")
        );
        assert_eq!(rgbf(1.0, 0.0, 0.467), parse_color("rgb(255,0,119.085)"));

        assert_eq!(
            Some(rgb(255, 8, 119)),
            parse_color("  rgb( 255  ,  8  ,  119 )  ")
        );

        assert_eq!(rgbf(1.0, 0.0, 0.498), parse_color("rgb(100%,0%,49.8%)"));
        assert_eq!(Some(rgb(255, 0, 127)), parse_color("rgb(100%,0%,49.8039%)"));
        assert_ne!(Some(rgb(255, 0, 128)), parse_color("rgb(100%,0%,50%)"));
        assert_eq!(Some(rgb(255, 0, 128)), parse_color("rgb(100%,0%,50.196%)"));
        assert_eq!(Some(rgb(255, 0, 129)), parse_color("rgb(100%,0%,50.588%)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%,0%,60%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(100%,0%,46.6667%)"));
        assert_eq!(Some(rgb(3, 54, 119)), parse_color("rgb(1.1765%,21.1765%,46.6667%)"));
        assert_eq!(rgbf(1.0, 0.0, 0.467), parse_color("rgb(100%,0%,46.7%)"));
        assert_eq!(rgbf(0.01, 0.212, 0.467), parse_color("rgb(1%,21.2%,46.7%)"));

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%,0%,60%)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100%, 0%, 60%)"));
        assert_eq!(
            Some(rgb(255, 0, 153)),
            parse_color("rgb( 100% , 0% , 60% )")
        );

        assert_ne!(Some(rgb(100, 5, 1)), parse_color("rgb(1e2, .5e1, .5e0)"));
        assert_eq!(
            rgbf255(100.0, 5.0, 0.5),
            parse_color("rgb(1e2, .5e1, .5e0)")
        );
        assert_eq!(Some(rgb(255, 0, 0)), parse_color("rgb(255,0,0)"));
        assert_ne!(Some(rgb(255, 0, 0)), parse_color("rgb(256,0,0)"));
        assert_eq!(Some(rgb(255, 255, 0)), parse_color("rgb(100%,100%,0%)"));

        // out-of-gamut not clamped
        assert_ne!(Some(rgb(255, 255, 0)), parse_color("rgb(100%,100%,-45%)"));
        assert_eq!(rgbf(1.0, 1.0, -0.45), parse_color("rgb(100%,100%,-45%)"));

        // case-insensitive
        assert_eq!(Some(rgb(255, 8, 119)), parse_color("RGB(255, 8, 119)"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("RGB(100%, 0%, 60%)"));
    }

    #[test]
    fn css_rgb_legacy_fn_alpha() {
        assert_eq!(
            Some(rgba(255, 0, 153, 0.375)),
            parse_color("rgb(255, 0, 153, 0.375)")
        );
        assert_eq!(
            Some(rgba(255, 0, 153, 0.375)),
            parse_color("rgb(255, 0, 153, 37.5%)")
        );

        assert_eq!(
            Some(rgba(255, 0, 153, 0.5)),
            parse_color("rgb(100%, 0%, 60%, 0.5)")
        );
        assert_eq!(
            Some(rgba(255, 0, 153, 0.5)),
            parse_color("rgb(100%, 0%, 60%, 50%)")
        );
    }

    #[test]
    fn css_rgb_legacy_fn_invalid() {
        // can't mix numbers and percentages
        assert_eq!(None, parse_color("rgb(128, 80%, 255)"));
        assert_eq!(None, parse_color("rgb(100%, 0, 0)"));

        // can't mix comma and space separators
        assert_eq!(None, parse_color("rgb(128, 64 32)"));

        // not enough arguments
        assert_eq!(None, parse_color("rgb(255, 0)"));
        // missing parenthesis
        assert_eq!(None, parse_color("rgb(255, 0, 0"));
        // no separators
        assert_eq!(None, parse_color("rgb(2550119)"));
    }

    #[test]
    fn css_rgb_legacy_fn_alpha_invalid() {
        // can't use slash separator with legacy syntax
        assert_eq!(None, parse_color("rgb(128, 64, 32 / 0.5)"));
        assert_eq!(None, parse_color("rgb(75%, 50%, 25% / 80%)"));

        // can't use space separator with legacy syntax
        assert_eq!(None, parse_color("rgb(128, 64, 32 0.5)"));
        assert_eq!(None, parse_color("rgb(75%, 50%, 25% 80%)"));

        // no value after comma separator for alpha
        assert_eq!(None, parse_color("rgb(128, 64, 32, )"));
        assert_eq!(None, parse_color("rgb(75%, 50%, 25%, )"));
    }

    #[test]
    fn css_rgb_modern_fn() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153)"));

        assert_eq!(Some(rgb(3, 54, 119)), parse_color("rgb(1.1765% 21.1765% 46.6667%)"));
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("rgb(255 0 119)"));
        assert_eq!(
            Some(rgb(255, 0, 119)),
            parse_color("rgb(    255      0      119)")
        );

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(100% 0% 60%)"));

        assert_ne!(Some(rgb(140, 0, 153)), parse_color("rgb(55% 0% 60%)"));
        assert_eq!(Some(rgb(140, 0, 153)), parse_color("rgb(54.902% 0% 60%)"));
        assert_eq!(Some(rgb(142, 0, 153)), parse_color("rgb(55.6863% 0% 60%)"));

        // out-of-gamut not clamped
        assert_ne!(Some(rgb(255, 255, 0)), parse_color("rgb(100% 100% -45%)"));
        assert_eq!(rgbf(1.0, 1.0, -0.45), parse_color("rgb(100% 100% -45%)"));
    }

    #[test]
    fn css_rgb_modern_fn_alpha() {
        assert_eq!(
            Some(rgba(255, 0, 153, 0.3)),
            parse_color("rgb(255 0 153 / 30%)")
        );
        assert_eq!(
            Some(rgba(255, 0, 153, 0.3)),
            parse_color("rgb(100% 0% 60% / 0.3)")
        );
    }

    #[test]
    fn css_rgb_modern_fn_invalid() {
        // can't mix numbers and percentages
        assert_eq!(None, parse_color("rgb(128 80% 255)"));
        assert_eq!(None, parse_color("rgb(100% 0 0)"));

        // not enough arguments
        assert_eq!(None, parse_color("rgb(255 0)"));
        // missing parenthesis
        assert_eq!(None, parse_color("rgb(255 0 0"));
    }

    #[test]
    fn css_rgb_modern_fn_alpha_invalid() {
        // not enough arguments before slash
        assert_eq!(None, parse_color("rgb(255 0 / 0.3)"));
        // too many arguments before slash
        assert_eq!(None, parse_color("rgb(100% 0% 60% 30% / 0.3)"));
        // no alpha value after slash
        assert_eq!(None, parse_color("rgb(255 0 30 /)"));

        // no slash before alpha value
        assert_eq!(None, parse_color("rgb(100% 0% 60% 30%)"));
        assert_eq!(None, parse_color("rgb(192 128 64 0.3)"));

        // can't use comma separator with modern syntax
        assert_eq!(None, parse_color("rgb(100% 0% 60%, 30%)"));
        assert_eq!(None, parse_color("rgb(192 128 64, 0.3)"));
    }

    #[test]
    fn css_rgb_fn_modern_legacy_equiv() {
        // Tests that the modern syntax and the legacy syntax produce equivalent
        // results.

        assert_eq!(
            parse_color("rgb(102, 0, 153)"),
            parse_color("rgb(102 0 153)")
        );
        assert_eq!(
            parse_color("rgb(40%, 0%, 60%)"),
            parse_color("rgb(40% 0% 60%)")
        );

        assert_eq!(
            parse_color("rgb(255, 0, 153, 0.375)"),
            parse_color("rgb(255 0 153 / 0.375)")
        );
        assert_eq!(
            parse_color("rgb(255, 0, 153, 37.5%)"),
            parse_color("rgb(255 0 153 / 37.5%)")
        );

        assert_eq!(
            parse_color("rgb(100%, 0%, 60%, 0.5)"),
            parse_color("rgb(100%  0% 60% / 0.5)")
        );
        assert_eq!(
            parse_color("rgb(100%, 0%, 60%, 50%)"),
            parse_color("rgb(100% 0% 60% / 50%)")
        );
    }

    #[test]
    fn css_rgb_fn_rgba_alias() {
        // "rgba" is just a compatibility alias for "rgb", so both names
        // should produce identical results.

        assert_eq!(Some(rgb(102, 0, 153)), parse_color("rgba(102, 0, 153)"));
        assert_eq!(Some(rgb(102, 0, 153)), parse_color("rgba(102 0 153)"));
        assert_eq!(Some(rgb(102, 0, 153)), parse_color("rgba(40%, 0%, 60%)"));
        assert_eq!(Some(rgb(102, 0, 153)), parse_color("rgba(40% 0% 60%)"));

        assert_eq!(
            parse_color("rgb(255, 0, 153, 0.375)"),
            parse_color("rgba(255, 0, 153, 0.375)")
        );
        assert_eq!(
            parse_color("rgb(255, 0, 153, 37.5%)"),
            parse_color("rgba(255, 0, 153, 37.5%)")
        );

        assert_eq!(
            parse_color("rgb(100%, 0%, 60%, 0.5)"),
            parse_color("rgba(100%, 0%, 60%, 0.5)")
        );
        assert_eq!(
            parse_color("rgb(100%, 0%, 60%, 50%)"),
            parse_color("rgba(100%, 0%, 60%, 50%)")
        );

        assert_eq!(
            parse_color("rgb(100%  0% 60% / 0.5)"),
            parse_color("rgba(100%  0% 60% / 0.5)")
        );
        assert_eq!(
            parse_color("rgb(255 0 153 / 37.5%)"),
            parse_color("rgba(255 0 153 / 37.5%)")
        );
    }

    #[test]
    fn parse_rgb_standalone_syntax() {
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
    fn css_hsl_legacy_fn() {
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
            Some(Color::from_hsl(-140.0, 0.2, 0.5)),
            parse_color("hsl(-140°,20%,50%)")
        );

        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(100grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.0, 0.2, 0.5)),
            parse_color("hsl(1.5708rad,20%,50%)")
        );
        assert_ne!(
            Some(Color::from_hsl(90.05, 0.2, 0.5)),
            parse_color("hsl(1.5708rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsl(90.05, 0.2, 0.5)),
            parse_color("hsl(1.5717rad,20%,50%)")
        );
        assert_ne!(
            Some(Color::from_hsl(90.05, 0.2, 0.5)),
            parse_color("hsl(1.572rad,20%,50%)")
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

        // case-insensitive
        assert_eq!(
            Some(Color::from_hsl(280.0, 0.2, 0.5)),
            parse_color("HSL(280, 20%, 50%)")
        );
    }

    #[test]
    fn css_hsl_legacy_fn_alpha() {
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.35)),
            parse_color("hsl(280, 20%, 50%, 0.35)")
        );
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.35)),
            parse_color("hsl(280, 20%, 50%, 35%)")
        );
    }

    #[test]
    fn css_hsl_legacy_fn_invalid() {
        // lightness must be a percentage
        assert_eq!(None, parse_color("hsl(280, 20%, 50)"));
        // saturation must be a percentage
        assert_eq!(None, parse_color("hsl(280, 20, 50%)"));
        // hue is not a percentage
        assert_eq!(None, parse_color("hsl(280%, 20%, 50%)"));
        // not enough arguments
        assert_eq!(None, parse_color("hsl(280, 20%)"));
    }

    #[test]
    fn css_hsl_legacy_fn_alpha_invalid() {
        // can't use slash separator with legacy syntax
        assert_eq!(None, parse_color("hsl(280, 20%, 50% / 0.5)"));
        assert_eq!(None, parse_color("hsl(280, 20%, 50% / 80%)"));

        // can't use space separator with legacy syntax
        assert_eq!(None, parse_color("hsl(280, 20%, 50% 0.5)"));
        assert_eq!(None, parse_color("hsl(280, 20%, 50% 80%)"));

        // no value after comma separator for alpha
        assert_eq!(None, parse_color("hsl(280, 20%, 50%, )"));
    }

    #[test]
    fn css_hsl_modern_fn() {
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("hsl(270 60% 70%)")
        );
    }

    #[test]
    fn css_hsl_modern_fn_alpha() {
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.5)),
            parse_color("hsl(280 20% 50% / 0.5)")
        );
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.5)),
            parse_color("hsl(280 20% 50% / 50%)")
        );
    }

    #[test]
    fn css_hsl_modern_fn_invalid() {
        // lightness must be a percentage
        assert_eq!(None, parse_color("hsl(280 20% 50)"));
        // saturation must be a percentage
        assert_eq!(None, parse_color("hsl(280 20 50%)"));
        // hue is not a percentage
        assert_eq!(None, parse_color("hsl(280% 20% 50%)"));
        // not enough arguments
        assert_eq!(None, parse_color("hsl(280 20%)"));
    }

    #[test]
    fn css_hsl_modern_fn_alpha_invalid() {
        // not enough arguments before slash
        assert_eq!(None, parse_color("hsl(180 70% / 0.3)"));
        // too many arguments before slash
        assert_eq!(None, parse_color("hsl(180 70% 60% 30% / 0.3)"));
        // no alpha value after slash
        assert_eq!(None, parse_color("hsl(180 70% 50% /)"));

        // no slash before alpha value
        assert_eq!(None, parse_color("hsl(280 20% 50% 0.5)"));
        assert_eq!(None, parse_color("hsl(280 20% 50% 50%)"));

        // can't use comma separator with modern syntax
        assert_eq!(None, parse_color("hsl(280 20% 50%, 0.5)"));
        assert_eq!(None, parse_color("hsl(280 20% 50%, 50%)"));
    }

    #[test]
    fn css_hsl_fn_modern_legacy_equiv() {
        // Tests that the modern syntax and the legacy syntax produce equivalent
        // results.

        assert_eq!(
            parse_color("hsl(270, 60%, 70%)"),
            parse_color("hsl(270 60% 70%)")
        );

        assert_eq!(
            parse_color("hsl(280, 20%, 50%, 0.35)"),
            parse_color("hsl(280 20% 50% / 0.35)")
        );

        assert_eq!(
            parse_color("hsl(280, 20%, 50%, 35%)"),
            parse_color("hsl(280 20% 50% / 35%)")
        );
    }

    #[test]
    fn css_hsl_fn_hsla_alias() {
        // "hsla" is just a compatibility alias for "hsl", so both names
        // should produce identical results.

        assert_eq!(
            parse_color("hsl(270, 60%, 70%)"),
            parse_color("hsla(270, 60%, 70%)")
        );
        assert_eq!(
            parse_color("hsl(270 60% 70%)"),
            parse_color("hsla(270 60% 70%)")
        );

        assert_eq!(
            parse_color("hsl(280, 20%, 50%, 0.35)"),
            parse_color("hsla(280, 20%, 50%, 0.35)")
        );
        assert_eq!(
            parse_color("hsl(280, 20%, 50%, 35%)"),
            parse_color("hsla(280, 20%, 50%, 35%)")
        );

        assert_eq!(
            parse_color("hsl(280 20% 50% / 0.35)"),
            parse_color("hsla(280 20% 50% / 0.35)")
        );
        assert_eq!(
            parse_color("hsl(280 20% 50% / 35%)"),
            parse_color("hsla(280 20% 50% / 35%)")
        );
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
            Some(Color::from_hsv(90.0, 0.2, 0.5)),
            parse_color("hsv(1.5708rad,20%,50%)")
        );
        assert_ne!(
            Some(Color::from_hsv(90.05, 0.2, 0.5)),
            parse_color("hsv(1.5708rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(90.05, 0.2, 0.5)),
            parse_color("hsv(1.5717rad,20%,50%)")
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

        // case-insensitive
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("HSV(280, 20%, 50%)")
        );

        assert_eq!(None, parse_color("hsv(280,20%,50)"));
        assert_eq!(None, parse_color("hsv(280,20,50%)"));
        assert_eq!(None, parse_color("hsv(280%,20%,50%)"));
        assert_eq!(None, parse_color("hsv(280,20%)"));
    }

    #[test]
    fn parse_hsb_syntax() {
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsb(280,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsb(280deg,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsb(280°,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("hsb(280, 20%, 50%)")
        );
        assert_eq!(
            Some(Color::from_hsv(270.0, 0.6, 0.7)),
            parse_color("hsb(270 60% 70%)")
        );

        // case-insensitive
        assert_eq!(
            Some(Color::from_hsv(280.0, 0.2, 0.5)),
            parse_color("HSB(280, 20%, 50%)")
        );

        assert_eq!(None, parse_color("hsb(280,20%)"));
    }

    #[test]
    fn parse_hwb_syntax() {
        assert_eq!(
            Some(Color::from_hwb(280.0, 0.2, 0.5)),
            parse_color("hwb(280,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(280.0, 0.2, 0.5)),
            parse_color("hwb(280deg,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(280.0, 0.2, 0.5)),
            parse_color("hwb(280°,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(280.33, 0.123, 0.456)),
            parse_color("hwb(280.33001,12.3%,45.6%)")
        );
        assert_eq!(
            Some(Color::from_hwb(280.0, 0.2, 0.5)),
            parse_color("hwb(  280 , 20% , 50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(270.0, 0.6, 0.7)),
            parse_color("hwb(270 60% 70%)")
        );

        assert_eq!(
            Some(Color::from_hwb(-140.0, 0.2, 0.5)),
            parse_color("hwb(-140°,20%,50%)")
        );

        assert_eq!(
            Some(Color::from_hwb(90.0, 0.2, 0.5)),
            parse_color("hwb(100grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(90.0, 0.2, 0.5)),
            parse_color("hwb(1.5708rad,20%,50%)")
        );
        assert_ne!(
            Some(Color::from_hwb(90.05, 0.2, 0.5)),
            parse_color("hwb(1.5708rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(90.05, 0.2, 0.5)),
            parse_color("hwb(1.5717rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(90.0, 0.2, 0.5)),
            parse_color("hwb(0.25turn,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(45.0, 0.2, 0.5)),
            parse_color("hwb(50grad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(45.0, 0.2, 0.5)),
            parse_color("hwb(0.7854rad,20%,50%)")
        );
        assert_eq!(
            Some(Color::from_hwb(45.0, 0.2, 0.5)),
            parse_color("hwb(0.125turn,20%,50%)")
        );

        // case-insensitive
        assert_eq!(
            Some(Color::from_hwb(280.0, 0.2, 0.5)),
            parse_color("HWB(280, 20%, 50%)")
        );

        assert_eq!(None, parse_color("hwb(280,20%,50)"));
        assert_eq!(None, parse_color("hwb(280,20,50%)"));
        assert_eq!(None, parse_color("hwb(280%,20%,50%)"));
        assert_eq!(None, parse_color("hwb(280,20%)"));
    }

    #[test]
    fn parse_gray_syntax() {
        assert_eq!(Some(Color::graytone(0.2)), parse_color("gray(0.2)"));
        assert_eq!(Some(Color::black()), parse_color("gray(0.0)"));
        assert_eq!(Some(Color::black()), parse_color("gray(0)"));
        assert_eq!(Some(Color::white()), parse_color("gray(1.0)"));
        assert_eq!(Some(Color::white()), parse_color("gray(1)"));

        // out-of-gamut not clamped
        assert_ne!(Some(Color::white()), parse_color("gray(7.3)"));
        assert_eq!(
            Some(Color::from_rgb_float(7.3, 7.3, 7.3)),
            parse_color("gray(7.3)")
        );

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
    fn parse_xyz_syntax() {
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3, 0.5, 0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3,0.5,0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3, 0.5, 0.7, 1.0)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3,0.5,0.7,1)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("xyz(0.3   0.5    0.7    1.0)")
        );

        assert_eq!(
            Some(Color::from_xyz(0.950_470, 1.0, 1.088_830)),
            parse_color("ciexyz(0.950470, 1.0, 1.088830)")
        );

        assert_eq!(
            Some(Color::from_xyz(-0.004, 1.007, -1.2222)),
            parse_color("ciexyz(-0.004, 1.007000, -1.2222)")
        );
    }

    #[test]
    fn parse_xyz_alpha() {
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.2)),
            parse_color("xyz(0.3, 0.5, 0.7, 0.2)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.2)),
            parse_color("xyz(0.3,0.5,0.7,0.2)")
        );

        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.65)),
            parse_color("xyz(0.3, 0.5, 0.7, 65%)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.65)),
            parse_color("xyz(0.3,0.5,0.7,65%)")
        );

        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.2)),
            parse_color("xyz(0.3 0.5 0.7 0.2)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.65)),
            parse_color("xyz(0.3 0.5 0.7 65%)")
        );
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
            Some(Color::from_lch(60.0, 50.0, 280.0, 1.0)),
            parse_color("lch(60,50,280)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 50.0, 280.0, 1.0)),
            parse_color("lch(60,50,280deg)")
        );
        assert_eq!(
            Some(Color::from_lch(23.3, 45.6, 280.33, 1.0)),
            parse_color("lch(23.3,45.6,280.33)")
        );
        assert_eq!(
            Some(Color::from_lch(23.3, 45.6, 280.33, 1.0)),
            parse_color("lch(23.3,45.6,280.3301)")
        );
        assert_ne!(
            Some(Color::from_lch(23.3, 45.6, 280.33, 1.0)),
            parse_color("lch(23.3,45.6,280.3)")
        );
        assert_eq!(
            Some(Color::from_lch(23.3, 45.6, 280.33, 1.0)),
            parse_color("lch(23.301,45.6,280.33)")
        );
        assert_ne!(
            Some(Color::from_lch(23.3, 45.6, 280.33, 1.0)),
            parse_color("lch(23.31,45.6,280.33)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 50.0, 280.0, 0.5)),
            parse_color("lch(60,50,280,0.5)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 50.0, 280.0, 1.0)),
            parse_color("lch( 60 , 50,  280 )")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 50.0, 270.0, 1.0)),
            parse_color("lch(60 50 270)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 50.0, 270.0, 1.0)),
            parse_color("lch(60% 50 270)")
        );

        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, -140.0, 1.0)),
            parse_color("lch(50,30,-140°)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 220.0, 1.0)),
            parse_color("lch(50,30,-140)")
        );

        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 90.0, 1.0)),
            parse_color("lch(50,30,100grad)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 90.05, 1.0)),
            parse_color("lch(50,30,1.5717rad)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 90.0, 1.0)),
            parse_color("lch(50,30,0.25turn)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 45.0, 1.0)),
            parse_color("lch(50,30,50grad)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 45.0, 1.0)),
            parse_color("lch(50,30,0.7854rad)")
        );
        assert_eq!(
            Some(Color::from_lch(50.0, 30.0, 45.0, 1.0)),
            parse_color("lch(50,30,0.125turn)")
        );

        assert_eq!(None, parse_color("lch(50,30%,280)"));
        assert_eq!(None, parse_color("lch(50,-30,280)"));
        assert_eq!(None, parse_color("lch(50,30,280%)"));
        assert_eq!(None, parse_color("lch(50,30)"));

        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 1.0)),
            parse_color("LCh(60,40,150)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 1.0)),
            parse_color("CIELCh(60,40,150)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 1.0)),
            parse_color("CIELch(60,40,150)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 0.4)),
            parse_color("cieLch(60,40,150,0.4)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 1.0)),
            parse_color("LCh(        60,  40,150   )")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 0.4)),
            parse_color("CieLch(60,40,150,0.4)")
        );
        assert_eq!(
            Some(Color::from_lch(60.0, 40.0, 150.0, 1.0)),
            parse_color("CIELCh(        60,  40,150   )")
        );
    }

    #[test]
    fn parse_luv_syntax() {
        assert_eq!(
            Some(Color::from_luv(12.43, -35.5, 43.4, 1.0)),
            parse_color("Luv(12.43,-35.5,43.4)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, -23.0, 43.0, 0.5)),
            parse_color("luv(15,-23,43,0.5)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELuv(15,23,-43)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, 35.5, -43.4, 1.0)),
            parse_color("CIELuv(15,35.5,-43.4)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, -35.5, -43.4, 0.4)),
            parse_color("cieLuv(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, 23.0, -43.0, 1.0)),
            parse_color("Luv(        15,  23,-43   )")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, -35.5, -43.4, 0.4)),
            parse_color("CieLuv(15,-35.5,-43.4,0.4)")
        );
        assert_eq!(
            Some(Color::from_luv(15.0, 23.0, -43.0, 1.0)),
            parse_color("CIELuv(        15,  23,-43   )")
        );
    }

    #[test]
    fn parse_lchuv_syntax() {
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 1.0)),
            parse_color("lchuv(60,50,280)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 1.0)),
            parse_color("lchuv(60,50,280deg)")
        );
        assert_eq!(
            Some(Color::from_lchuv(23.3, 45.6, 280.33, 1.0)),
            parse_color("lchuv(23.3,45.6,280.33001)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 0.5)),
            parse_color("lchuv(60,50,280,0.5)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 270.0, 1.0)),
            parse_color("lchuv(60 50 270)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 270.0, 1.0)),
            parse_color("lchuv(60% 50 270)")
        );

        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, -140.0, 1.0)),
            parse_color("lchuv(50,30,-140°)")
        );
        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, 220.0, 1.0)),
            parse_color("lchuv(50,30,-140)")
        );

        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, 90.0, 1.0)),
            parse_color("lchuv(50,30,100grad)")
        );
        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, 90.0, 1.0)),
            parse_color("lchuv(50,30,1.5708rad)")
        );
        assert_ne!(
            Some(Color::from_lchuv(50.0, 30.0, 90.05, 1.0)),
            parse_color("lchuv(50,30,1.5708rad)")
        );
        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, 90.05, 1.0)),
            parse_color("lchuv(50,30,1.5717rad)")
        );
        assert_eq!(
            Some(Color::from_lchuv(50.0, 30.0, 45.0, 1.0)),
            parse_color("lchuv(50,30,0.125turn)")
        );

        assert_eq!(None, parse_color("lchuv(50,30%,280)"));
        assert_eq!(None, parse_color("lchuv(50,-30,280)"));
        assert_eq!(None, parse_color("lchuv(50,30,280%)"));
        assert_eq!(None, parse_color("lchuv(50,30)"));

        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("LChuv(60,40,150)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("CIELChuv(60,40,150)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 0.4)),
            parse_color("cielchuv(60,40,150,0.4)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("LChuv(        60,  40,150   )")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("CIELChuv(        60,  40,150   )")
        );
    }

    #[test]
    fn parse_hcl_syntax() {
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 1.0)),
            parse_color("hcl(280,50,60)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 1.0)),
            parse_color("hcl(280deg,50,60)")
        );
        assert_eq!(
            Some(Color::from_lchuv(23.3, 45.6, 280.4, 1.0)),
            parse_color("hcl(280.4,45.6,23.3)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 280.0, 0.5)),
            parse_color("hcl(280,50,60,0.5)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 270.0, 1.0)),
            parse_color("hcl(270 50 60)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 50.0, 270.0, 1.0)),
            parse_color("hcl(270 50 60%)")
        );

        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("HCL(150,40,60)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("hCL(150,40,60)")
        );
        assert_eq!(
            Some(Color::from_lchuv(60.0, 40.0, 150.0, 1.0)),
            parse_color("HCL(        150,  40,60   )")
        );
    }

    #[test]
    fn css_color_fn_xyz() {
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz 0.3 0.5 0.7)")
        );

        assert_eq!(
            Some(Color::from_xyz(0.950_470, 1.0, 1.088_830)),
            parse_color("color(xyz 0.950470 1 1.088830)")
        );

        assert_eq!(
            Some(Color::from_xyz(-0.004, 1.007, -1.2222)),
            parse_color("color(xyz -0.004 1.007000 -1.2222)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz  0.3   0.5    0.7)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(XYZ 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(Xyz 0.3 0.5 0.7)")
        );

        // alpha is supported
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7 / 0.9)")
        );

        // not enough parameters
        assert_eq!(None, parse_color("color(xyz 0.3 0.5)"));
        // too many parameters
        assert_eq!(None, parse_color("color(xyz 0.3 0.5 0.7 1.0)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(xyz 0.3, 0.5, 0.7)"));
        // percentages are disallowed
        assert_eq!(None, parse_color("color(xyz 30% 50% 70%)"));
    }

    #[test]
    fn css_color_fn_srgb() {
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(srgb 0.3 0.5 0.7)")
        );

        // percentages are supported
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(srgb 30% 50% 70%)")
        );

        // numbers and percentages can be mixed
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(srgb 30% 0.5 70%)")
        );
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(srgb 0.3 50% 0.7)")
        );

        // extra spaces are allowed
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(srgb  0.3   0.5    0.7)")
        );

        // color space name is case-insensitive
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(sRGB 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 0.5, 0.7)),
            parse_color("color(Srgb 0.3 0.5 0.7)")
        );

        // alpha is supported
        assert_eq!(
            Some(Color::from_rgba_float(0.3, 0.5, 0.7, 0.1)),
            parse_color("color(srgb 0.3 0.5 0.7 / 0.1)")
        );

        // out-of-gamut values are allowed
        assert_eq!(
            Some(Color::from_rgb_float(0.3, 1.2, 0.7)),
            parse_color("color(srgb 0.3 1.2 0.7)")
        );
        assert_eq!(
            Some(Color::from_rgb_float(-0.1, 0.5, 0.7)),
            parse_color("color(srgb -0.1 0.5 0.7)")
        );

        // not enough parameters
        assert_eq!(None, parse_color("color(srgb 0.3 0.5)"));
        // too many parameters
        assert_eq!(None, parse_color("color(srgb 0.3 0.5 0.7 1.0)"));
        // comma separators not allowed
        assert_eq!(None, parse_color("color(srgb 0.3, 0.5, 0.7)"));
    }

    #[test]
    fn css_color_fn_alpha() {
        // Tests alternative `color` function alpha formats that apply to any
        // valid color space.  Alpha support should be tested for each color
        // space, but this saves having to test all the alternative cases in
        // multiple places.

        // spaces optional around alpha separator
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7 /0.9)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7/ 0.9)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7/0.9)")
        );

        // extra spaces allowed around alpha separator
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7    /  0.9)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7/      0.9)")
        );
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7     /0.9)")
        );

        // alpha value can be a percentage
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.9)),
            parse_color("color(xyz 0.3 0.5 0.7 / 90%)")
        );

        // explicit unit (opaque) alpha value is allowed
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz 0.3 0.5 0.7 / 1)")
        );

        // explicit zero (transparent) alpha value is allowed
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.0)),
            parse_color("color(xyz 0.3 0.5 0.7 / 0)")
        );

        // alpha separator with no value is invalid
        assert_eq!(None, parse_color("color(xyz 0.3 0.5 0.7 /)"));
    }

    #[test]
    fn css_color_fn_whitespace() {
        // Tests optional whitespace in the `color` function arguments
        // (applies to any valid color space).

        // spaces are allowed before color space name
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(  xyz 0.3 0.5 0.7)")
        );

        // spaces are allowed after last channel value
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("color(xyz 0.3 0.5 0.7  )")
        );

        // spaces are allowed after alpha value
        assert_eq!(
            Some(Color::from_xyza(0.3, 0.5, 0.7, 0.1)),
            parse_color("color(xyz 0.3 0.5 0.7 / 0.1  )")
        );
    }

    #[test]
    fn css_color_fn_case_insensitive() {
        // Tests case-insensitivity of the `color` function name.
        // (note: color space names should be tested separately)
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("Color(xyz 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("COLOR(xyz 0.3 0.5 0.7)")
        );
        assert_eq!(
            Some(Color::from_xyz(0.3, 0.5, 0.7)),
            parse_color("cOLOr(xyz 0.3 0.5 0.7)")
        );
    }

    #[test]
    fn css_color_fn_invalid() {
        // undefined color space
        assert_eq!(
            None,
            parse_color("color(qqqq 0.1 0.2 0.3 0.4)")
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
}