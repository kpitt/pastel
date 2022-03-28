use nom::{
    Err,
    IResult,
    branch::alt,
    bytes::complete::*,
    character::complete::*,
    combinator::*,
    error::{ErrorKind, ParseError},
    number::complete::double,
    sequence::*
};
use std::f64::consts::PI;

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

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading
/// and trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn comma_separator(input: &str) -> IResult<&str, &str> {
    recognize(delimited(space0, char(','), space0))(input)
}

fn parse_separator(input: &str) -> IResult<&str, &str> {
    alt((comma_separator, space1))(input)
}

fn opt_hash_char(s: &str) -> IResult<&str, Option<char>> {
    opt(char('#'))(s)
}

fn percentage(input: &str) -> IResult<&str, f64> {
    map(terminated(double, char('%')), |p| p / 100.)(input)
}

fn percentage_or_double(input: &str) -> IResult<&str, f64> {
    alt((percentage, double))(input)
}

fn number255(input: &str) -> IResult<&str, f64> {
    // Parses a double that is scaled from [0, 255] down to [0, 1].
    map(double, |n| n / 255.0)(input)
}

fn percentage_or_number255(input: &str) -> IResult<&str, f64> {
    alt((percentage, number255))(input)
}

fn parse_degrees(input: &str) -> IResult<&str, f64> {
    terminated(
        double,
        opt(alt((tag("°"), tag_no_case("deg"))))
    )(input)
}

fn parse_rads(input: &str) -> IResult<&str, f64> {
    map(
        terminated(double, tag_no_case("rad")),
        |rads| rads * 180. / PI
    )(input)
}

fn parse_grads(input: &str) -> IResult<&str, f64> {
    map(
        terminated(double, alt((tag_no_case("grad"), tag_no_case("grd")))),
        |grads| grads * 360. / 400.
    )(input)
}

fn parse_turns(input: &str) -> IResult<&str, f64> {
    map(
        terminated(double, alt((tag_no_case("turn"), tag_no_case("trn")))),
        |turns| turns * 360.
    )(input)
}

fn hue_angle(input: &str) -> IResult<&str, f64> {
    alt((parse_turns, parse_grads, parse_rads, parse_degrees))(input)
}

fn legacy_alpha(input: &str) -> IResult<&str, f64> {
    map(opt(preceded(comma_separator, percentage_or_double)), |a| {
        a.unwrap_or(1.0)
    })(input)
}

fn css_alpha_separator(input: &str) -> IResult<&str, &str> {
    recognize(delimited(space0, char('/'), space0))(input)
}

fn strict_css_alpha(input: &str) -> IResult<&str, f64> {
    map(
        opt(preceded(css_alpha_separator, percentage_or_double)),
        |a| a.unwrap_or(1.0),
    )(input)
}

fn lenient_css_alpha(input: &str) -> IResult<&str, f64> {
    map(
        opt(preceded(
            alt((css_alpha_separator, space1)),
            percentage_or_double,
        )),
        |a| a.unwrap_or(1.0),
    )(input)
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

fn fn_arguments<'a, F: 'a, O, E: ParseError<&'a str>>(
    args: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(char('('), ws(args), char(')'))
}

fn parse_rgb(input: &str) -> IResult<&str, Color> {
    // must list alternatives from most to least specific
    preceded(
        alt((tag_no_case("rgba"), tag_no_case("rgb"))),
        fn_arguments(rgb_arguments),
    )(input)
}

fn rgb_arguments(input: &str) -> IResult<&str, Color> {
    let (input, (r, g, b, a)) = alt((
        rgb_modern_arguments,
        rgb_legacy_arguments,
    ))(input)?;

    let c = Color::from_rgba_float(r, g, b, a);
    Ok((input, c))
}

fn rgb_modern_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        percentage_or_number255,
        preceded(space1, percentage_or_number255),
        preceded(space1, percentage_or_number255),
        lenient_css_alpha,
    ))(input)
}

fn rgb_legacy_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        percentage_or_number255,
        preceded(comma_separator, percentage_or_number255),
        preceded(comma_separator, percentage_or_number255),
        legacy_alpha,
    ))(input)
}

