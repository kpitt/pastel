use std::fmt;

use super::helper::Fraction;
use super::helper::MaxPrecision;
use super::types::{Hue, Scalar};
use super::Color;
use super::Format;
use super::{LCh, Lab, CMYK, HSLA, HSVA, LMS, RGBA, XYZ};

// impl Color {
//     /// Format the color as a HSL-representation string (`hsla(123, 50.3%, 80.1%, 0.4)`). If the
//     /// alpha channel is `1.0`, the simplified `hsl()` format will be used instead.
//     pub fn to_hsl_string(&self, format: Format) -> String {
//         let (a_prefix, a) = if self.alpha == 1.0 {
//             ("", "".to_string())
//         } else {
//             (
//                 "a",
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, self.alpha),
//                 ),
//             )
//         };
//         format!(
//             "hsl{a_prefix}({h:.0}, {s:.1}%, {l:.1}%{a})",
//             a_prefix = a_prefix,
//             h = self.hue.value(),
//             s = 100.0 * self.saturation,
//             l = 100.0 * self.lightness,
//             a = a,
//         )
//     }

//     /// Format the color as a HSV-representation string (`hsva(123, 50.3%, 80.1%, 0.4)`). If the
//     /// alpha channel is `1.0`, the simplified `hsv()` format will be used instead.
//     pub fn to_hsv_string(&self, format: Format) -> String {
//         let hsv = HSVA::from(self);
//         let (a_prefix, a) = if hsv.alpha == 1.0 {
//             ("", "".to_string())
//         } else {
//             (
//                 "a",
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, hsv.alpha),
//                 ),
//             )
//         };
//         format!(
//             "hsv{a_prefix}({h:.0}, {s:.1}%, {v:.1}%{a})",
//             a_prefix = a_prefix,
//             h = hsv.h,
//             s = 100.0 * hsv.s,
//             v = 100.0 * hsv.v,
//             a = a,
//         )
//     }

//     /// Format the color as a RGB-representation string (`rgba(255, 127, 0, 0.5)`). If the alpha channel
//     /// is `1.0`, the simplified `rgb()` format will be used instead.
//     pub fn to_rgb_string(&self, format: Format) -> String {
//         let rgba = RGBA::<u8>::from(self);
//         let (a_prefix, a) = if self.alpha == 1.0 {
//             ("", "".to_string())
//         } else {
//             (
//                 "a",
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, rgba.alpha),
//                 ),
//             )
//         };
//         format!(
//             "rgb{a_prefix}({r}, {g}, {b}{a})",
//             a_prefix = a_prefix,
//             r = rgba.r,
//             g = rgba.g,
//             b = rgba.b,
//             a = a,
//         )
//     }

//     /// Format the color as a CMYK-representation string (`cmyk(0, 50, 100, 100)`).
//     pub fn to_cmyk_string(&self, format: Format) -> String {
//         let cmyk = CMYK::from(self);
//         format!(
//             "cmyk({c}, {m}, {y}, {k})",
//             c = (cmyk.c * 100.0).round(),
//             m = (cmyk.m * 100.0).round(),
//             y = (cmyk.y * 100.0).round(),
//             k = (cmyk.k * 100.0).round(),
//         )
//     }

//     /// Format the color as a floating point RGB-representation string (`rgb(1.0, 0.5, 0)`). If the alpha channel
//     /// is `1.0`, the simplified `rgb()` format will be used instead.
//     pub fn to_rgb_float_string(&self, format: Format) -> String {
//         let rgba = RGBA::<f64>::from(self);
//         let (a_prefix, a) = if self.alpha == 1.0 {
//             ("", "".to_string())
//         } else {
//             (
//                 "a",
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, rgba.alpha),
//                 ),
//             )
//         };
//         format!(
//             "rgb{a_prefix}({r:.3}, {g:.3}, {b:.3}{a})",
//             a_prefix = a_prefix,
//             r = rgba.r,
//             g = rgba.g,
//             b = rgba.b,
//             a = a,
//         )
//     }

//     /// Format the color as a RGB-representation string (`#fc0070`). The output will contain 6 hex
//     /// digits if the alpha channel is `1.0`, or 8 hex digits otherwise.
//     pub fn to_rgb_hex_string(&self, leading_hash: bool) -> String {
//         let rgba = self.to_rgba();
//         format!(
//             "{}{:02x}{:02x}{:02x}{}",
//             if leading_hash { "#" } else { "" },
//             rgba.r,
//             rgba.g,
//             rgba.b,
//             if rgba.alpha == 1.0 {
//                 "".to_string()
//             } else {
//                 format!("{:02x}", (rgba.alpha * 255.).round() as u8)
//             }
//         )
//     }

