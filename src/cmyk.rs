use std::fmt;

use crate::{rgb::RGBA, types::Scalar, Color, Format};

#[derive(Debug, Clone, PartialEq)]
pub struct CMYK {
    pub c: Scalar,
    pub m: Scalar,
    pub y: Scalar,
    pub k: Scalar,
}

impl From<&Color> for CMYK {
    fn from(color: &Color) -> Self {
        let rgba = RGBA::<u8>::from(color);
        let r = (rgba.r as f64) / 255.0;
        let g = (rgba.g as f64) / 255.0;
        let b = (rgba.b as f64) / 255.0;
        let biggest = if r >= g && r >= b {
            r
        } else if g >= r && g >= b {
            g
        } else {
            b
        };
        let out_k = 1.0 - biggest;
        let out_c = (1.0 - r - out_k) / biggest;
        let out_m = (1.0 - g - out_k) / biggest;
        let out_y = (1.0 - b - out_k) / biggest;

        CMYK {
            c: if out_c.is_nan() { 0.0 } else { out_c },
            m: if out_m.is_nan() { 0.0 } else { out_m },
            y: if out_y.is_nan() { 0.0 } else { out_y },
            k: out_k,
        }
    }
}

// from CMYK to Color so you can do -> let new_color = Color::from(&some_cmyk);
impl From<&CMYK> for Color {
    fn from(color: &CMYK) -> Self {
        #![allow(clippy::many_single_char_names)]
        let r = (1.0 - color.c) * (1.0 - color.k);
        let g = (1.0 - color.m) * (1.0 - color.k);
        let b = (1.0 - color.y) * (1.0 - color.k);

        Color::from(&RGBA::new(r, g, b))
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

impl CMYK {
    #[inline]
    pub fn new(c: Scalar, m: Scalar, y: Scalar, k: Scalar) -> Self {
        CMYK { c, m, y, k }
    }

    /// Format the color as a CMYK-representation string (`cmyk(0, 50, 100, 100)`).
    pub fn to_color_string(&self, format: Format) -> String {
        format!(
            "cmyk({c},{space}{m},{space}{y},{space}{k})",
            c = (self.c * 100.0).round(),
            m = (self.m * 100.0).round(),
            y = (self.y * 100.0).round(),
            k = (self.k * 100.0).round(),
            space = if format == Format::Spaces { " " } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_conversion() {
        assert_eq!(Color::aqua(), Color::from_cmyk(1.0, 0.0, 0.0, 0.0));
        assert_eq!(Color::fuchsia(), Color::from_cmyk(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Color::yellow(), Color::from_cmyk(0.0, 0.0, 1.0, 0.0));
        assert_eq!(Color::black(), Color::from_cmyk(0.0, 0.0, 0.0, 1.0));

        assert_eq!(Color::red(), Color::from_cmyk(0.0, 1.0, 1.0, 0.0));
        assert_eq!(Color::lime(), Color::from_cmyk(1.0, 0.0, 1.0, 0.0));
        assert_eq!(Color::blue(), Color::from_cmyk(1.0, 1.0, 0.0, 0.0));

        assert_eq!(Color::green(), Color::from_cmyk(1.0, 0.0, 1.0, 0.5));
    }

    #[test]
    fn to_cmyk_string() {
        let white = CMYK::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!("cmyk(0, 0, 0, 0)", white.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,0,0,0)", white.to_color_string(Format::NoSpaces));

        let black = CMYK::new(0.0, 0.0, 0.0, 1.0);
        assert_eq!("cmyk(0, 0, 0, 100)", black.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,0,0,100)", black.to_color_string(Format::NoSpaces));

        let gray = CMYK::new(0.0, 0.0, 0.0, 0.75);
        assert_eq!("cmyk(0, 0, 0, 75)", gray.to_color_string(Format::Spaces));

        let c1 = CMYK::new(0.0, 0.0, 0.95, 0.9);
        assert_eq!("cmyk(0, 0, 95, 90)", c1.to_color_string(Format::Spaces));

        let c2 = CMYK::new(0.0, 0.14, 0.43, 0.47);
        assert_eq!("cmyk(0, 14, 43, 47)", c2.to_color_string(Format::Spaces));
        assert_eq!("cmyk(0,14,43,47)", c2.to_color_string(Format::NoSpaces));

        let c3 = CMYK::new(0.25, 0.1, 0.0, 0.5);
        assert_eq!("cmyk(25, 10, 0, 50)", c3.to_color_string(Format::Spaces));
    }
}