fn parse_hsl(input: &str) -> IResult<&str, Color> {
    // must list alternatives from most to least specific
    preceded(
        alt((tag_no_case("hsla"), tag_no_case("hsl"))),
        fn_arguments(hsl_arguments),
    )(input)
}

fn hsl_arguments(input: &str) -> IResult<&str, Color> {
    let (input, (h, s, l, a)) = alt((
        hsl_modern_arguments,
        hsl_legacy_arguments,
    ))(input)?;

    let c = Color::from_hsla(h, s, l, a);
    Ok((input, c))
}

fn hsl_modern_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        hue_angle,
        preceded(space1, percentage),
        preceded(space1, percentage),
        lenient_css_alpha,
    ))(input)
}

fn hsl_legacy_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        hue_angle,
        preceded(comma_separator, percentage),
        preceded(comma_separator, percentage),
        legacy_alpha,
    ))(input)
}

fn parse_hsv(input: &str) -> IResult<&str, Color> {
    let (input, _) = alt((tag_no_case("hsv("), tag_no_case("hsb(")))(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, s) = percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = percentage(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hsv(h, s, l);

    Ok((input, c))
}

fn parse_hwb(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("hwb(")(input)?;
    let (input, _) = space0(input)?;
    let (input, h) = hue_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, w) = percentage(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, b) = percentage(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(')')(input)?;

    let c = Color::from_hwb(h, w, b);

    Ok((input, c))
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

fn parse_xyz(input: &str) -> IResult<&str, Color> {
    preceded(
        preceded(opt(tag_no_case("cie")), tag_no_case("xyz")),
        fn_arguments(xyz_arguments),
    )(input)
}

fn xyz_arguments(input: &str) -> IResult<&str, Color> {
    let (input, (x, y, z, a)) = alt((
        xyz_modern_arguments,
        xyz_legacy_arguments,
    ))(input)?;

    let c = Color::from_xyza(x, y, z, a);
    Ok((input, c))
}

fn xyz_modern_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        double,
        preceded(space1, double),
        preceded(space1, double),
        lenient_css_alpha,
    ))(input)
}

fn xyz_legacy_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        double,
        preceded(comma_separator, double),
        preceded(comma_separator, double),
        legacy_alpha,
    ))(input)
}

fn parse_lab(input: &str) -> IResult<&str, Color> {
    preceded(
        preceded(opt(tag_no_case("cie")), tag_no_case("lab")),
        fn_arguments(lab_arguments),
    )(input)
}

fn lab_arguments(input: &str) -> IResult<&str, Color> {
    let (input, (l, a, b, alpha)) = alt((
        lab_modern_arguments,
        lab_legacy_arguments,
    ))(input)?;

    let c = Color::from_lab(l, a, b, alpha);
    Ok((input, c))
}

fn lab_modern_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        lab_lightness,
        preceded(space1, double),
        preceded(space1, double),
        lenient_css_alpha,
    ))(input)
}

fn lab_legacy_arguments(input: &str) -> IResult<&str, (f64, f64, f64, f64)> {
    tuple((
        lab_lightness,
        preceded(comma_separator, double),
        preceded(comma_separator, double),
        legacy_alpha,
    ))(input)
}

fn lab_lightness(input: &str) -> IResult<&str, f64> {
    terminated(double, opt(char('%')))(input)
}

fn parse_lch<'a>(input: &'a str) -> IResult<&'a str, Color> {
    let (input, _) = opt(tag_no_case("cie"))(input)?;
    let (input, _) = tag_no_case("lch(")(input)?;
    let (input, _) = space0(input)?;
    let (input, l) = lab_lightness(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, h) = hue_angle(input)?;
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
    let (input, l) = lab_lightness(input)?;
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
    let (input, l) = lab_lightness(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, h) = hue_angle(input)?;
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
    let (input, h) = hue_angle(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, c) = verify(double, |&d| d >= 0.)(input)?;
    let (input, _) = parse_separator(input)?;
    let (input, l) = lab_lightness(input)?;
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
    let (input, alpha) = strict_css_alpha(input)?;

    let c = Color::from_xyza(x, y, z, alpha);

    Ok((input, c))
}