//     /// Format the color as a Lab-representation string (`Lab(41, 83, -93, 0.5)`). If the alpha channel
//     /// is `1.0`, it won't be included in the output.
//     pub fn to_lab_string(&self, format: Format) -> String {
//         let lab = Lab::from(self);
//         format!(
//             "Lab({l:.0}, {a:.0}, {b:.0}{alpha})",
//             l = lab.l,
//             a = lab.a,
//             b = lab.b,
//             alpha = if self.alpha == 1.0 {
//                 "".to_string()
//             } else {
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, self.alpha),
//                 )
//             }
//         )
//     }

//     /// Format the color as a LCh-representation string (`LCh(0.3, 0.2, 0.1, 0.5)`). If the alpha channel
//     /// is `1.0`, it won't be included in the output.
//     pub fn to_lch_string(&self, format: Format) -> String {
//         let lch = LCh::from(self);
//         format!(
//             "LCh({l:.0}, {c:.0}, {h:.0}{alpha})",
//             l = lch.l,
//             c = lch.c,
//             h = lch.h,
//             alpha = if self.alpha == 1.0 {
//                 "".to_string()
//             } else {
//                 format!(
//                     ", {alpha}",
//                     alpha = MaxPrecision::wrap(3, self.alpha),
//                 )
//             }
//         )
//     }
// }

// by default Colors will be printed into HSLA format
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", HSLA::from(self))
    }
}

impl fmt::Display for RGBA<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl fmt::Display for RGBA<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({r}, {g}, {b})", r = self.r, g = self.g, b = self.b,)
    }
}

impl fmt::Display for HSLA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsl({h}, {s}, {l})", h = self.h, s = self.s, l = self.l,)
    }
}

impl fmt::Display for HSVA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsv({h}, {s}, {v})", h = self.h, s = self.s, v = self.v)
    }
}

impl fmt::Display for XYZ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XYZ({x}, {y}, {z})", x = self.x, y = self.y, z = self.z,)
    }
}

impl fmt::Display for LMS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LMS({l}, {m}, {s})", l = self.l, m = self.m, s = self.s,)
    }
}

impl fmt::Display for Lab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lab({l}, {a}, {b})", l = self.l, a = self.a, b = self.b,)
    }
}

