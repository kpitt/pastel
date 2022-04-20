pub mod ansi;
pub mod colorspace;
pub mod delta_e;
pub mod distinct;
mod helper;
pub mod named;
pub mod parser;
pub mod random;
mod types;

use std::fmt;

use colorspace::ColorSpace;
pub use helper::Fraction;
use helper::{clamp, interpolate, interpolate_angle, mod_positive, round_to, round_sig};
use types::Scalar;

/// The representation of a color.
///
/// Note:
/// - Colors are stored as CIE XYZ coordinates, which allows all possible colors
///   to be represented including out-of-gamut colors that cannot be displayed
///   on a typical computer screen.
/// - The `PartialEq` implementation compares two `Color`s by checking if the
///   difference between corresponding XYZ values is less than the precision of
///   a 16-bit integer channel value (about 5 decimal places), which is the
///   minimum round-trip precision required by the CSS Color Level 4 draft spec.
#[derive(Clone)]
pub struct Color {
    x: Scalar,
    y: Scalar,
    z: Scalar,
    alpha: Scalar,
}

// Illuminant D65 constants used for Lab color space conversions.
const D65_XN: Scalar = 0.950_470;
const D65_YN: Scalar = 1.0;
const D65_ZN: Scalar = 1.088_830;

/// Formats an alpha value extension according to the CSS Color 4 serialization
/// rules.  Returns an empty string if the alpha value is exactly 1.  Otherwise,
/// returns the " / " separator followed by the alpha value as a number rounded
/// to 4 decimal places.
///
/// See: https://www.w3.org/TR/css-color-4/#serializing-alpha-values
fn format_css_alpha(a: Scalar) -> String {
    if a == 1.0 { String::new() } else { format!(" / {}", round_to(a, 4)) }
}

impl Color {
    pub fn from_hsla(hue: Scalar, saturation: Scalar, lightness: Scalar, alpha: Scalar) -> Color {
        Self::from(&HSLA {
            h: hue,
            s: saturation,
            l: lightness,
            alpha,
        })
    }

    ///
    pub fn from_hsl(hue: Scalar, saturation: Scalar, lightness: Scalar) -> Color {
        Self::from(&HSLA {
            h: hue,
            s: saturation,
            l: lightness,
            alpha: 1.0,
        })
    }

    /// Create a `Color` from integer RGB values between 0 and 255 and a floating
    /// point alpha value between 0.0 and 1.0.
    pub fn from_rgba(r: u8, g: u8, b: u8, alpha: Scalar) -> Color {
        // RGB to HSL conversion algorithm adapted from
        // https://en.wikipedia.org/wiki/HSL_and_HSV
        Self::from(&RGBA::<u8> { r, g, b, alpha })
    }

