use std::fmt;

use crate::{
    colorspace::ColorSpace,
    format_css_alpha,
    helper::{clamp, interpolate, interpolate_angle, MaxPrecision},
    hsv::HSVA,
    types::Scalar,
    Color, Format, Fraction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HWBA {
    pub h: Scalar,
    pub w: Scalar,
    pub b: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for HWBA {
    fn from_color(c: &Color) -> Self {
        c.to_hwba()
    }

    fn into_color(self) -> Color {
        Color::from_hwba(self.h, self.w, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if (self.w + self.b) >= 1.0 {
            other.h
        } else {
            self.h
        };
        let other_hue = if (other.w + other.b) >= 1.0 {
            self.h
        } else {
            other.h
        };

        Self {
            h: interpolate_angle(self_hue, other_hue, fraction),
            w: interpolate(self.w, other.w, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for HWBA {
    fn from(color: &Color) -> Self {
        let HSVA { h, s, v, alpha } = HSVA::from(color);

        let w = (1.0 - s) * v;
        let b = 1.0 - v;
        HWBA { h, w, b, alpha }
    }
}

impl From<&HWBA> for Color {
    fn from(color: &HWBA) -> Self {
        if color.w + color.b >= 1.0 {
            let gray = color.w / (color.w + color.b);
            Self::from_rgba_float(gray, gray, gray, color.alpha)
        } else {
            let w = clamp(0.0, 1.0, color.w);
            let b = clamp(0.0, 1.0, color.b);
            let v = 1.0 - b;
            let s = 1.0 - (w / v);
            Self::from(&HSVA::with_alpha(color.h, s, v, color.alpha))
        }
    }
}

impl fmt::Display for HWBA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_color_string(Format::Spaces))
    }
}

impl HWBA {
    #[inline]
    pub fn new(h: Scalar, w: Scalar, b: Scalar) -> Self {
        Self::with_alpha(h, w, b, 1.0)
    }

    #[inline]
    pub fn with_alpha(h: Scalar, w: Scalar, b: Scalar, alpha: Scalar) -> Self {
        HWBA { h, w, b, alpha }
    }

    /// Format the color as a HWB-representation string (`hwb(123, 50.3%, 80.1%)`).
    pub fn to_color_string(&self, format: Format) -> String {
        format!(
            "hwb({h:.0} {w}% {b}%{alpha})",
            h = self.h,
            w = MaxPrecision::wrap(1, 100.0 * self.w),
            b = MaxPrecision::wrap(1, 100.0 * self.b),
            alpha = format_css_alpha(self.alpha, format)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwb_conversion() {
        let rgbf = Color::from_rgb_float;
        let rgb128 = 128.0 / 255.0;

        assert_eq!(Color::white(), Color::from_hwb(0.0, 1.0, 0.0));
        assert_eq!(Color::white(), Color::from_hwb(120.0, 1.0, 0.0));
        assert_eq!(rgbf(0.5, 0.5, 0.5), Color::from_hwb(0.0, 0.5, 0.5));
        assert_eq!(Color::gray(), Color::from_hwb(300.0, rgb128, 1.0 - rgb128));
        assert_eq!(Color::black(), Color::from_hwb(0.0, 0.0, 1.0));
        assert_eq!(Color::black(), Color::from_hwb(240.0, 0.0, 1.0));
        assert_eq!(Color::red(), Color::from_hwb(0.0, 0.0, 0.0));
        assert_eq!(
            Color::from_hsl(60.0, 1.0, 0.375),
            Color::from_hwb(60.0, 0.0, 0.25)
        ); //yellow-green
        assert_eq!(Color::green(), Color::from_hwb(120.0, 0.0, 1.0 - rgb128));
        assert_eq!(
            Color::from_hsl(240.0, 1.0, 0.75),
            Color::from_hwb(240.0, 0.5, 0.0)
        ); // blue-ish

        assert_eq!(
            Color::from_hsl(49.5, 0.8922, 0.4973),
            Color::from_hwb(49.5, 0.0536, 0.0590)
        ); //yellow
        assert_eq!(
            Color::from_hsl(162.4, 0.7794, 0.4468),
            Color::from_hwb(162.4, 0.09856, 0.20496)
        ); // cyan 2

        assert_eq!(
            Color::from_rgba_float(0.75, 0.0, 0.75, 0.4),
            Color::from_hwba(300.0, 0.0, 0.25, 0.4)
        )
    }

    #[test]
    fn hwb_roundtrip_conversion() {
        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let hwb1 = color1.to_hwba();
            let color2 = Color::from_hwb(hwb1.h, hwb1.w, hwb1.b);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn to_hwb_string() {
        let c = HWBA::new(91.0, 0.541, 0.383);
        // modern CSS functional syntax
        assert_eq!("hwb(91 54.1% 38.3%)", c.to_color_string(Format::Spaces));
        // spaces are required, so NoSpaces has no effect without akpha
        assert_eq!("hwb(91 54.1% 38.3%)", c.to_color_string(Format::NoSpaces));

        let c1 = HWBA::new(91.3, 0.541, 0.383172);
        // hue is rounded to integer, w and b are rounded to 1 decimal
        assert_eq!("hwb(91 54.1% 38.3%)", c1.to_color_string(Format::Spaces));

        let c2 = HWBA::new(90.0, 0.5, 0.25);
        // trailing decimal zeros are not included
        assert_eq!("hwb(90 50% 25%)", c2.to_color_string(Format::Spaces));

        let c2a = HWBA::with_alpha(90.0, 0.5, 0.25, 0.8);
        // non-unit alpha is serialized as a number
        assert_eq!("hwb(90 50% 25% / 0.8)", c2a.to_color_string(Format::Spaces));
        // spaces are optional around alpha separator, so NoSpaces applies
        assert_eq!("hwb(90 50% 25%/0.8)", c2a.to_color_string(Format::NoSpaces));
    }
}