impl fmt::Display for LCh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LCh({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

impl fmt::Display for CMYK {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cmyk({c}, {m}, {y}, {k})",
            c = self.c,
            m = self.m,
            y = self.y,
            k = self.k,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_almost_equal(c1: &Color, c2: &Color) {
        let c1 = c1.to_rgba();
        let c2 = c2.to_rgba();

        assert!((c1.r as i32 - c2.r as i32).abs() <= 1);
        assert!((c1.g as i32 - c2.g as i32).abs() <= 1);
        assert!((c1.b as i32 - c2.b as i32).abs() <= 1);
    }

    #[test]
    fn to_hsl_string() {
        let c = Color::from_hsl(91.3, 0.541, 0.983);
        assert_eq!("hsl(91, 54.1%, 98.3%)", c.to_hsl_string());
    }

    #[test]
    fn to_rgb_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("rgb(255, 127, 4)", c.to_rgb_string());
    }

    #[test]
    fn to_rgb_float_string() {
        assert_eq!(
            "rgb(0.000, 0.000, 0.000)",
            Color::black().to_rgb_float_string()
        );

        assert_eq!(
            "rgb(1.000, 1.000, 1.000)",
            Color::white().to_rgb_float_string()
        );

        // some minor rounding errors here, but that is to be expected:
        let c = Color::from_rgb_float(0.12, 0.45, 0.78);
        assert_eq!("rgb(0.122, 0.451, 0.780)", c.to_rgb_float_string());
    }

    #[test]
    fn to_rgb_hex_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("ff7f04", c.to_rgb_hex_string(false));
        assert_eq!("#ff7f04", c.to_rgb_hex_string(true));
    }

    #[test]
    fn to_lab_string() {
        let c = Color::from_lab(41.0, 83.0, -93.0, 1.0);
        assert_eq!("Lab(41, 83, -93)", c.to_lab_string());
    }

    #[test]
    fn to_lch_string() {
        let c = Color::from_lch(52.0, 44.0, 271.0, 1.0);
        assert_eq!("LCh(52, 44, 271)", c.to_lch_string());
    }

    #[test]
    fn color_scale_sample_same_position() {
        let mix = Color::mix::<Lab>;

        let mut color_scale = ColorScale::empty();

        color_scale
            .add_stop(Color::red(), Fraction::from(0.0))
            .add_stop(Color::green(), Fraction::from(1.0))
            .add_stop(Color::blue(), Fraction::from(0.0))
            .add_stop(Color::white(), Fraction::from(1.0));

        let sample_blue = color_scale.sample(Fraction::from(0.0), &mix).unwrap();
        let sample_white = color_scale.sample(Fraction::from(1.0), &mix).unwrap();

        assert_eq!(sample_blue, Color::blue());
        assert_eq!(sample_white, Color::white());
    }

    #[test]
    fn color_scale_sample() {
        let mix = Color::mix::<Lab>;

        let mut color_scale = ColorScale::empty();

        color_scale
            .add_stop(Color::green(), Fraction::from(1.0))
            .add_stop(Color::red(), Fraction::from(0.0));

        let sample_red_green = color_scale.sample(Fraction::from(0.5), &mix).unwrap();

        let mix_red_green = mix(&Color::red(), &Color::green(), Fraction::from(0.5));

        assert_eq!(sample_red_green, mix_red_green);
    }

    #[test]
    fn color_scale_sample_position() {
        let mix = Color::mix::<Lab>;

        let mut color_scale = ColorScale::empty();

        color_scale
            .add_stop(Color::green(), Fraction::from(0.5))
            .add_stop(Color::red(), Fraction::from(0.0))
            .add_stop(Color::blue(), Fraction::from(1.0));

        let sample_red = color_scale.sample(Fraction::from(0.0), &mix).unwrap();
        let sample_green = color_scale.sample(Fraction::from(0.5), &mix).unwrap();
        let sample_blue = color_scale.sample(Fraction::from(1.0), &mix).unwrap();

        let sample_red_green = color_scale.sample(Fraction::from(0.25), &mix).unwrap();
        let sample_green_blue = color_scale.sample(Fraction::from(0.75), &mix).unwrap();

        let mix_red_green = mix(&Color::red(), &Color::green(), Fraction::from(0.50));
        let mix_green_blue = mix(&Color::green(), &Color::blue(), Fraction::from(0.50));

        assert_eq!(sample_red, Color::red());
        assert_eq!(sample_green, Color::green());
        assert_eq!(sample_blue, Color::blue());

        assert_eq!(sample_red_green, mix_red_green);
        assert_eq!(sample_green_blue, mix_green_blue);
    }

    #[test]
    fn to_cmyk_string() {
        let white = Color::from_rgb(255, 255, 255);
        assert_eq!("cmyk(0, 0, 0, 0)", white.to_cmyk_string());

        let black = Color::from_rgb(0, 0, 0);
        assert_eq!("cmyk(0, 0, 0, 100)", black.to_cmyk_string());

        let c = Color::from_rgb(19, 19, 1);
        assert_eq!("cmyk(0, 0, 95, 93)", c.to_cmyk_string());

        let c1 = Color::from_rgb(55, 55, 55);
        assert_eq!("cmyk(0, 0, 0, 78)", c1.to_cmyk_string());

        let c2 = Color::from_rgb(136, 117, 78);
        assert_eq!("cmyk(0, 14, 43, 47)", c2.to_cmyk_string());

        let c3 = Color::from_rgb(143, 111, 76);
        assert_eq!("cmyk(0, 22, 47, 44)", c3.to_cmyk_string());
    }

    #[test]
    fn alpha_roundtrip_hex_to_decimal() {
        // We use a max of 3 decimal places when displaying RGB floating point
        // alpha values. This test insures that is sufficient to "roundtrip"
        // from hex (0 < n < 255) to float (0 < n < 1) and back again,
        // e.g. hex `80` is float `0.502`, which parses to hex `80`, and so on.
        for alpha_int in 0..255 {
            let hex_string = format!("#000000{:02x}", alpha_int);
            let parsed_from_hex = hex_string.parse::<Color>().unwrap();
            let rgba_string = parsed_from_hex.to_rgb_float_string();
            let parsed_from_rgba = rgba_string.parse::<Color>().unwrap();
            assert_eq!(hex_string, parsed_from_rgba.to_rgb_hex_string(true));
        }
    }
}