    /// Create a `Color` from integer RGB values between 0 and 255.
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Self::from(&RGBA::<u8> {
            r,
            g,
            b,
            alpha: 1.0,
        })
    }

    /// Create a `Color` from RGB and alpha values between 0.0 and 1.0. Values outside this range
    /// will be clamped.
    pub fn from_rgba_float(r: Scalar, g: Scalar, b: Scalar, alpha: Scalar) -> Color {
        Self::from(&RGBA::<f64> { r, g, b, alpha })
    }

    /// Create a `Color` from RGB values between 0.0 and 1.0. Values outside this range will be
    /// clamped.
    pub fn from_rgb_float(r: Scalar, g: Scalar, b: Scalar) -> Color {
        Self::from(&RGBA::<f64> {
            r,
            g,
            b,
            alpha: 1.0,
        })
    }

    /// Create a `Color` from hue, saturation and value coordinates in the HSV color space
    /// and a floating point alpha value between 0.0 and 1.0.
    pub fn from_hsva(hue: Scalar, saturation: Scalar, value: Scalar, alpha: Scalar) -> Color {
        Self::from(&HSVA {
            h: hue,
            s: saturation,
            v: value,
            alpha,
        })
    }

    /// Create a `Color` from hue, saturation and value coordinates in the HSV color space.
    ///
    /// See:
    /// - https://en.wikipedia.org/wiki/HWB_color_model
    pub fn from_hsv(hue: Scalar, saturation: Scalar, value: Scalar) -> Color {
        Self::from(&HSVA {
            h: hue,
            s: saturation,
            v: value,
            alpha: 1.0,
        })
    }

    /// Create a `Color` from a hue, and whiteness and blackness values with a
    /// floating point alpha value between 0.0 and 1.0.
    pub fn from_hwba(hue: Scalar, whiteness: Scalar, blackness: Scalar, alpha: Scalar) -> Color {
        Self::from(&HWBA {
            h: hue,
            w: whiteness,
            b: blackness,
            alpha,
        })
    }

    /// Create a `Color` from a hue, and whiteness and blackness values with a
    /// floating point alpha value between 0.0 and 1.0.
    pub fn from_hwb(hue: Scalar, whiteness: Scalar, blackness: Scalar) -> Color {
        Self::from_hwba(
            hue,
            whiteness,
            blackness,
            1.0,
        )
    }

    /// Create a `Color` from XYZ coordinates in the CIE 1931 color space.
    ///
    /// See:
    /// - https://en.wikipedia.org/wiki/CIE_1931_color_space
    pub fn from_xyz(x: Scalar, y: Scalar, z: Scalar) -> Color {
        Self::from_xyza(x, y, z, 1.0)
    }

    /// Create a `Color` from XYZ coordinates in the CIE 1931 color space and a
    /// floating point alpha value between 0.0 and 1.0.
    pub fn from_xyza(x: Scalar, y: Scalar, z: Scalar, alpha: Scalar) -> Color {
        Self::from(&XYZ { x, y, z, alpha })
    }

    /// Create a `Color` from LMS coordinates. This is the matrix inverse of the matrix that
    /// appears in `to_lms`.
    pub fn from_lms(l: Scalar, m: Scalar, s: Scalar, alpha: Scalar) -> Color {
        Self::from(&LMS { l, m, s, alpha })
    }

    /// Create a `Color` from L, a and b coordinates coordinates in the Lab color
    /// space. Note: See documentation for `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://en.wikipedia.org/wiki/Lab_color_space
    pub fn from_lab(l: Scalar, a: Scalar, b: Scalar, alpha: Scalar) -> Color {
        Self::from(&Lab { l, a, b, alpha })
    }

    /// Create a `Color` from lightness, chroma and hue coordinates in the CIE LCh color space.
    /// This is a cylindrical transform of the Lab color space. Note: See documentation for
    /// `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://en.wikipedia.org/wiki/Lab_color_space
    pub fn from_lch(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Color {
        Self::from(&LCh { l, c, h, alpha })
    }

    /// Create a `Color` from L, u and v coordinates coordinates in the CIE Luv color
    /// space. Note: See documentation for `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://en.wikipedia.org/wiki/CIELUV
    pub fn from_luv(l: Scalar, u: Scalar, v: Scalar, alpha: Scalar) -> Color {
        Self::from(&Luv { l, u, v, alpha })
    }

    /// Create a `Color` from lightness, chroma and hue coordinates in the CIE LCh(uv) color
    /// space. This is a cylindrical transform of the Luv color space. Note: See documentation
    /// for `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://en.wikipedia.org/wiki/CIELUV#Cylindrical_representation_(CIELCh)
    pub fn from_lchuv(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Color {
        Self::from(&LChuv { l, c, h, alpha })
    }

    /// Create a `Color` from L, a and b coordinates coordinates in the OKLab color
    /// space. Note: See documentation for `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://bottosson.github.io/posts/oklab/
    pub fn from_oklaba(l: Scalar, a: Scalar, b: Scalar, alpha: Scalar) -> Color {
        Self::from(&OKLab { l, a, b, alpha })
    }

    pub fn from_oklab(l: Scalar, a: Scalar, b: Scalar) -> Color {
        Self::from_oklaba(l, a, b, 1.0)
    }

    /// Create a `Color` from lightness, chroma and hue coordinates in the OKLCh color space.
    /// This is a cylindrical transform of the OKLab color space. Note: See documentation for
    /// `from_xyz`. The same restrictions apply here.
    ///
    /// See: https://bottosson.github.io/posts/oklab/
    pub fn from_oklcha(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Color {
        Self::from(&OKLCh { l, c, h, alpha })
    }

    pub fn from_oklch(l: Scalar, c: Scalar, h: Scalar) -> Color {
        Self::from_oklcha(l, c, h, 1.0)
    }

    /// Create a `Color` from  the four colours of the CMYK model: Cyan, Magenta, Yellow and Black.
    /// The CMYK colours are subtractive. This means the colours get darker as you blend them together
    pub fn from_cmyk(c: Scalar, m: Scalar, y: Scalar, k: Scalar) -> Color {
        Self::from(&CMYK { c, m, y, k })
    }

    /// Convert a `Color` to its hue, saturation, lightness and alpha values. The hue is given
    /// in degrees, as a number between 0.0 and 360.0. Saturation, lightness and alpha are numbers
    /// between 0.0 and 1.0.
    pub fn to_hsla(&self) -> HSLA {
        HSLA::from(self)
    }

    /// Format the color as a HSL-representation string (`hsl(123, 50.3%, 80.1%)`).
    pub fn to_hsl_string(&self) -> String {
        let hsl100 = |v| { round_to(100.0 * v, 4) };
        let hsl = HSLA::from(self);
        if self.alpha == 1.0 {
            format!(
                "hsl({h}, {s}%, {l}%)",
                h = round_to(hsl.h, 3),
                s = hsl100(hsl.s),
                l = hsl100(hsl.l),
            )
        } else {
            format!(
                "hsla({h}, {s}%, {l}%, {alpha})",
                h = round_to(hsl.h, 3),
                s = hsl100(hsl.s),
                l = hsl100(hsl.l),
                alpha = round_to(self.alpha, 4),
            )
        }
    }

    /// Format the color as the shorter original HSL string for use with color
    /// block output.
    pub fn to_hsl_string_short(&self) -> String {
        let hsl = HSLA::from(self);
        format!(
            "hsl({h}, {s}%, {l}%)",
            h = hsl.h.round(),
            s = round_to(100.0 * hsl.s, 1),
            l = round_to(100.0 * hsl.l, 1),
        )
    }

    /// Convert a `Color` to its red, green, blue and alpha values. The RGB values are integers in
    /// the range from 0 to 255. The alpha channel is a number between 0.0 and 1.0.
    pub fn to_rgba(&self) -> RGBA<u8> {
        RGBA::<u8>::from(self)
    }

    /// Format the color as a RGB-representation string (`rgb(255, 127,  0)`).
    pub fn to_rgb_string(&self) -> String {
        let rgba = RGBA::<u8>::from(self);
        if self.alpha == 1.0 {
            format!("rgb({r}, {g}, {b})",
                r = rgba.r,
                g = rgba.g,
                b = rgba.b,
            )
        } else {
            format!("rgba({r}, {g}, {b}, {alpha})",
                r = rgba.r,
                g = rgba.g,
                b = rgba.b,
                alpha = round_to(self.alpha, 4)
            )
        }
    }

    /// Convert a `Color` to its hue, saturation, value/brightness and alpha values. The hue is given
    /// in degrees, as a number between 0.0 and 360.0. Saturation, value and alpha are numbers
    /// between 0.0 and 1.0.
    pub fn to_hsva(&self) -> HSVA {
        HSVA::from(self)
    }

    /// Format the color as a HSV-representation string (`hsv(123 50.3% 80.1%)`).
    pub fn to_hsv_string(&self) -> String {
        let hsv100 = |v| { round_to(100.0 * v, 4) };
        let hsv = HSVA::from(self);
        format!(
            "hsv({h} {s}% {v}%{alpha})",
            h = round_to(hsv.h, 3),
            s = hsv100(hsv.s),
            v = hsv100(hsv.v),
            alpha = format_css_alpha(self.alpha)
        )
    }

    /// Convert a `Color` to its hue, whiteness, blackness, and alpha values. The
    /// hue is given in degrees, as a number between 0.0 and 360.0. Whiteness,
    /// blackness, and alpha are numbers between 0.0 and 1.0.
    pub fn to_hwba(&self) -> HWBA {
        HWBA::from(self)
    }

    /// Format the color as a HWB-representation string (`hwb(123 50.3% 80.1%)`).
    pub fn to_hwb_string(&self) -> String {
        let rd100 = |v| { round_to(100.0 * v, 4) };
        let hwb = HWBA::from(self);
        format!(
            "hwb({h} {w}% {b}%{alpha})",
            h = round_to(hwb.h, 3),
            w = rd100(hwb.w),
            b = rd100(hwb.b),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Convert a `Color` to its cyan, magenta, yellow, and black values. The CMYK
    /// values are floats smaller than or equal to 1.0.
    pub fn to_cmyk(&self) -> CMYK {
        CMYK::from(self)
    }

    /// Format the color as a CMYK-representation string (`device-cmyk(0% 50% 100% 100%)`).
    pub fn to_cmyk_string(&self) -> String {
        let rd100 = |v| { round_to(100.0 * v, 4) };
        let cmyk = CMYK::from(self);
        format!(
            "device-cmyk({c}% {m}% {y}% {k}%{alpha})",
            c = rd100(cmyk.c),
            m = rd100(cmyk.m),
            y = rd100(cmyk.y),
            k = rd100(cmyk.k),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Format the color as a floating point RGB-representation string
    /// (`rgb(100%, 50%,  0%)`).
    pub fn to_rgb_float_string(&self) -> String {
        let rd100 = |v| { round_to(100.0 * v, 4) };
        let rgba = RGBA::<f64>::from(self);
        if self.alpha == 1.0 {
            format!("rgb({r}%, {g}%, {b}%)",
                r = rd100(rgba.r),
                g = rd100(rgba.g),
                b = rd100(rgba.b),
            )
        } else {
            format!("rgba({r}%, {g}%, {b}%, {alpha})",
                r = rd100(rgba.r),
                g = rd100(rgba.g),
                b = rd100(rgba.b),
                alpha = round_to(self.alpha, 4)
            )
        }
    }

    /// Format the color as a CSS `color()` string in the sRGB color space
    /// (`color(srgb 1 0.5 0)`).
    pub fn to_color_srgb_string(&self) -> String {
        let rd = |v| { round_to(v, 6) };
        let rgba = RGBA::<f64>::from(self);
        format!("color(srgb {r} {g} {b}{alpha})",
            r = rd(rgba.r),
            g = rd(rgba.g),
            b = rd(rgba.b),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Format the color as a floating point representation that can be parsed
    /// as an input color with minimal loss of precision.  This is used as the
    /// output format when piping color values to another `pastel` command.
    ///
    /// To avoid an unnecessary conversion step, we output the XYZ color values
    /// used in the common `Color` representation.  The format of an `f64` value
    /// can represent at least 15 significant digits, but we want to preserve
    /// at least one guard digit to compensate for rounding error, so we round
    /// all values to 14 significant digits.
    pub fn to_precise_input_string(&self) -> String {
        let rd = |v| { round_sig(v, 14) };

        let x = rd(self.x);
        let y = rd(self.y);
        let z = rd(self.z);
        if self.alpha == 1.0 {
            format!("xyz({},{},{})", x, y, z)
        } else {
            format!("xyz({},{},{},{})", x, y, z, rd(self.alpha))
        }
    }

    /// Format the color as a RGB-representation string (`#fc0070`).
    pub fn to_rgb_hex_string(&self, leading_hash: bool) -> String {
        let rgba = self.to_rgba();
        let hex = format!(
            "{}{:02x}{:02x}{:02x}",
            if leading_hash { "#" } else { "" },
            rgba.r,
            rgba.g,
            rgba.b
        );
        if self.alpha == 1.0 {
            hex
        } else {
            format!(
                "{h}{a:02x}",
                h = hex,
                a = (255.0 * self.alpha) as u8
            )
        }
    }

    /// Convert a `Color` to its red, green, blue and alpha values. All numbers are from the range
    /// between 0.0 and 1.0.
    pub fn to_rgba_float(&self) -> RGBA<Scalar> {
        RGBA::<f64>::from(self)
    }

    /// Return the color as an integer in RGB representation (`0xRRGGBB`)
    pub fn to_u32(&self) -> u32 {
        let rgba = self.to_rgba();
        u32::from(rgba.r).wrapping_shl(16) + u32::from(rgba.g).wrapping_shl(8) + u32::from(rgba.b)
    }

    /// Convert a `Color` to its linear sRGB component values.  Several operations require
    /// linear RGB values, and XYZ colors are already in linear form, so we can reduce
    /// error propagation by converting XYZ directly to linear RGB instead of converting to
    /// standard gamma-compressed sRGB and then converting back to linear form.
    pub fn to_linear_srgb(&self) -> RGBA<f64> {
        #![allow(clippy::many_single_char_names)]
        let r =
              3.240_969_941_904_522_60 * self.x
            - 1.537_383_177_570_094_00 * self.y
            - 0.498_610_760_293_003_40 * self.z;
        let g =
            - 0.969_243_636_280_879_60 * self.x
            + 1.875_967_501_507_720_20 * self.y
            + 0.041_555_057_407_175_59 * self.z;
        let b =
              0.055_630_079_696_993_66 * self.x
            - 0.203_976_958_888_976_52 * self.y
            + 1.056_971_514_242_878_60 * self.z;

        RGBA {
            r,
            g,
            b,
            alpha: self.alpha,
        }
    }

    /// Get XYZ coordinates according to the CIE 1931 color space.
    ///
    /// See:
    /// - https://en.wikipedia.org/wiki/CIE_1931_color_space
    /// - https://en.wikipedia.org/wiki/SRGB
    pub fn to_xyz(&self) -> XYZ {
        XYZ::from(self)
    }

    /// Format the color as a XYZ-representation string (`color(xyz 0.9504 1 1.0889)`).
    pub fn to_xyz_string(&self) -> String {
        let rd = |v| { round_to(v, 6) };
        let xyz = XYZ::from(self);
        format!(
            "color(xyz {x} {y} {z}{alpha})",
            x = rd(xyz.x),
            y = rd(xyz.y),
            z = rd(xyz.z),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Get coordinates according to the LSM color space
    ///
    /// See https://en.wikipedia.org/wiki/LMS_color_space for info on the color space as well as an
    /// algorithm for converting from CIE XYZ
    pub fn to_lms(&self) -> LMS {
        LMS::from(self)
    }

    /// Get L, a and b coordinates according to the Lab color space.
    ///
    /// See: https://en.wikipedia.org/wiki/Lab_color_space
    pub fn to_lab(&self) -> Lab {
        Lab::from(self)
    }

    /// Format the color as a Lab-representation string (`lab(41% 83 -93)`).
    pub fn to_lab_string(&self) -> String {
        let rd = |v| { round_to(v, 4) };
        let lab = Lab::from(self);
        format!(
            "lab({l}% {a} {b}{alpha})",
            l = rd(lab.l),
            a = rd(lab.a),
            b = rd(lab.b),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Get L, C and h coordinates according to the CIE LCh color space.
    ///
    /// See: https://en.wikipedia.org/wiki/Lab_color_space
    pub fn to_lch(&self) -> LCh {
        LCh::from(self)
    }

    /// Format the color as a LCh-representation string (`lch(80.7% 95.4 126)`).
    pub fn to_lch_string(&self) -> String {
        let lch = LCh::from(self);
        format!(
            "lch({l}% {c} {h}{alpha})",
            l = round_to(lch.l, 4),
            c = round_to(lch.c, 4),
            h = round_to(lch.h, 3),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Get L, u and v coordinates according to the CIE Luv color space.
    ///
    /// See: https://en.wikipedia.org/wiki/CIELUV
    pub fn to_luv(&self) -> Luv {
        Luv::from(self)
    }

    /// Format the color as a Luv-representation string (`luv(49% 67 10)`).
    pub fn to_luv_string(&self) -> String {
        let rd = |v| { round_to(v, 4) };
        let luv = Luv::from(self);
        format!(
            "luv({l}% {u} {v}{alpha})",
            l = rd(luv.l),
            u = rd(luv.u),
            v = rd(luv.v),
            alpha = format_css_alpha(self.alpha)
        )
    }

    /// Get L, C and h coordinates according to the CIE LCh(uv) color space.
    ///
    /// See: https://en.wikipedia.org/wiki/CIELUV#Cylindrical_representation_(CIELCh)
    pub fn to_lchuv(&self) -> LChuv {
        LChuv::from(self)
    }

    /// Format the color as a LCh(uv)-representation string (`lchuv(80.7% 95.4 126)`).
    pub fn to_lchuv_string(&self) -> String {
        let lch = LChuv::from(self);
        format!(
            "lchuv({l}% {c} {h}{alpha})",
            l = round_to(lch.l, 4),
            c = round_to(lch.c, 4),
            h = round_to(lch.h, 3),
            alpha = format_css_alpha(self.alpha)
        )
    }

    /// Format the color as a HCL-representation string (`hcl(126 95.4 80.7%)`).
    pub fn to_hcl_string(&self) -> String {
        let lch = LChuv::from(self);
        format!(
            "hcl({h} {c} {l}%{alpha})",
            l = round_to(lch.l, 4),
            c = round_to(lch.c, 4),
            h = round_to(lch.h, 3),
            alpha = format_css_alpha(self.alpha)
        )
    }

    /// Get L, a and b coordinates according to the OKLab color space.
    ///
    /// See: https://bottosson.github.io/posts/oklab/
    pub fn to_oklab(&self) -> OKLab {
        OKLab::from(self)
    }

    /// Format the color as an OKLab-representation string (`oklab(41% 83 -93)`).
    pub fn to_oklab_string(&self) -> String {
        let lab = OKLab::from(self);
        format!(
            "oklab({l}% {a} {b}{alpha})",
            l = round_to(lab.l * 100.0, 4),
            a = round_to(lab.a, 6),
            b = round_to(lab.b, 6),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Get L, C and h coordinates according to the OKLCh color space.
    ///
    /// See: https://en.wikipedia.org/wiki/Lab_color_space
    pub fn to_oklch(&self) -> OKLCh {
        OKLCh::from(self)
    }

    /// Format the color as an OKLCh-representation string (`oklch(80.7% 0.4 126)`).
    pub fn to_oklch_string(&self) -> String {
        let lch = OKLCh::from(self);
        format!(
            "oklch({l}% {c} {h}{alpha})",
            l = round_to(lch.l * 100.0, 4),
            c = round_to(lch.c, 6),
            h = round_to(lch.h, 4),
            alpha = format_css_alpha(self.alpha),
        )
    }

    /// Pure black.
    pub fn black() -> Color {
        Color::from_hsl(0.0, 0.0, 0.0)
    }

    /// Pure white.
    pub fn white() -> Color {
        Color::from_hsl(0.0, 0.0, 1.0)
    }

    /// Red (`#ff0000`)
    pub fn red() -> Color {
        Color::from_rgb(255, 0, 0)
    }

    /// Green (`#008000`)
    pub fn green() -> Color {
        Color::from_rgb(0, 128, 0)
    }

    /// Blue (`#0000ff`)
    pub fn blue() -> Color {
        Color::from_rgb(0, 0, 255)
    }

    /// Yellow (`#ffff00`)
    pub fn yellow() -> Color {
        Color::from_rgb(255, 255, 0)
    }

    /// Fuchsia (`#ff00ff`)
    pub fn fuchsia() -> Color {
        Color::from_rgb(255, 0, 255)
    }

    /// Aqua (`#00ffff`)
    pub fn aqua() -> Color {
        Color::from_rgb(0, 255, 255)
    }

    /// Lime (`#00ff00`)
    pub fn lime() -> Color {
        Color::from_rgb(0, 255, 0)
    }

    /// Maroon (`#800000`)
    pub fn maroon() -> Color {
        Color::from_rgb(128, 0, 0)
    }

    /// Olive (`#808000`)
    pub fn olive() -> Color {
        Color::from_rgb(128, 128, 0)
    }

    /// Navy (`#000080`)
    pub fn navy() -> Color {
        Color::from_rgb(0, 0, 128)
    }

    /// Purple (`#800080`)
    pub fn purple() -> Color {
        Color::from_rgb(128, 0, 128)
    }

    /// Teal (`#008080`)
    pub fn teal() -> Color {
        Color::from_rgb(0, 128, 128)
    }

    /// Silver (`#c0c0c0`)
    pub fn silver() -> Color {
        Color::from_rgb(192, 192, 192)
    }

    /// Gray (`#808080`)
    pub fn gray() -> Color {
        Color::from_rgb(128, 128, 128)
    }

    /// Create a gray tone from a lightness value (0.0 is black, 1.0 is white).
    pub fn graytone(lightness: Scalar) -> Color {
        Color::from_hsl(0.0, 0.0, lightness)
    }

    /// Rotate along the "hue" axis.
    pub fn rotate_hue(&self, delta: Scalar) -> Color {
        let hsl = HSLA::from(self);
        Self::from_hsla(
            hsl.h + delta,
            hsl.s,
            hsl.l,
            self.alpha,
        )
    }

    /// Get the complementary color (hue rotated by 180°).
    pub fn complementary(&self) -> Color {
        self.rotate_hue(180.0)
    }

    /// Lighten a color by adding a certain amount (number between -1.0 and 1.0) to the lightness
    /// channel. If the number is negative, the color is darkened.
    pub fn lighten(&self, f: Scalar) -> Color {
        let hsl = HSLA::from(self);
        Self::from_hsla(
            hsl.h,
            hsl.s,
            hsl.l + f,
            self.alpha,
        )
    }

    /// Darken a color by subtracting a certain amount (number between -1.0 and 1.0) from the
    /// lightness channel. If the number is negative, the color is lightened.
    pub fn darken(&self, f: Scalar) -> Color {
        self.lighten(-f)
    }

    /// Increase the saturation of a color by adding a certain amount (number between -1.0 and 1.0)
    /// to the saturation channel. If the number is negative, the color is desaturated.
    pub fn saturate(&self, f: Scalar) -> Color {
        let hsl = HSLA::from(self);
        Self::from_hsla(
            hsl.h,
            hsl.s + f,
            hsl.l,
            self.alpha,
        )
    }

    /// Decrease the saturation of a color by subtracting a certain amount (number between -1.0 and
    /// 1.0) from the saturation channel. If the number is negative, the color is saturated.
    pub fn desaturate(&self, f: Scalar) -> Color {
        self.saturate(-f)
    }

    /// Adjust the long-, medium-, and short-wavelength cone perception of a color to simulate what
    /// a colorblind person sees. Since there are multiple kinds of colorblindness, the desired
    /// kind must be specified in `cb_ty`.
    pub fn simulate_colorblindness(&self, cb_ty: ColorblindnessType) -> Color {
        // Coefficients here are taken from
        // https://ixora.io/projects/colorblindness/color-blindness-simulation-research/
        let (l, m, s, alpha) = match cb_ty {
            ColorblindnessType::Protanopia => {
                let LMS { m, s, alpha, .. } = self.to_lms();
                let l = 1.051_182_94 * m - 0.051_160_99 * s;
                (l, m, s, alpha)
            }
            ColorblindnessType::Deuteranopia => {
                let LMS { l, s, alpha, .. } = self.to_lms();
                let m = 0.951_309_2 * l + 0.048_669_92 * s;
                (l, m, s, alpha)
            }
            ColorblindnessType::Tritanopia => {
                let LMS { l, m, alpha, .. } = self.to_lms();
                let s = -0.867_447_36 * l + 1.867_270_89 * m;
                (l, m, s, alpha)
            }
        };

        Color::from_lms(l, m, s, alpha)
    }

    /// Convert a color to a gray tone with the same perceived luminance (see `luminance`).
    pub fn to_gray(&self) -> Color {
        let c = self.to_lch();

        // the desaturation step is only needed to correct minor rounding errors.
        Color::from_lch(c.l, 0.0, 0.0, 1.0).desaturate(1.0)
    }

    /// The percieved brightness of the color (A number between 0.0 and 1.0).
    ///
    /// See: https://www.w3.org/TR/AERT#color-contrast
    pub fn brightness(&self) -> Scalar {
        let c = self.to_rgba_float();
        (299.0 * c.r + 587.0 * c.g + 114.0 * c.b) / 1000.0
    }

    /// Determine whether a color is perceived as a light color (perceived brightness is larger
    /// than 0.5).
    pub fn is_light(&self) -> bool {
        self.brightness() > 0.5
    }

    /// The relative brightness of a color (normalized to 0.0 for darkest black
    /// and 1.0 for lightest white), according to the WCAG definition.
    ///
    /// See: https://www.w3.org/TR/2008/REC-WCAG20-20081211/#relativeluminancedef
    pub fn luminance(&self) -> Scalar {
        let c = self.to_linear_srgb();
        0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
    }

    /// Contrast ratio between two colors as defined by the WCAG. The ratio can range from 1.0
    /// to 21.0. Two colors with a contrast ratio of 4.5 or higher can be used as text color and
    /// background color and should be well readable.
    ///
    /// https://www.w3.org/TR/2008/REC-WCAG20-20081211/#contrast-ratiodef
    pub fn contrast_ratio(&self, other: &Color) -> Scalar {
        let l_self = self.luminance();
        let l_other = other.luminance();

        if l_self > l_other {
            (l_self + 0.05) / (l_other + 0.05)
        } else {
            (l_other + 0.05) / (l_self + 0.05)
        }
    }

    /// Return a readable foreground text color (either `black` or `white`) for a
    /// given background color.
    pub fn text_color(&self) -> Color {
        // This threshold can be easily computed by solving
        //
        //   contrast(L_threshold, L_black) == contrast(L_threshold, L_white)
        //
        // where contrast(.., ..) is the color contrast as defined by the WCAG (see above)
        const THRESHOLD: Scalar = 0.179;

        if self.luminance() > THRESHOLD {
            Color::black()
        } else {
            Color::white()
        }
    }

    /// Compute the perceived 'distance' between two colors according to the CIE76 delta-E
    /// standard. A distance below ~2.3 is not noticable.
    ///
    /// See: https://en.wikipedia.org/wiki/Color_difference
    pub fn distance_delta_e_cie76(&self, other: &Color) -> Scalar {
        delta_e::cie76(&self.to_lab(), &other.to_lab())
    }

    /// Compute the perceived 'distance' between two colors according to the CIEDE2000 delta-E
    /// standard.
    ///
    /// See: https://en.wikipedia.org/wiki/Color_difference
    pub fn distance_delta_e_ciede2000(&self, other: &Color) -> Scalar {
        delta_e::ciede2000(&self.to_lab(), &other.to_lab())
    }

    /// Mix two colors by linearly interpolating between them in the specified color space.
    /// For the angle-like components (hue), the shortest path along the unit circle is chosen.
    pub fn mix<C: ColorSpace>(self: &Color, other: &Color, fraction: Fraction) -> Color {
        C::from_color(self)
            .mix(&C::from_color(other), fraction)
            .into_color()
    }
}

// by default Colors will be printed into HSLA fromat
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", HSLA::from(self).to_string())
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Color components must be equal within the precision of a 16-bit
        // integer channel value (1.0 / 65536.0), which is about 5 decimal
        // places.  Showing 6 decimal places for debugging ensures that the
        // displayed values will always be different if two colors don't
        // compare as equal.
        let rd = |v| { round_to(v, 6) };

        let x = rd(self.x);
        let y = rd(self.y);
        let z = rd(self.z);
        if self.alpha == 1.0 {
            write!(f, "Color::from_xyz({}, {}, {})", x, y, z)
        } else {
            write!(f,
                "Color::from_xyza({}, {}, {}, {})",
                x, y, z, rd(self.alpha)
            )
        }
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Color) -> bool {
        use approx::abs_diff_eq;

        // Check if components are equal within the precision of a 16-bit
        // channel value.
        let eq = |x, y| { abs_diff_eq!(x, y, epsilon = 1.0 / 65536.0) };
        eq(self.x, other.x) &&
        eq(self.y, other.y) &&
        eq(self.z, other.z) &&
        eq(self.alpha, other.alpha)
    }
}

impl From<&HSLA> for Color {
    fn from(color: &HSLA) -> Self {
        let h_s = mod_positive(color.h, 360.0) / 60.0;
        let l = clamp(0.0, 1.0, color.l);
        let s = clamp(0.0, 1.0, color.s);
        let chr = (1.0 - Scalar::abs(2.0 * l - 1.0)) * s;
        let m = l - chr / 2.0;
        let x = chr * (1.0 - Scalar::abs(h_s % 2.0 - 1.0));

        #[allow(clippy::upper_case_acronyms)]
        struct RGB(Scalar, Scalar, Scalar);

        let col = if h_s < 1.0 {
            RGB(chr, x, 0.0)
        } else if (1.0..2.0).contains(&h_s) {
            RGB(x, chr, 0.0)
        } else if (2.0..3.0).contains(&h_s) {
            RGB(0.0, chr, x)
        } else if (3.0..4.0).contains(&h_s) {
            RGB(0.0, x, chr)
        } else if (4.0..5.0).contains(&h_s) {
            RGB(x, 0.0, chr)
        } else {
            RGB(chr, 0.0, x)
        };

        Self::from(&RGBA {
            r: col.0 + m,
            g: col.1 + m,
            b: col.2 + m,
            alpha: color.alpha,
        })
    }
}

impl From<&RGBA<u8>> for Color {
    fn from(color: &RGBA<u8>) -> Self {
        let r = Scalar::from(color.r) / 255.0;
        let g = Scalar::from(color.g) / 255.0;
        let b = Scalar::from(color.b) / 255.0;

        Self::from(&RGBA::<f64> {
            r,
            g,
            b,
            alpha: color.alpha,
        })
    }
}

impl From<&RGBA<f64>> for Color {
    fn from(color: &RGBA<f64>) -> Self {
        #![allow(clippy::many_single_char_names)]
        let finv = |c_: f64| {
            let sign = Scalar::signum(c_);
            let abs = Scalar::abs(c_);
            if abs <= 0.04045 {
                c_ / 12.92
            } else {
                sign * Scalar::powf((abs + 0.055) / 1.055, 2.4)
            }
        };

        let r = finv(color.r);
        let g = finv(color.g);
        let b = finv(color.b);

        let x =
            0.412_390_799_265_959_34 * r +
            0.357_584_339_383_878_00 * g +
            0.180_480_788_401_834_30 * b;
        let y =
            0.212_639_005_871_510_27 * r +
            0.715_168_678_767_756_00 * g +
            0.072_192_315_360_733_71 * b;
        let z =
            0.019_330_818_715_591_82 * r +
            0.119_194_779_794_625_98 * g +
            0.950_532_152_249_660_70 * b;

        Self::from(&XYZ {
            x,
            y,
            z,
            alpha: color.alpha,
        })
    }
}

impl From<&HSVA> for Color {
    fn from(color: &HSVA) -> Self {
        let sv = clamp(0.0, 1.0, color.s);
        let v = clamp(0.0, 1.0, color.v);
        let l = v * (1.0 - sv / 2.0);
        let sl = if l == 0.0 || l == 1.0 { 0.0 } else { (v - l) / f64::min(l, 1.0 - l) };
        Self::from(&HSLA {
            h: color.h,
            s: sl,
            l,
            alpha: color.alpha,
        })
    }
}

impl From<&HWBA> for Color {
    fn from(color: &HWBA) -> Self {
        if color.w + color.b > 1.0 {
            let gray = color.w / (color.w + color.b);
            Self::from_rgba_float(gray, gray, gray, color.alpha)
        } else {
            let w = clamp(0.0, 1.0, color.w);
            let b = clamp(0.0, 1.0, color.b);
            let v = 1.0 - b;
            let s = 1.0 - (w / v);
            Self::from(&HSVA {
                h: color.h,
                s,
                v,
                alpha: color.alpha,
            })
        }
    }
}

impl From<&XYZ> for Color {
    fn from(color: &XYZ) -> Self {
        Color {
            x: color.x,
            y: color.y,
            z: color.z,
            alpha: color.alpha,
        }
    }
}

impl From<&LMS> for Color {
    fn from(color: &LMS) -> Self {
        #![allow(clippy::many_single_char_names)]
        let x = 1.91020 * color.l - 1.112_120 * color.m + 0.201_908 * color.s;
        let y = 0.37095 * color.l + 0.629_054 * color.m + 0.000_000 * color.s;
        let z = 0.00000 * color.l + 0.000_000 * color.m + 1.000_000 * color.s;
        Self::from(&XYZ {
            x,
            y,
            z,
            alpha: color.alpha,
        })
    }
}

impl From<&Lab> for Color {
    fn from(color: &Lab) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DELTA: Scalar = 6.0 / 29.0;

        let finv = |t| {
            if t > DELTA {
                Scalar::powf(t, 3.0)
            } else {
                3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
            }
        };

        let l_ = (color.l + 16.0) / 116.0;
        let x = D65_XN * finv(l_ + color.a / 500.0);
        let y = D65_YN * finv(l_);
        let z = D65_ZN * finv(l_ - color.b / 200.0);

        Self::from(&XYZ {
            x,
            y,
            z,
            alpha: color.alpha,
        })
    }
}

impl From<&LCh> for Color {
    fn from(color: &LCh) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DEG2RAD: Scalar = std::f64::consts::PI / 180.0;

        // Clamp negative chroma to 0.
        let c = f64::max(color.c, 0.0);
        let a = c * Scalar::cos(color.h * DEG2RAD);
        let b = c * Scalar::sin(color.h * DEG2RAD);

        Self::from(&Lab {
            l: color.l,
            a,
            b,
            alpha: color.alpha,
        })
    }
}

impl From<&Luv> for Color {
    fn from(color: &Luv) -> Self {
        #![allow(clippy::many_single_char_names)]
        let uprime = |x, y, z| {
            4.0 * x / (x + 15.0 * y + 3.0 * z)
        };
        let vprime = |x, y, z| {
            9.0 * y / (x + 15.0 * y + 3.0 * z)
        };

        let u_ = color.u / (13.0 * color.l) + uprime(D65_XN, D65_YN, D65_ZN);
        let v_ = color.v / (13.0 * color.l) + vprime(D65_XN, D65_YN, D65_ZN);

        let y = if color.l > 8.0 {
            D65_YN * Scalar::powf((color.l + 16.0) / 116.0, 3.0)
        } else {
            D65_YN * color.l * Scalar::powf(3.0 / 29.0, 3.0)
        };
        let x = y * (9.0 * u_) / (4.0 * v_);
        let z = y * (12.0 - 3.0 * u_ - 20.0 * v_) / (4.0 * v_);

        Self::from(&XYZ {
            x,
            y,
            z,
            alpha: color.alpha,
        })
    }
}

impl From<&LChuv> for Color {
    fn from(color: &LChuv) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DEG2RAD: Scalar = std::f64::consts::PI / 180.0;

        // Clamp negative chroma to 0.
        let c = f64::max(color.c, 0.0);
        let u = c * Scalar::cos(color.h * DEG2RAD);
        let v = c * Scalar::sin(color.h * DEG2RAD);

        Self::from(&Luv {
            l: color.l,
            u,
            v,
            alpha: color.alpha,
        })
    }
}

impl From<&OKLab> for Color {
    fn from(color: &OKLab) -> Self {
        // Given OKLab, convert to XYZ relative to D65
        #![allow(clippy::many_single_char_names)]
        let l_ =
            0.999_999_998_450_519_814_32 * color.l +
            0.396_337_792_173_767_856_78 * color.a +
            0.215_803_758_060_758_803_39 * color.b;
        let m_ =
            1.000_000_008_881_760_776_70 * color.l -
            0.105_561_342_323_656_349_40 * color.a -
            0.063_854_174_771_705_903_402 * color.b;
        let s_ =
            1.000_000_054_672_410_917_70 * color.l -
            0.089_484_182_094_965_759_684 * color.a -
            1.291_485_537_864_091_739_90 * color.b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        Self::from(&XYZ {
            x:  1.226_879_873_374_155_70 * l - 0.557_814_996_555_481_30 * m + 0.281_391_050_177_215_83 * s,
            y: -0.040_575_762_624_313_72 * l + 1.112_286_829_397_059_40 * m - 0.071_711_066_661_517_01 * s,
            z: -0.076_372_949_746_721_42 * l - 0.421_493_323_962_791_40 * m + 1.586_924_024_427_241_80 * s,
            alpha: color.alpha,
        })
    }
}

impl From<&OKLCh> for Color {
    fn from(color: &OKLCh) -> Self {
        #![allow(clippy::many_single_char_names)]
        const DEG2RAD: Scalar = std::f64::consts::PI / 180.0;

        // Clamp negative chroma to 0.
        let c = f64::max(color.c, 0.0);
        let a = c * Scalar::cos(color.h * DEG2RAD);
        let b = c * Scalar::sin(color.h * DEG2RAD);

        Self::from(&OKLab {
            l: color.l,
            a,
            b,
            alpha: color.alpha,
        })
    }
}

// from CMYK to Color so you can do -> let new_color = Color::from(&some_cmyk);
impl From<&CMYK> for Color {
    fn from(color: &CMYK) -> Self {
        #![allow(clippy::many_single_char_names)]
        let r = 255.0 * ((1.0 - color.c) / 100.0) * ((1.0 - color.k) / 100.0);
        let g = 255.0 * ((1.0 - color.m) / 100.0) * ((1.0 - color.k) / 100.0);
        let b = 255.0 * ((1.0 - color.y) / 100.0) * ((1.0 - color.k) / 100.0);

        Color::from(&RGBA::<f64> {
            r,
            g,
            b,
            alpha: 1.0,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RGBA<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub alpha: Scalar,
}

impl ColorSpace for RGBA<f64> {
    fn from_color(c: &Color) -> Self {
        c.to_rgba_float()
    }

    fn into_color(self) -> Color {
        Color::from_rgba_float(self.r, self.g, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            r: interpolate(self.r, other.r, fraction),
            g: interpolate(self.g, other.g, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for RGBA<f64> {
    fn from(color: &Color) -> Self {
        #![allow(clippy::many_single_char_names)]
        let f = |c| {
            let sign = Scalar::signum(c);
            let abs = Scalar::abs(c);
            if abs <= 0.003_130_8 {
                12.92 * c
            } else {
                sign * (1.055 * Scalar::powf(abs, 1.0 / 2.4) - 0.055)
            }
        };

        let lin_rgb = color.to_linear_srgb();
        let r = f(lin_rgb.r);
        let g = f(lin_rgb.g);
        let b = f(lin_rgb.b);

        RGBA {
            r,
            g,
            b,
            alpha: color.alpha,
        }
    }
}

impl From<&Color> for RGBA<u8> {
    fn from(color: &Color) -> Self {
        let c = RGBA::<f64>::from(color);
        let r = Scalar::round(clamp(0.0, 255.0, 255.0 * c.r)) as u8;
        let g = Scalar::round(clamp(0.0, 255.0, 255.0 * c.g)) as u8;
        let b = Scalar::round(clamp(0.0, 255.0, 255.0 * c.b)) as u8;

        RGBA {
            r,
            g,
            b,
            alpha: color.alpha,
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct HSLA {
    pub h: Scalar,
    pub s: Scalar,
    pub l: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for HSLA {
    fn from_color(c: &Color) -> Self {
        c.to_hsla()
    }

    fn into_color(self) -> Color {
        Color::from_hsla(self.h, self.s, self.l, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
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

impl From<&Color> for HSLA {
    fn from(color: &Color) -> Self {
        use approx::relative_eq;
        let is_zero = |v| { relative_eq!(v, 0.0, epsilon = 1.0e-15) };

        let c = RGBA::<f64>::from(color);
        let r_s = clamp(0.0, 1.0, c.r);
        let g_s = clamp(0.0, 1.0, c.g);
        let b_s = clamp(0.0, 1.0, c.b);

        let max_chroma = f64::max(f64::max(r_s, g_s), b_s);
        let min_chroma = f64::min(f64::min(r_s, g_s), b_s);

        let chroma_s = max_chroma - min_chroma;

        let hue = 60.0
            * (if is_zero(chroma_s) {
                0.0
            } else if r_s == max_chroma {
                mod_positive((g_s - b_s) / chroma_s, 6.0)
            } else if g_s == max_chroma {
                (b_s - r_s) / chroma_s + 2.0
            } else {
                (r_s - g_s) / chroma_s + 4.0
            });

        let lightness = (max_chroma + min_chroma) / 2.0;
        let saturation = if is_zero(chroma_s) {
            0.0
        } else {
            chroma_s / (1.0 - Scalar::abs(2.0 * lightness - 1.0))
        };
        HSLA {
            h: hue,
            s: saturation,
            l: lightness,
            alpha: color.alpha,
        }
    }
}

impl fmt::Display for HSLA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsl({h}, {s}, {l})", h = self.h, s = self.s, l = self.l,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HSVA {
    pub h: Scalar,
    pub s: Scalar,
    pub v: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for HSVA {
    fn from_color(c: &Color) -> Self {
        c.to_hsva()
    }

    fn into_color(self) -> Color {
        Color::from_hsva(self.h, self.s, self.v, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.s < 0.0001 { other.h } else { self.h };
        let other_hue = if other.s < 0.0001 { self.h } else { other.h };

        Self {
            h: interpolate_angle(self_hue, other_hue, fraction),
            s: interpolate(self.s, other.s, fraction),
            v: interpolate(self.v, other.v, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for HSVA {
    fn from(color: &Color) -> Self {
        #![allow(clippy::many_single_char_names)]
        let HSLA { h, s, l, alpha } = HSLA::from(color);
        let v = l + s * f64::min(l, 1.0 - l);
        let sv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
        HSVA {
            h,
            s: sv,
            v,
            alpha,
        }
    }
}

impl fmt::Display for HSVA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsv({h}, {s}, {v})", h = self.h, s = self.s, v = self.v,)
    }
}

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
        let self_hue = if (self.w + self.b) >= 1.0 { other.h } else { self.h };
        let other_hue = if (other.w + other.b) >= 1.0 { self.h } else { other.h };

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
        #![allow(clippy::many_single_char_names)]
        let HSVA { h, s, v, alpha } = HSVA::from(color);

        let w = (1.0 - s) * v;
        let b = 1.0 - v;
        HWBA {
            h,
            w,
            b,
            alpha,
        }
    }
}

impl fmt::Display for HWBA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hwb({h}, {w}, {b})", h = self.h, w = self.w, b = self.b,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct XYZ {
    pub x: Scalar,
    pub y: Scalar,
    pub z: Scalar,
    pub alpha: Scalar,
}

impl From<&Color> for XYZ {
    fn from(color: &Color) -> Self {
        XYZ {
            x: color.x,
            y: color.y,
            z: color.z,
            alpha: color.alpha
        }
    }
}

impl fmt::Display for XYZ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XYZ({x}, {y}, {z})", x = self.x, y = self.y, z = self.z,)
    }
}

/// A color space whose axes correspond to the responsivity spectra of the long-, medium-, and
/// short-wavelength cone cells in the human eye. More info
/// [here](https://en.wikipedia.org/wiki/LMS_color_space).
#[derive(Debug, Clone, PartialEq)]
pub struct LMS {
    pub l: Scalar,
    pub m: Scalar,
    pub s: Scalar,
    pub alpha: Scalar,
}

impl From<&Color> for LMS {
    fn from(color: &Color) -> Self {
        let XYZ { x, y, z, alpha } = XYZ::from(color);
        let l = 0.38971 * x + 0.68898 * y - 0.07868 * z;
        let m = -0.22981 * x + 1.18340 * y + 0.04641 * z;
        let s = 0.00000 * x + 0.00000 * y + 1.00000 * z;

        LMS { l, m, s, alpha }
    }
}

impl fmt::Display for LMS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LMS({l}, {m}, {s})", l = self.l, m = self.m, s = self.s,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lab {
    pub l: Scalar,
    pub a: Scalar,
    pub b: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for Lab {
    fn from_color(c: &Color) -> Self {
        c.to_lab()
    }

    fn into_color(self) -> Color {
        Color::from_lab(self.l, self.a, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            l: interpolate(self.l, other.l, fraction),
            a: interpolate(self.a, other.a, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for Lab {
    fn from(color: &Color) -> Self {
        let rec = XYZ::from(color);

        let cut = Scalar::powf(6.0 / 29.0, 3.0);
        let f = |t| {
            if t > cut {
                Scalar::powf(t, 1.0 / 3.0)
            } else {
                (1.0 / 3.0) * Scalar::powf(29.0 / 6.0, 2.0) * t + 4.0 / 29.0
            }
        };

        let fy = f(rec.y / D65_YN);

        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (f(rec.x / D65_XN) - fy);
        let b = 200.0 * (fy - f(rec.z / D65_ZN));

        Lab {
            l,
            a,
            b,
            alpha: color.alpha,
        }
    }
}

impl fmt::Display for Lab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lab({l}, {a}, {b})", l = self.l, a = self.a, b = self.b,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LCh {
    pub l: Scalar,
    pub c: Scalar,
    pub h: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for LCh {
    fn from_color(c: &Color) -> Self {
        c.to_lch()
    }

    fn into_color(self) -> Color {
        Color::from_lch(self.l, self.c, self.h, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.c < 0.1 { other.h } else { self.h };
        let other_hue = if other.c < 0.1 { self.h } else { other.h };

        Self {
            l: interpolate(self.l, other.l, fraction),
            c: interpolate(self.c, other.c, fraction),
            h: interpolate_angle(self_hue, other_hue, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for LCh {
    fn from(color: &Color) -> Self {
        let Lab { l, a, b, alpha } = Lab::from(color);

        const RAD2DEG: Scalar = 180.0 / std::f64::consts::PI;

        let c = Scalar::sqrt(a * a + b * b);
        let h = mod_positive(Scalar::atan2(b, a) * RAD2DEG, 360.0);

        LCh { l, c, h, alpha }
    }
}

impl fmt::Display for LCh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LCh({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Luv {
    pub l: Scalar,
    pub u: Scalar,
    pub v: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for Luv {
    fn from_color(c: &Color) -> Self {
        c.to_luv()
    }

    fn into_color(self) -> Color {
        Color::from_luv(self.l, self.u, self.v, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            l: interpolate(self.l, other.l, fraction),
            u: interpolate(self.u, other.u, fraction),
            v: interpolate(self.v, other.v, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for Luv {
    fn from(color: &Color) -> Self {
        let rec = XYZ::from(color);

        let yr = rec.y / D65_YN;

        let cut = Scalar::powf(6.0 / 29.0, 3.0);
        let l = if yr > cut {
            116.0 * Scalar::powf(yr, 1.0 / 3.0) - 16.0
        } else {
            Scalar::powf(29.0 / 3.0, 3.0) * yr
        };

        let uprime = |x, y, z| {
            4.0 * x / (x + 15.0 * y + 3.0 * z)
        };
        let vprime = |x, y, z| {
            9.0 * y / (x + 15.0 * y + 3.0 * z)
        };

        let u = 13.0 * l *
                (uprime(rec.x, rec.y, rec.z) - uprime(D65_XN, D65_YN, D65_ZN));
        let v = 13.0 * l *
                (vprime(rec.x, rec.y, rec.z) - vprime(D65_XN, D65_YN, D65_ZN));

        Luv {
            l,
            u,
            v,
            alpha: color.alpha,
        }
    }
}

impl fmt::Display for Luv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Luv({l}, {u}, {v})", l = self.l, u = self.u, v = self.v,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LChuv {
    pub l: Scalar,
    pub c: Scalar,
    pub h: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for LChuv {
    fn from_color(c: &Color) -> Self {
        c.to_lchuv()
    }

    fn into_color(self) -> Color {
        Color::from_lchuv(self.l, self.c, self.h, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.c < 0.1 { other.h } else { self.h };
        let other_hue = if other.c < 0.1 { self.h } else { other.h };

        Self {
            l: interpolate(self.l, other.l, fraction),
            c: interpolate(self.c, other.c, fraction),
            h: interpolate_angle(self_hue, other_hue, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for LChuv {
    fn from(color: &Color) -> Self {
        let Luv { l, u, v, alpha } = Luv::from(color);

        const RAD2DEG: Scalar = 180.0 / std::f64::consts::PI;

        let c = Scalar::sqrt(u * u + v * v);
        let h = mod_positive(Scalar::atan2(v, u) * RAD2DEG, 360.0);

        LChuv { l, c, h, alpha }
    }
}

impl fmt::Display for LChuv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LChuv({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OKLab {
    pub l: Scalar,
    pub a: Scalar,
    pub b: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for OKLab {
    fn from_color(c: &Color) -> Self {
        c.to_oklab()
    }

    fn into_color(self) -> Color {
        Color::from_oklaba(self.l, self.a, self.b, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        Self {
            l: interpolate(self.l, other.l, fraction),
            a: interpolate(self.a, other.a, fraction),
            b: interpolate(self.b, other.b, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for OKLab {
    fn from(color: &Color) -> Self {
        // Given XYZ relative to D65, convert to OKLab
        #![allow(clippy::many_single_char_names)]
        let rec = XYZ::from(color);

        let l =
            0.819_022_443_216_431_90 * rec.x +
            0.361_906_256_280_122_10 * rec.y -
            0.128_873_782_612_164_14 * rec.z;
        let m =
            0.032_983_667_198_027_10 * rec.x +
            0.929_286_846_896_554_60 * rec.y +
            0.036_144_668_169_998_44 * rec.z;
        let s =
            0.048_177_199_566_046_255 * rec.x +
            0.264_239_524_944_227_64 * rec.y +
            0.633_547_825_813_693_70 * rec.z;

        let l_ = f64::cbrt(l);
        let m_ = f64::cbrt(m);
        let s_ = f64::cbrt(s);

        // L in range [0,1]. For use in CSS, multiply by 100 and add a percent
        OKLab {
            l: 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_,
            a: 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_,
            b: 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_,
            alpha: color.alpha,
        }
    }
}

impl fmt::Display for OKLab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OKLab({l}, {a}, {b})", l = self.l, a = self.a, b = self.b,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OKLCh {
    pub l: Scalar,
    pub c: Scalar,
    pub h: Scalar,
    pub alpha: Scalar,
}

impl ColorSpace for OKLCh {
    fn from_color(c: &Color) -> Self {
        c.to_oklch()
    }

    fn into_color(self) -> Color {
        Color::from_oklcha(self.l, self.c, self.h, self.alpha)
    }

    fn mix(&self, other: &Self, fraction: Fraction) -> Self {
        // make sure that the hue is preserved when mixing with gray colors
        let self_hue = if self.c < 0.1 { other.h } else { self.h };
        let other_hue = if other.c < 0.1 { self.h } else { other.h };

        Self {
            l: interpolate(self.l, other.l, fraction),
            c: interpolate(self.c, other.c, fraction),
            h: interpolate_angle(self_hue, other_hue, fraction),
            alpha: interpolate(self.alpha, other.alpha, fraction),
        }
    }
}

impl From<&Color> for OKLCh {
    fn from(color: &Color) -> Self {
        let OKLab { l, a, b, alpha } = OKLab::from(color);

        const RAD2DEG: Scalar = 180.0 / std::f64::consts::PI;

        let c = Scalar::sqrt(a * a + b * b);
        let h = mod_positive(Scalar::atan2(b, a) * RAD2DEG, 360.0);

        OKLCh { l, c, h, alpha }
    }
}

impl fmt::Display for OKLCh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OKLCh({l}, {c}, {h})", l = self.l, c = self.c, h = self.h,)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMYK {
    pub c: Scalar,
    pub m: Scalar,
    pub y: Scalar,
    pub k: Scalar,
}

impl From<&Color> for CMYK {
    fn from(color: &Color) -> Self {
        let rgba = RGBA::<f64>::from(color);
        let r = clamp(0.0, 1.0, rgba.r);
        let g = clamp(0.0, 1.0, rgba.g);
        let b = clamp(0.0, 1.0, rgba.b);
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

/// A representation of the different kinds of colorblindness. More info
/// [here](https://en.wikipedia.org/wiki/Color_blindness).
pub enum ColorblindnessType {
    /// Protanopic people lack red cones
    Protanopia,
    /// Deuteranopic people lack green cones
    Deuteranopia,
    /// Tritanopic people lack blue cones
    Tritanopia,
}

/// The representation of a color stop for a `ColorScale`.
/// The position defines where the color is placed from left (0.0) to right (1.0).
#[derive(Debug, Clone)]
struct ColorStop {
    color: Color,
    position: Fraction,
}

/// The representation of a color scale.
/// The first `ColorStop` (position 0.0) defines the left end color.
/// The last `ColorStop` (position 1.0) defines the right end color.
#[derive(Debug, Clone)]
pub struct ColorScale {
    color_stops: Vec<ColorStop>,
}

impl ColorScale {
    /// Create an empty `ColorScale`.
    pub fn empty() -> Self {
        Self {
            color_stops: Vec::new(),
        }
    }

    /// Add a `Color` at the given position.
    pub fn add_stop(&mut self, color: Color, position: Fraction) -> &mut Self {
        #![allow(clippy::float_cmp)]
        let same_position = self
            .color_stops
            .iter_mut()
            .find(|c| position.value() == c.position.value());

        match same_position {
            Some(color_stop) => color_stop.color = color,
            None => {
                let next_index = self
                    .color_stops
                    .iter()
                    .position(|c| position.value() < c.position.value());

                let index = next_index.unwrap_or_else(|| self.color_stops.len());

                let color_stop = ColorStop { color, position };

                self.color_stops.insert(index, color_stop);
            }
        };

        self
    }

    /// Get the color at the given position using the mixing function.
    ///
    /// Note:
    /// - No color is returned if position isn't between two color stops or the `ColorScale` is empty.
    pub fn sample(
        &self,
        position: Fraction,
        mix: &dyn Fn(&Color, &Color, Fraction) -> Color,
    ) -> Option<Color> {
        if self.color_stops.len() < 2 {
            return None;
        }

        let left_stop = self
            .color_stops
            .iter()
            .rev()
            .find(|c| position.value() >= c.position.value());

        let right_stop = self
            .color_stops
            .iter()
            .find(|c| position.value() <= c.position.value());

        match (left_stop, right_stop) {
            (Some(left_stop), Some(right_stop)) => {
                let diff_color_stops = right_stop.position.value() - left_stop.position.value();
                let diff_position = position.value() - left_stop.position.value();
                let local_position = Fraction::from(diff_position / diff_color_stops);

                let color = mix(&left_stop.color, &right_stop.color, local_position);

                Some(color)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn color_partial_eq() {
        assert_eq!(
            Color::from_hsl(120.0, 0.3, 0.5),
            Color::from_hsl(360.0 + 120.0, 0.3, 0.5),
        );
        assert_eq!(
            Color::from_rgba(1, 2, 3, 0.3),
            Color::from_rgba(1, 2, 3, 0.3),
        );
        assert_eq!(Color::black(), Color::from_hsl(123.0, 0.3, 0.0));
        assert_eq!(Color::white(), Color::from_hsl(123.0, 0.3, 1.0));
        assert_eq!(
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.300_000_001, 0.500_000_001, 0.700_000_001),
        );

        assert_ne!(
            Color::from_hsl(120.0, 0.3, 0.5),
            Color::from_hsl(122.0, 0.3, 0.5),
        );
        assert_ne!(
            Color::from_hsl(120.0, 0.3, 0.5),
            Color::from_hsl(120.0, 0.32, 0.5),
        );
        assert_ne!(
            Color::from_hsl(120.0, 0.3, 0.5),
            Color::from_hsl(120.0, 0.3, 0.52),
        );
        assert_ne!(
            Color::from_hsla(120.0, 0.3, 0.5, 0.9),
            Color::from_hsla(120.0, 0.3, 0.5, 0.901),
        );
        assert_ne!(
            Color::from_rgba(1, 2, 3, 0.3),
            Color::from_rgba(2, 2, 3, 0.3),
        );
        assert_ne!(
            Color::from_rgba(1, 2, 3, 0.3),
            Color::from_rgba(1, 3, 3, 0.3),
        );
        assert_ne!(
            Color::from_rgba(1, 2, 3, 0.3),
            Color::from_rgba(1, 2, 4, 0.3),
        );
        assert_ne!(
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.300_1, 0.5, 0.7),
        );
        assert_eq!(  // equal within 16-bit precision
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.300_001, 0.5, 0.7),
        );
        assert_ne!(
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.3, 0.500_1, 0.7),
        );
        assert_ne!(
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.3, 0.5, 0.700_1),
        );
        assert_ne!(
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.3, 0.499_9, 0.7),
        );
        assert_eq!(  // equal within 16-bit precision
            Color::from_rgb_float(0.3, 0.5, 0.7),
            Color::from_rgb_float(0.3, 0.499_999, 0.7),
        );
    }

    #[test]
    fn rgb_conversion() {
        let rgb128 = 128.0 / 255.0;

        assert_eq!(Color::white(), Color::from_rgb_float(1.0, 1.0, 1.0));
        assert_eq!(Color::gray(), Color::from_rgb_float(rgb128, rgb128, rgb128));
        assert_eq!(Color::black(), Color::from_rgb_float(0.0, 0.0, 0.0));
        assert_eq!(Color::red(), Color::from_rgb_float(1.0, 0.0, 0.0));
        assert_eq!(
            Color::from_hsl(60.0, 1.0, 0.375),
            Color::from_rgb_float(0.75, 0.75, 0.0)
        ); //yellow-green
        assert_eq!(Color::green(), Color::from_rgb_float(0.0, rgb128, 0.0));
        assert_eq!(
            Color::from_hsl(240.0, 1.0, 0.75),
            Color::from_rgb_float(0.5, 0.5, 1.0)
        ); // blue-ish
        assert_eq!(
            Color::from_hsl(49.5, 0.893, 0.497),
            Color::from_rgb_float(0.94082, 0.78548, 0.05318)
        ); // yellow
        assert_eq!(
            Color::from_hsl(162.4, 0.779, 0.447),
            Color::from_rgb_float(0.09879, 0.79521, 0.59093)
        ); // cyan 2
    }

    #[test]
    fn rgb_hsl_roundtrip_conversion() {
        let roundtrip = |h, s, l| {
            // generate a color from an HSL value, which is stored as RGB
            let color1 = Color::from_hsl(h, s, l);
            // convert the stored RGB color to HSL
            let hsl = color1.to_hsla();
            // convert the calculated HSL back to a Color value and compare
            // it to the original HSL color
            let color2 = Color::from(&hsl);
            assert_eq!(color1, color2);
        };

        roundtrip(0.0, 0.0, 1.0);
        roundtrip(0.0, 0.0, 0.5);
        roundtrip(0.0, 0.0, 0.0);
        roundtrip(60.0, 1.0, 0.375);
        roundtrip(120.0, 1.0, 0.25);
        roundtrip(240.0, 1.0, 0.75);
        roundtrip(49.5, 0.893, 0.497);
        roundtrip(162.4, 0.779, 0.447);

        for degree in 0..360 {
            roundtrip(Scalar::from(degree), 0.5, 0.8);
        }
    }

    #[test]
    fn to_u32() {
        assert_eq!(0, Color::black().to_u32());
        assert_eq!(0xff0000, Color::red().to_u32());
        assert_eq!(0xffffff, Color::white().to_u32());
        assert_eq!(0xf4230f, Color::from_rgb(0xf4, 0x23, 0x0f).to_u32());
    }

    #[test]
    fn hsv_conversion() {
        let rgbf = |r, g, b| { Color::from_rgb_float(r, g, b) };
        let rgb128 = 128.0 / 255.0;

        assert_eq!(Color::white(), Color::from_hsv(0.0, 0.0, 1.0));
        assert_eq!(Color::white(), Color::from_hsv(120.0, 0.0, 1.0));
        assert_eq!(rgbf(0.5, 0.5, 0.5), Color::from_hsv(0.0, 0.0, 0.5));
        assert_eq!(Color::gray(), Color::from_hsv(300.0, 0.0, rgb128));
        assert_eq!(Color::black(), Color::from_hsv(0.0, 0.0, 0.0));
        assert_eq!(Color::black(), Color::from_hsv(240.0, 0.0, 0.0));
        assert_eq!(Color::red(), Color::from_hsv(0.0, 1.0, 1.0));
        assert_eq!(
            Color::from_hsl(60.0, 1.0, 0.375),
            Color::from_hsv(60.0, 1.0, 0.75)
        ); //yellow-green
        assert_eq!(Color::green(), Color::from_hsv(120.0, 1.0, rgb128));
        assert_eq!(
            Color::from_hsl(240.0, 1.0, 0.75),
            Color::from_hsv(240.0, 0.5, 1.0)
        ); // blue-ish
        assert_eq!(
            Color::from_hsl(49.5, 0.8922, 0.4973),
            Color::from_hsv(49.5, 0.94303, 0.94099)
        ); // yellow
        assert_eq!(
            Color::from_hsl(162.4, 0.7794, 0.4468),
            Color::from_hsv(162.4, 0.87603, 0.79504)
        ); // cyan 2

        assert_eq!(
            Color::from_rgba_float(0.75, 0.0, 0.75, 0.4),
            Color::from_hsva(300.0, 1.0, 0.75, 0.4)
        )
    }

    #[test]
    fn hsv_roundtrip_conversion() {
        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let hsv1 = color1.to_hsva();
            let color2 = Color::from_hsv(hsv1.h, hsv1.s, hsv1.v);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn hwb_conversion() {
        let rgbf = |r, g, b| { Color::from_rgb_float(r, g, b) };
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
    fn xyz_conversion() {
        assert_eq!(Color::white(), Color::from_xyz(0.950_46, 1.0, 1.089_06));
        assert_eq!(Color::red(), Color::from_xyz(0.412_391, 0.212_639, 0.019_331));
        assert_eq!(
            Color::from_hsl(109.99, 0.0865, 0.4078),
            Color::from_xyz(0.130_045, 0.152_291, 0.130_798)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let xyz1 = color1.to_xyz();
            let color2 = Color::from_xyz(xyz1.x, xyz1.y, xyz1.z);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn lms_conversion() {
        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lms1 = color1.to_lms();
            let color2 = Color::from_lms(lms1.l, lms1.m, lms1.s, 1.0);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn lab_conversion() {
        assert_eq!(Color::red(), Color::from_lab(53.2371, 80.0882, 67.1996, 1.0));

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lab1 = color1.to_lab();
            let color2 = Color::from_lab(lab1.l, lab1.a, lab1.b, 1.0);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn lch_conversion() {
        assert_eq!(
            Color::from_hsl(0.0, 1.0, 0.245),
            Color::from_lch(24.818, 60.063, 38.177, 1.0)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lch1 = color1.to_lch();
            let color2 = Color::from_lch(lch1.l, lch1.c, lch1.h, 1.0);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn luv_conversion() {
        assert_eq!(Color::red(), Color::from_luv(53.237, 175.003, 37.754, 1.0));

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let luv1 = color1.to_luv();
            let color2 = Color::from_luv(luv1.l, luv1.u, luv1.v, 1.0);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn lchuv_conversion() {
        assert_eq!(
            Color::from_hsl(0.0, 1.0, 0.245),
            Color::from_lchuv(24.818, 83.460, 12.174, 1.0)
        );
        assert_ne!(
            Color::from_hsl(0.0, 1.0, 0.245),
            Color::from_lchuv(24.82, 83.48, 12.17, 1.0)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lch1 = color1.to_lchuv();
            let color2 = Color::from_lchuv(lch1.l, lch1.c, lch1.h, 1.0);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn oklab_conversion() {
        assert_eq!(
            Color::from_xyz(0.950456, 1.000000, 1.089058),
            Color::from_oklab(1.0, 0.0, 0.0)
        );
        assert_eq!(
            Color::from_xyz(1.0, 0.0, 0.0),
            Color::from_oklab(0.449937, 1.235758, -0.018982)
        );
        assert_eq!(
            Color::from_xyz(0.0, 1.0, 0.0),
            Color::from_oklab(0.921816, -0.671211, 0.263400)
        );
        assert_eq!(
            Color::from_xyz(0.0, 0.0, 1.0),
            Color::from_oklab(0.152597, -1.415088, -0.448819)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lab1 = color1.to_oklab();
            let color2 = Color::from_oklab(lab1.l, lab1.a, lab1.b);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn oklch_conversion() {
        assert_eq!(
            Color::from_xyz(0.950456, 1.000000, 1.089058),
            Color::from_oklch(1.0, 0.0, 0.0)
        );
        assert_eq!(
            Color::from_xyz(1.0, 0.0, 0.0),
            Color::from_oklch(0.449937, 1.235904, 359.1200)
        );
        assert_eq!(
            Color::from_xyz(0.0, 1.0, 0.0),
            Color::from_oklch(0.921816, 0.721044, 158.5737)
        );
        assert_eq!(
            Color::from_xyz(0.0, 0.0, 1.0),
            Color::from_oklch(0.152597, 1.484558, 197.5973)
        );

        let roundtrip = |h, s, l| {
            let color1 = Color::from_hsl(h, s, l);
            let lch1 = color1.to_oklch();
            let color2 = Color::from_oklch(lch1.l, lch1.c, lch1.h);
            assert_eq!(&color1, &color2);
        };

        for hue in 0..360 {
            roundtrip(Scalar::from(hue), 0.2, 0.8);
        }
    }

    #[test]
    fn rotate_hue() {
        assert_eq!(Color::lime(), Color::red().rotate_hue(120.0));
    }

    #[test]
    fn complementary() {
        assert_eq!(Color::fuchsia(), Color::lime().complementary());
        assert_eq!(Color::lime(), Color::fuchsia().complementary());
    }

    #[test]
    fn lighten() {
        assert_eq!(
            Color::from_hsl(90.0, 0.5, 0.7),
            Color::from_hsl(90.0, 0.5, 0.3).lighten(0.4)
        );
        assert_eq!(
            Color::from_hsl(90.0, 0.5, 1.0),
            Color::from_hsl(90.0, 0.5, 0.3).lighten(0.8)
        );
    }

    #[test]
    fn to_gray() {
        let salmon = Color::from_rgb(250, 128, 114);
        assert_eq!(0.0, salmon.to_gray().to_hsla().s);
        assert_relative_eq!(
            salmon.luminance(),
            salmon.to_gray().luminance(),
            max_relative = 0.01
        );

        assert_eq!(Color::graytone(0.3), Color::graytone(0.3).to_gray());
    }

    #[test]
    fn brightness() {
        assert_relative_eq!(0.0, Color::black().brightness());
        assert_relative_eq!(1.0, Color::white().brightness());
        assert_relative_eq!(0.5, Color::graytone(0.5).brightness());
    }

    #[test]
    fn luminance() {
        assert_relative_eq!(1.0, Color::white().luminance());
        let hotpink = Color::from_rgb(255, 105, 180);
        assert_relative_eq!(0.347, hotpink.luminance(), max_relative = 0.01);
        assert_relative_eq!(0.0, Color::black().luminance());
    }

    #[test]
    fn contrast_ratio() {
        assert_relative_eq!(21.0, Color::black().contrast_ratio(&Color::white()));
        assert_relative_eq!(21.0, Color::white().contrast_ratio(&Color::black()));

        assert_relative_eq!(1.0, Color::white().contrast_ratio(&Color::white()));
        assert_relative_eq!(1.0, Color::red().contrast_ratio(&Color::red()));

        assert_relative_eq!(
            4.26,
            Color::from_rgb(255, 119, 153).contrast_ratio(&Color::from_rgb(0, 68, 85)),
            max_relative = 0.01
        );
    }

    #[test]
    fn text_color() {
        assert_eq!(Color::white(), Color::graytone(0.4).text_color());
        assert_eq!(Color::black(), Color::graytone(0.6).text_color());
    }

    #[test]
    fn distance_delta_e_cie76() {
        let c = Color::from_rgb(255, 127, 14);
        assert_eq!(0.0, c.distance_delta_e_cie76(&c));

        let c1 = Color::from_rgb(50, 100, 200);
        let c2 = Color::from_rgb(200, 10, 0);
        assert_eq!(123.0, c1.distance_delta_e_cie76(&c2).round());
    }

    #[test]
    fn to_hsl_string() {
        let c = Color::from_hsl(91.35, 0.5415, 0.98314);
        assert_eq!("hsl(91.35, 54.15%, 98.314%)", c.to_hsl_string());
        assert_eq!("hsl(91, 54.1%, 98.3%)", c.to_hsl_string_short());

        let c1 = Color::from_hsla(91.3, 0.542, 0.983, 0.4);
        assert_eq!("hsla(91.3, 54.2%, 98.3%, 0.4)", c1.to_hsl_string());
    }

    #[test]
    fn to_rgb_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("rgb(255, 127, 4)", c.to_rgb_string());

        let c1 = Color::from_rgba(255, 127, 4, 0.75);
        assert_eq!("rgba(255, 127, 4, 0.75)", c1.to_rgb_string());
    }

    #[test]
    fn to_rgb_float_string() {
        assert_eq!("rgb(0%, 0%, 0%)", Color::black().to_rgb_float_string());
        assert_eq!(
            "rgb(100%, 100%, 100%)",
            Color::white().to_rgb_float_string()
        );

        let c = Color::from_rgb_float(0.123, 0.456, 0.789);
        assert_eq!("rgb(12.3%, 45.6%, 78.9%)", c.to_rgb_float_string());

        let c1 = Color::from_rgba_float(0.4, 0.2, 0.6, 0.8);
        assert_eq!("rgba(40%, 20%, 60%, 0.8)", c1.to_rgb_float_string());

        // values rounded to negative zero should format as "0%", nor "-0%"
        let e = -1.0 / 1.0e10;
        let c2 = Color::from_rgb_float(0.75, 0.5, e);
        assert_eq!("rgb(75%, 50%, 0%)", c2.to_rgb_float_string());
    }

    #[test]
    fn to_color_srgb_string() {
        assert_eq!("color(srgb 0 0 0)", Color::black().to_color_srgb_string());
        assert_eq!("color(srgb 1 1 1)", Color::white().to_color_srgb_string());
        assert_eq!(
            "color(srgb 0.501961 0 0.501961)",
            Color::purple().to_color_srgb_string()
        );

        let c = Color::from_rgb_float(0.123, 0.456, 0.789);
        assert_eq!("color(srgb 0.123 0.456 0.789)", c.to_color_srgb_string());
    }

    #[test]
    fn to_color_srgb_string_alpha() {
        let c = Color::from_rgba_float(0.4, 0.2, 0.6, 0.8);
        assert_eq!("color(srgb 0.4 0.2 0.6 / 0.8)", c.to_color_srgb_string());
    }

    #[test]
    fn to_rgb_hex_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("ff7f04", c.to_rgb_hex_string(false));
        assert_eq!("#ff7f04", c.to_rgb_hex_string(true));
    }

    #[test]
    fn to_rgb_hex_string_alpha() {
        let c = Color::from_rgba(255, 127, 4, 0.4);
        assert_eq!("ff7f0466", c.to_rgb_hex_string(false));
        assert_eq!("#ff7f0466", c.to_rgb_hex_string(true));
    }

    #[test]
    fn to_hsv_string() {
        let c0 = Color::from_hsv(91.0, 0.541, 0.983);
        assert_eq!("hsv(91 54.1% 98.3%)", c0.to_hsv_string());

        let c1 = Color::from_hsv(91.3, 0.541, 0.983172);
        assert_eq!("hsv(91.3 54.1% 98.3172%)", c1.to_hsv_string());
    }

    #[test]
    fn to_hsv_string_alpha() {
        let c = Color::from_hsva(45.0, 0.5, 0.25, 0.7);
        assert_eq!("hsv(45 50% 25% / 0.7)", c.to_hsv_string());
    }

    #[test]
    fn to_hwb_string() {
        let c = Color::from_hwb(91.0, 0.541, 0.383);
        assert_eq!("hwb(91 54.1% 38.3%)", c.to_hwb_string());

        let c1 = Color::from_hwb(91.3, 0.541, 0.383172);
        assert_eq!("hwb(91.3 54.1% 38.3172%)", c1.to_hwb_string());
    }

    #[test]
    fn to_hwb_string_alpha() {
        let c = Color::from_hwba(45.0, 0.5, 0.25, 0.7);
        assert_eq!("hwb(45 50% 25% / 0.7)", c.to_hwb_string());
    }

    #[test]
    fn to_xyz_string() {
        let d65 = Color::from_xyz(D65_XN, D65_YN, D65_ZN);
        assert_eq!(
            "color(xyz 0.95047 1 1.08883)",
            d65.to_xyz_string()
        );
    }

    #[test]
    fn to_xyz_string_alpha() {
        let d50 = Color::from_xyza(0.9642, 1.0, 0.8249, 0.369);
        assert_eq!(
            "color(xyz 0.9642 1 0.8249 / 0.369)",
            d50.to_xyz_string()
        );
    }

    #[test]
    fn to_lab_string() {
        let c = Color::from_lab(41.0, 83.0, -93.0, 1.0);
        assert_eq!("lab(41% 83 -93)", c.to_lab_string());
    }

    #[test]
    fn to_lab_string_alpha() {
        let c = Color::from_lab(30.0, 60.0, -35.0, 0.25);
        assert_eq!("lab(30% 60 -35 / 0.25)", c.to_lab_string());
    }

    #[test]
    fn to_lch_string() {
        let c = Color::from_lch(52.0, 44.0, 271.0, 1.0);
        assert_eq!("lch(52% 44 271)", c.to_lch_string());

        let c1 = Color::from_lch(45.142857, 22.2222, 135.1415926, 1.0);
        assert_eq!("lch(45.1429% 22.2222 135.142)", c1.to_lch_string());
    }

    #[test]
    fn to_lch_string_alpha() {
        let c = Color::from_lch(30.0, 70.0, 330.0, 0.5);
        assert_eq!("lch(30% 70 330 / 0.5)", c.to_lch_string());
    }

    #[test]
    fn to_luv_string() {
        let c0 = Color::from_luv(41.0, 23.0, -39.0, 1.0);
        assert_eq!("luv(41% 23 -39)", c0.to_luv_string());

        let c1 = Color::from_luv(41.414141, 23.232323, -39.939393, 1.0);
        assert_eq!("luv(41.4141% 23.2323 -39.9394)", c1.to_luv_string());
    }

    #[test]
    fn to_luv_string_alpha() {
        let c = Color::from_luv(30.0, 60.0, -35.0, 0.25);
        assert_eq!("luv(30% 60 -35 / 0.25)", c.to_luv_string());
    }

    #[test]
    fn to_lchuv_string() {
        let c0 = Color::from_lchuv(52.0, 44.0, 271.0, 1.0);
        assert_eq!("lchuv(52% 44 271)", c0.to_lchuv_string());

        let c1 = Color::from_lchuv(52.525252, 44.444444, 271.271271, 1.0);
        assert_eq!("lchuv(52.5253% 44.4444 271.271)", c1.to_lchuv_string());
    }

    #[test]
    fn to_lchuv_string_alpha() {
        let c = Color::from_lchuv(30.0, 70.0, 330.0, 0.5);
        assert_eq!("lchuv(30% 70 330 / 0.5)", c.to_lchuv_string());
    }

    #[test]
    fn to_hcl_string() {
        let c = Color::from_lchuv(52.0, 44.0, 271.0, 1.0);
        assert_eq!("hcl(271 44 52%)", c.to_hcl_string());
    }

    #[test]
    fn to_hcl_string_alpha() {
        let c = Color::from_lchuv(40.0, 110.0, 10.0, 0.4);
        assert_eq!("hcl(10 110 40% / 0.4)", c.to_hcl_string());
    }

    #[test]
    fn to_oklab_string() {
        let c1 = Color::from_oklab(0.450, 1.236, -0.019);
        assert_eq!("oklab(45% 1.236 -0.019)", c1.to_oklab_string());

        let c2 = Color::from_oklab(0.922, -0.671, 0.263);
        assert_eq!("oklab(92.2% -0.671 0.263)", c2.to_oklab_string());
    }

    #[test]
    fn to_oklab_string_alpha() {
        let c = Color::from_oklaba(0.153, -1.415, -0.449, 0.25);
        assert_eq!("oklab(15.3% -1.415 -0.449 / 0.25)", c.to_oklab_string());
    }

    #[test]
    fn to_oklch_string() {
        let c = Color::from_oklch(0.52, 0.44, 271.0);
        assert_eq!("oklch(52% 0.44 271)", c.to_oklch_string());

        let c1 = Color::from_oklch(0.45142857, 0.2222222, 135.1415926);
        assert_eq!("oklch(45.1429% 0.222222 135.1416)", c1.to_oklch_string());
    }

    #[test]
    fn to_oklch_string_alpha() {
        let c = Color::from_oklcha(0.3, 0.4, 330.0, 0.5);
        assert_eq!("oklch(30% 0.4 330 / 0.5)", c.to_oklch_string());
    }

    #[test]
    fn mix() {
        assert_eq!(
            Color::from_rgb_float(0.5, 0.0, 0.5),  // purple
            Color::red().mix::<RGBA<f64>>(&Color::blue(), Fraction::from(0.5))
        );
        assert_eq!(
            Color::fuchsia(),
            Color::red().mix::<HSLA>(&Color::blue(), Fraction::from(0.5))
        );
        assert_eq!(
            Color::fuchsia(),
            Color::red().mix::<HWBA>(&Color::blue(), Fraction::from(0.5))
        );
    }

    #[test]
    fn mixing_with_gray_preserves_hue() {
        let hue = 123.0;

        let input = Color::from_hsla(hue, 0.5, 0.5, 1.0);

        let hue_after_mixing = |other| input.mix::<HSLA>(&other, Fraction::from(0.5)).to_hsla().h;

        assert_relative_eq!(hue, hue_after_mixing(Color::black()), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::graytone(0.2)), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::graytone(0.7)), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::white()), epsilon=1.0e-6);
    }

    #[test]
    fn mixing_with_gray_in_hwb_preserves_hue() {
        let hue = 123.0;

        let input = Color::from_hsla(hue, 0.5, 0.5, 1.0);

        let hue_after_mixing = |other| input.mix::<HWBA>(&other, Fraction::from(0.5)).to_hsla().h;

        assert_relative_eq!(hue, hue_after_mixing(Color::black()), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::graytone(0.2)), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::graytone(0.7)), epsilon=1.0e-6);
        assert_relative_eq!(hue, hue_after_mixing(Color::white()), epsilon=1.0e-6);
    }

    #[test]
    fn color_scale_add_preserves_ordering() {
        let mut color_scale = ColorScale::empty();

        color_scale
            .add_stop(Color::red(), Fraction::from(0.5))
            .add_stop(Color::gray(), Fraction::from(0.0))
            .add_stop(Color::blue(), Fraction::from(1.0));

        assert_eq!(color_scale.color_stops.get(0).unwrap().color, Color::gray());
        assert_eq!(color_scale.color_stops.get(1).unwrap().color, Color::red());
        assert_eq!(color_scale.color_stops.get(2).unwrap().color, Color::blue());
    }

    #[test]
    fn color_scale_empty_sample_none() {
        let mix = Color::mix::<Lab>;

        let color_scale = ColorScale::empty();

        let color = color_scale.sample(Fraction::from(0.0), &mix);

        assert_eq!(color, None);
    }

    #[test]
    fn color_scale_one_color_sample_none() {
        let mix = Color::mix::<Lab>;

        let mut color_scale = ColorScale::empty();

        color_scale.add_stop(Color::red(), Fraction::from(0.0));

        let color = color_scale.sample(Fraction::from(0.0), &mix);

        assert_eq!(color, None);
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
        assert_eq!("device-cmyk(0% 0% 0% 0%)", white.to_cmyk_string());

        let black = Color::from_rgb(0, 0, 0);
        assert_eq!("device-cmyk(0% 0% 0% 100%)", black.to_cmyk_string());

        let c = Color::from_rgb(19, 19, 1);
        assert_eq!(
            "device-cmyk(0% 0% 94.7368% 92.549%)",
            c.to_cmyk_string()
        );

        let c1 = Color::from_rgb(55, 55, 55);
        assert_eq!(
            "device-cmyk(0% 0% 0% 78.4314%)",
            c1.to_cmyk_string()
        );

        let c2 = Color::from_rgb(136, 117, 78);
        assert_eq!(
            "device-cmyk(0% 13.9706% 42.6471% 46.6667%)",
            c2.to_cmyk_string()
        );

        let c3 = Color::from_rgb(143, 111, 76);
        assert_eq!(
            "device-cmyk(0% 22.3776% 46.8531% 43.9216%)",
            c3.to_cmyk_string()
        );
    }

    #[test]
    fn to_cmyk_string_alpha() {
        let c = Color::from_rgba_float(0.5, 0.0, 0.5, 0.8);
        assert_eq!(
            "device-cmyk(0% 100% 0% 50% / 0.8)",
            c.to_cmyk_string()
        );
    }

    #[test]
    fn out_of_gamut() {
        let c = Color::from_lch(50.0, 75.0, 60.0, 1.0);
        assert_eq!(c, Color::from_rgb_float(0.755768, 0.349482, -0.075965));

        let c2 = c.to_rgba();
        assert_eq!(c2, RGBA { r: 193, g: 89, b: 0, alpha: 1.0 });
    }
}