fn parse_srgb_colorspace(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag_no_case("srgb")(input)?;
    let (input, _) = space1(input)?;
    let (input, r) = percentage_or_double(input)?;
    let (input, _) = space1(input)?;
    let (input, g) = percentage_or_double(input)?;
    let (input, _) = space1(input)?;
    let (input, b) = percentage_or_double(input)?;
    let (input, alpha) = strict_css_alpha(input)?;

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
        all_consuming(parse_rgb),
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
        // Most supported formats have clear markers that the parser can detect.
        // If none of these match, trying to pull out a standalone list of sRGB
        // arguments is a last-ditch effort.
        all_consuming(rgb_arguments),
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

    fn rgbaf(r: f64, g: f64, b: f64, a: f64) -> Option<Color> {
        Some(Color::from_rgba_float(r, g, b, a))
    }

    fn rgbf255(r: f64, g: f64, b: f64) -> Option<Color> {
        rgbf(r / 255.0, g / 255.0, b / 255.0)
    }

    fn rgbaf255(r: f64, g: f64, b: f64, a: f64) -> Option<Color> {
        rgbaf(r / 255.0, g / 255.0, b / 255.0, a)
    }

    #[test]
    fn css_rgb_hex() {
        // 6 digits -> 2 digits each for R, G, B
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#ff0099"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#FF0099"));
        assert_eq!(Some(rgb(87, 166, 206)), parse_color("#57A6CE"));

        // 3 digits -> 1 digit each for R, G, B
        // 3-digit RGB is equivalent to 6-digit RRGGBB
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#f09"));
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("#F09"));

        // leading and trailing whitespace is ignored
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("  #ff0077  "));
    }

    #[test]
    fn css_rgb_hex_alpha() {
        // 8 digits -> 2 digits each for R, G, B, A
        assert_eq!(Some(rgba(255, 0, 153, 0.8)), parse_color("#FF0099CC"));
        assert_eq!(Some(rgba(255, 0, 153, 0.8)), parse_color("#ff0099cc"));

        // 4 digits -> 1 digit each for R, G, B, A
        // 4-digit RGBA is equivalent to 8-digit RRGGBBAA
        assert_eq!(Some(rgba(255, 0, 153, 0.4)), parse_color("#f096"));

        // leading and trailing whitespace is ignored
        assert_eq!(Some(rgba(255, 0, 119, 0.6)), parse_color("  #ff007799  "));
    }

    #[test]
    fn rgb_hex_lenient() {
        // Tests that hex values are also accepted without a leading hash char.

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("ff0099"));
        assert_eq!(Some(rgb(87, 166, 206)), parse_color("57A6CE"));

        assert_eq!(Some(rgb(255, 0, 153)), parse_color("f09"));
        assert_eq!(Some(rgb(170, 187, 204)), parse_color("ABC"));

        assert_eq!(Some(rgba(255, 0, 153, 0.8)), parse_color("ff0099cc"));
        assert_eq!(Some(rgba(255, 0, 153, 0.4)), parse_color("f096"));

        // leading and trailing whitespace is ignored
        assert_eq!(Some(rgb(255, 0, 119)), parse_color("   ff0077  "));
    }

    #[test]
    fn rgb_hex_invalid() {
        // not a recognized number of digits
        assert_eq!(None, parse_color("#1"));
        assert_eq!(None, parse_color("#12"));
        assert_eq!(None, parse_color("#12345"));
        assert_eq!(None, parse_color("#1234567"));
        assert_eq!(None, parse_color("#123456789"));
        assert_eq!(None, parse_color("12"));
        assert_eq!(None, parse_color("abcde"));

        // C-style "0x" hex prefix is not allowed
        assert_eq!(None, parse_color("0x4488CC"));

        // invalid hex characters
        assert_eq!(None, parse_color("#hh0033"));
        assert_eq!(None, parse_color("#h03"));
        assert_eq!(None, parse_color("03h"));

        // no space between '#' and value
        assert_eq!(None, parse_color("# c6c6c6"));
    }

    #[test]
    fn css_rgb_fn_legacy() {
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
        assert_eq!(
            Some(rgb(3, 54, 119)),
            parse_color("rgb(1.1765%,21.1765%,46.6667%)")
        );
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
    fn css_rgb_fn_legacy_alpha() {
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
    fn rgb_fn_legacy_lenient() {
        // allow mixing numbers and percentages
        assert_eq!(
            rgbf(128.0 / 255.0, 0.8, 1.0),
            parse_color("rgb(128, 80%, 255)")
        );
        assert_eq!(rgbf(1.0, 0.0, 0.0), parse_color("rgb(100%, 0, 0)"));
    }

    #[test]
    fn rgb_fn_legacy_invalid() {
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
    fn rgb_fn_legacy_alpha_invalid() {
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
    fn css_rgb_fn_modern() {
        assert_eq!(Some(rgb(255, 0, 153)), parse_color("rgb(255 0 153)"));

        assert_eq!(
            Some(rgb(3, 54, 119)),
            parse_color("rgb(1.1765% 21.1765% 46.6667%)")
        );
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
    fn css_rgb_fn_modern_alpha() {
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
    fn rgb_fn_modern_lenient() {
        // allow mixing numbers and percentages
        assert_eq!(
            rgbf(128.0 / 255.0, 0.8, 1.0),
            parse_color("rgb(128 80% 255)")
        );
        assert_eq!(rgbf(1.0, 0.0, 0.0), parse_color("rgb(100% 0 0)"));

        // no slash before alpha value
        assert_eq!(
            rgbaf(1.0, 0.0, 0.6, 0.3),
            parse_color("rgb(100% 0% 60% 30%)")
        );
        assert_eq!(
            rgbaf255(192.0, 128.0, 64.0, 0.3),
            parse_color("rgb(192 128 64 0.3)")
        );
    }

    #[test]
    fn rgb_fn_modern_invalid() {
        // not enough arguments
        assert_eq!(None, parse_color("rgb(255 0)"));
        // missing parenthesis
        assert_eq!(None, parse_color("rgb(255 0 0"));
    }

    #[test]
    fn rgb_fn_modern_alpha_invalid() {
        // not enough arguments before slash
        assert_eq!(None, parse_color("rgb(255 0 / 0.3)"));
        // too many arguments before slash
        assert_eq!(None, parse_color("rgb(100% 0% 60% 30% / 0.3)"));
        // no alpha value after slash
        assert_eq!(None, parse_color("rgb(255 0 30 /)"));

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

    // BEGIN: tests for hue angle variants
    // Tests all the supported alternatives for specifying a hue angle so that
    // we don't need to check them separately for each cylindrical color space.

    fn unwrap<T>(result: IResult<&str, T>) -> Option<T> {
        result.ok().map(|(_, v)| v)
    }

    fn parse_hue(input: &str) -> Option<f64> {
        unwrap(all_consuming(hue_angle)(input))
    }

    #[test]
    fn css_hue_angle_degrees() {
        // unit suffix "deg" (case-insensitive)
        assert_eq!(Some(123.0), parse_hue("123deg"));
        assert_eq!(Some(123.0), parse_hue("123DEG"));

        // decimals are supported
        assert_eq!(Some(123.456), parse_hue("123.456deg"));

        // no unit defaults to degrees
        assert_eq!(Some(123.0), parse_hue("123.0"));

        // out-of-range values are accepted and not normalized
        assert_eq!(Some(-90.0), parse_hue("-90deg"));
        assert_eq!(Some(720.0), parse_hue("720deg"));
    }

    #[test]
    fn css_hue_angle_radians() {
        // unit suffix "rad" (case-insensitive)
        assert_eq!(Some(180.0 / PI), parse_hue("1rad"));
        assert_eq!(Some(180.0 / PI), parse_hue("1RAD"));

        // decimals are supported
        assert_eq!(Some(1.234 * 180.0 / PI), parse_hue("1.234rad"));

        // out-of-range values are accepted and not normalized
        assert_eq!(Some(-1.0 * 180.0 / PI), parse_hue("-1rad"));
        assert_eq!(Some(5.0 * 180.0 / PI), parse_hue("5rad"));
    }

    #[test]
    fn css_hue_angle_gradians() {
        // unit suffix "grad" (case-insensitive)
        assert_eq!(Some(180.0), unwrap(hue_angle("200grad")));
        assert_eq!(Some(180.0), unwrap(hue_angle("200GRAD")));

        // decimals are supported
        assert_eq!(Some(123.45 * 360. / 400.), parse_hue("123.45grad"));

        // out-of-range values are accepted and not normalized
        assert_eq!(Some(-90.0), parse_hue("-100grad"));
        assert_eq!(Some(720.0), parse_hue("800grad"));
    }

    #[test]
    fn css_hue_angle_turns() {
        // unit suffix "turn" (case-insensitive)
        assert_eq!(Some(180.0), parse_hue("0.5turn"));
        assert_eq!(Some(270.0), parse_hue("0.75TURN"));

        // out-of-range values are accepted and not normalized
        assert_eq!(Some(-90.0), parse_hue("-0.25turn"));
        assert_eq!(Some(720.0), parse_hue("2turn"));
    }

    #[test]
    fn hue_angle_lenient() {
        // Tests acceaptable alternatives that aren't defined in the CSS spec.

        // accept the "°" degree symbol as a unit
        assert_eq!(Some(123.4), parse_hue("123.4°"));

        // accept "grd" as an abbreviation for gradians
        assert_eq!(Some(180.0), unwrap(hue_angle("200grd")));
        assert_eq!(Some(180.0), unwrap(hue_angle("200GRD")));

        // accept "trn" as an abbreviation for turns
        assert_eq!(Some(180.0), parse_hue("0.5trn"));
        assert_eq!(Some(270.0), parse_hue("0.75TRN"));
    }

    #[test]
    fn hue_angle_invalid() {
        // must have a valid unit
        assert_eq!(None, parse_hue("123abc"));

        // spelling out the unit names is not accepted
        assert_eq!(None, parse_hue("123degrees"));
        assert_eq!(None, parse_hue("1.5radians"));
        assert_eq!(None, parse_hue("150gradians"));
        assert_eq!(None, parse_hue("0.25turns"));

        // can't have a space between value and unit
        assert_eq!(None, parse_hue("100 grad"));
    }

    // END: tests for hue angle variants

    #[test]
    fn css_hsl_fn_legacy() {
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
    fn css_hsl_fn_legacy_alpha() {
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
    fn hsl_fn_legacy_invalid() {
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
    fn hsl_fn_legacy_alpha_invalid() {
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
    fn css_hsl_fn_modern() {
        assert_eq!(
            Some(Color::from_hsl(270.0, 0.6, 0.7)),
            parse_color("hsl(270 60% 70%)")
        );
    }

    #[test]
    fn css_hsl_fn_modern_alpha() {
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
    fn hsl_fn_modern_lenient() {
        // no slash before alpha value
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.5)), 
            parse_color("hsl(280 20% 50% 0.5)")
        );
        assert_eq!(
            Some(Color::from_hsla(280.0, 0.2, 0.5, 0.5)), 
            parse_color("hsl(280 20% 50% 50%)")
        );
    }

    #[test]
    fn hsl_fn_modern_invalid() {
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
    fn hsl_fn_modern_alpha_invalid() {
        // not enough arguments before slash
        assert_eq!(None, parse_color("hsl(180 70% / 0.3)"));
        // too many arguments before slash
        assert_eq!(None, parse_color("hsl(180 70% 60% 30% / 0.3)"));
        // no alpha value after slash
        assert_eq!(None, parse_color("hsl(180 70% 50% /)"));

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
    fn xyz_fn() {
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
    fn xyz_fn_alpha() {
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
    fn css_lab_fn() {
        // Tests the standard CSS syntax for the `lab()` function.

        assert_eq!(
            Some(Color::from_lab(42.24, -35.5, 43.4, 1.0)),
            parse_color("lab(42.24% -35.5 43.4)")
        );
        // case-insensitive
        assert_eq!(
            Some(Color::from_lab(50.0, -35.5, 43.4, 1.0)),
            parse_color("LaB(50% -35.5 43.4)")
        );
        // extra whitespace before and after arguments is ingored
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(   15 23 -43    )")
        );
        // extra whitespace between arguments is ignored
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(15    23      -43)")
        );
    }

    #[test]
    fn css_lab_fn_alpha() {
        // Tests the standard CSS syntax for the `lab()` function with alpha values.

        assert_eq!(
            Some(Color::from_lab(50.0, -23.0, 43.0, 0.5)),
            parse_color("lab(50% -23 43 / 0.5)")
        );
        // alpha can be a percentage
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("lab(15% -35.5 -43.4 / 40%)")
        );
        // no spaces required around alpha separator
        assert_eq!(
            Some(Color::from_lab(75.0, -35.5, -43.4, 0.4)),
            parse_color("lab(75% -35.5 -43.4/0.4)")
        );
    }

    #[test]
    fn lab_fn_modern_lenient() {
        // Tests lenient parsing of the "modern" space-separated format.

        // percent sign after lightness is optional
        assert_eq!(
            Some(Color::from_lab(60.0, -35.5, 43.4, 1.0)),
            parse_color("lab(60.00 -35.5 43.4)")
        );

        // alpha can be separated with just a space
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("lab(15% -35.5 -43.4 40%)")
        );
        assert_eq!(
            Some(Color::from_lab(75.0, -35.5, -43.4, 0.4)),
            parse_color("lab(75% -35.5 -43.4 0.4)")
        );
    }

    #[test]
    fn lab_fn_legacy() {
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(15, 23, -43)")
        );
        // percent sign after lightness is allowed
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(15%, 23, -43)")
        );
        // case-insensitive, no spaces around commands between arguments
        assert_eq!(
            Some(Color::from_lab(12.43, -35.5, 43.4, 1.0)),
            parse_color("Lab(12.43,-35.5,43.4)")
        );
        // extra whitespace before and after arguments is ignored
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(        15, 23, -43   )")
        );
        // extra whitespace around commas between arguments is ignored
        assert_eq!(
            Some(Color::from_lab(15.0, 23.0, -43.0, 1.0)),
            parse_color("lab(15   ,  23 ,-43)")
        );
    }

    #[test]
    fn lab_fn_legacy_alpha() {
        // alpha as a number from [0, 1]
        assert_eq!(
            Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
            parse_color("lab(15%, -23, 43, 0.5)")
        );
        // case-insensitive, alpha as a percentage
        assert_eq!(
            Some(Color::from_lab(15.0, -23.0, 43.0, 0.5)),
            parse_color("LAB(15%, -23, 43, 50%)")
        );
        // extra spaces around comma before alpha value are ignored
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("lab(15, -35.5, -43.4     ,  0.4)")
        );
        assert_eq!(
            Some(Color::from_lab(15.0, -35.5, -43.4, 0.4)),
            parse_color("lab(15,-35.5,-43.4    ,    40%)")
        );
    }

    #[test]
    fn lab_fn_cielab_alias() {
        // Tests that "CIELab" (case-insensitive) is a valid alias for "lab"
        // with either modern- or legacy-style arguments.

        // modern-style arguments
        assert_eq!(
            parse_color("lab(70% -25 40)"),
            parse_color("CIELab(70% -25 40)")
        );
        assert_eq!(
            parse_color("lab(70% -25 40 / 0.8)"),
            parse_color("cielab(70% -25 40 / 80%)")
        );
        assert_eq!(
            parse_color("lab(70% -25 40 / 0.8)"),
            parse_color("cielab(70 -25 40 0.8)")
        );

        // legacy-style arguments
        assert_eq!(
            parse_color("lab(70%, -25, 40)"),
            parse_color("CIELab(70%, -25, 40)")
        );
        assert_eq!(
            parse_color("lab(70%, -25, 40, 80%)"),
            parse_color("cielab(70%, -25, 40, 80%)")
        );
        assert_eq!(
            parse_color("lab(70% -25 40 / 0.8)"),
            parse_color("CIELAB(70, -25, 40, 0.8)")
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
