pub mod ansi;
pub mod cmyk;
pub mod colorspace;
pub mod convert;
pub mod delta_e;
pub mod distinct;
mod helper;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod lab;
pub mod lch;
pub mod lms;
pub mod matrix;
pub mod named;
pub mod parser;
pub mod random;
pub mod rgb;
mod types;
pub mod xyz;

#[cfg(test)]
mod test_helper;

use std::{fmt, str::FromStr};

// Re-export color space types
pub use cmyk::CMYK;
pub use hsl::HSLA;
pub use hsv::HSVA;
pub use hwb::HWBA;
pub use lab::Lab;
pub use lch::LCh;
pub use lms::LMS;
pub use rgb::RGBA;
pub use xyz::XYZ;

use colorspace::ColorSpace;
pub use helper::Fraction;
use helper::MaxPrecision;
use types::{Hue, Scalar};

/// The representation of a color.
///
/// Note:
/// - Colors outside the sRGB gamut (which cannot be displayed on a typical
///   computer screen) can not be represented by `Color`.
/// - The `PartialEq` instance compares two `Color`s by comparing their (integer)
///   RGB values. This is different from comparing the HSL values. For example,
///   HSL has many different representations of black (arbitrary hue and
///   saturation values).
#[derive(Clone)]
pub struct Color {
    hue: Hue,
    saturation: Scalar,
    lightness: Scalar,
    alpha: Scalar,
}

// Illuminant D65 constants used for Lab color space conversions.
const D65_XN: Scalar = 0.950_470;
const D65_YN: Scalar = 1.0;
const D65_ZN: Scalar = 1.088_830;

fn format_css_alpha(alpha: Scalar, format: Format) -> String {
    if alpha == 1.0 {
        String::from("")
    } else {
        format!(
            "{space}/{space}{alpha}",
            alpha = MaxPrecision::wrap(3, alpha),
            space = if format == Format::Spaces { " " } else { "" }
        )
    }
}

impl Color {
    #[inline]
    pub fn from_hsla(hue: Scalar, saturation: Scalar, lightness: Scalar, alpha: Scalar) -> Color {
        Self::from(&HSLA::with_alpha(hue, saturation, lightness, alpha))
    }

    #[inline]
    pub fn from_hsl(hue: Scalar, saturation: Scalar, lightness: Scalar) -> Color {
        Self::from(&HSLA::new(hue, saturation, lightness))
    }

    #[inline]
    pub fn from_hsva(hue: Scalar, saturation: Scalar, value: Scalar, alpha: Scalar) -> Color {
        Self::from(&HSVA::with_alpha(hue, saturation, value, alpha))
    }

    #[inline]
    pub fn from_hsv(hue: Scalar, saturation: Scalar, value: Scalar) -> Color {
        Self::from(&HSVA::new(hue, saturation, value))
    }

    /// Create a `Color` from a hue, and whiteness and blackness values with a
    /// floating point alpha value between 0.0 and 1.0.
    ///
    /// See:
    /// - https://en.wikipedia.org/wiki/HWB_color_model
    #[inline]
    pub fn from_hwba(hue: Scalar, whiteness: Scalar, blackness: Scalar, alpha: Scalar) -> Color {
        Self::from(&HWBA::with_alpha(hue, whiteness, blackness, alpha))
    }

    /// Create a `Color` from a hue, and whiteness and blackness values.
    #[inline]
    pub fn from_hwb(hue: Scalar, whiteness: Scalar, blackness: Scalar) -> Color {
        Self::from(&HWBA::new(hue, whiteness, blackness))
    }

    /// Create a `Color` from integer RGB values between 0 and 255 and a floating
    /// point alpha value between 0.0 and 1.0.
    #[inline]
    pub fn from_rgba(r: u8, g: u8, b: u8, alpha: Scalar) -> Color {
        // RGB to HSL conversion algorithm adapted from
        // https://en.wikipedia.org/wiki/HSL_and_HSV
        Self::from(&RGBA::with_alpha(r, g, b, alpha))
    }

    /// Create a `Color` from integer RGB values between 0 and 255.
    #[inline]
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Self::from(&RGBA::new(r, g, b))
    }

    /// Create a `Color` from RGB and alpha values between 0.0 and 1.0. Values outside this range
    /// will be clamped.
    #[inline]
    pub fn from_rgba_float(r: Scalar, g: Scalar, b: Scalar, alpha: Scalar) -> Color {
        Self::from(&RGBA::with_alpha(r, g, b, alpha))
    }

    /// Create a `Color` from RGB values between 0.0 and 1.0. Values outside this range will be
    /// clamped.
    #[inline]
    pub fn from_rgb_float(r: Scalar, g: Scalar, b: Scalar) -> Color {
        Self::from(&RGBA::new(r, g, b))
    }

    /// Create a `Color` from XYZ coordinates in the CIE 1931 color space. Note that a `Color`
    /// always represents a color in the sRGB gamut (colors that can be represented on a typical
    /// computer screen) while the XYZ color space is bigger. This function will tend to create
    /// fully saturated colors at the edge of the sRGB gamut if the coordinates lie outside the
    /// sRGB range.
    ///
    /// See:
    /// - <https://en.wikipedia.org/wiki/CIE_1931_color_space>
    /// - <https://en.wikipedia.org/wiki/SRGB>
    #[inline]
    pub fn from_xyz(x: Scalar, y: Scalar, z: Scalar, alpha: Scalar) -> Color {
        Self::from(&XYZ::with_alpha(x, y, z, alpha))
    }

    /// Create a `Color` from LMS coordinates. This is the matrix inverse of the matrix that
    /// appears in `to_lms`.
    #[inline]
    pub fn from_lms(l: Scalar, m: Scalar, s: Scalar, alpha: Scalar) -> Color {
        Self::from(&LMS::with_alpha(l, m, s, alpha))
    }

    /// Create a `Color` from L, a and b coordinates coordinates in the Lab color
    /// space. Note: See documentation for `from_xyz`. The same restrictions apply here.
    ///
    /// See: <https://en.wikipedia.org/wiki/Lab_color_space>
    #[inline]
    pub fn from_lab(l: Scalar, a: Scalar, b: Scalar, alpha: Scalar) -> Color {
        Self::from(&Lab::with_alpha(l, a, b, alpha))
    }

    /// Create a `Color` from lightness, chroma and hue coordinates in the CIE LCh color space.
    /// This is a cylindrical transform of the Lab color space. Note: See documentation for
    /// `from_xyz`. The same restrictions apply here.
    ///
    /// See: <https://en.wikipedia.org/wiki/Lab_color_space>
    #[inline]
    pub fn from_lch(l: Scalar, c: Scalar, h: Scalar, alpha: Scalar) -> Color {
        Self::from(&LCh::with_alpha(l, c, h, alpha))
    }

    /// Create a `Color` from  the four colours of the CMYK model: Cyan, Magenta, Yellow and Black.
    /// The CMYK colours are subtractive. This means the colours get darker as you blend them together
    #[inline]
    pub fn from_cmyk(c: Scalar, m: Scalar, y: Scalar, k: Scalar) -> Color {
        Self::from(&CMYK::new(c, m, y, k))
    }

    /// Convert a `Color` to its hue, saturation, lightness and alpha values. The hue is given
    /// in degrees, as a number between 0.0 and 360.0. Saturation, lightness and alpha are numbers
    /// between 0.0 and 1.0.
    #[inline]
    pub fn to_hsla(&self) -> HSLA {
        HSLA::from(self)
    }

    /// Format the color as a HSL-representation string (`hsla(123, 50.3%, 80.1%, 0.4)`). If the
    /// alpha channel is `1.0`, the simplified `hsl()` format will be used instead.
    #[inline]
    pub fn to_hsl_string(&self, format: Format) -> String {
        HSLA::from(self).to_color_string(format)
    }

    /// Convert a `Color` to its hue, saturation, value and alpha values. The hue is given
    /// in degrees, as a number between 0.0 and 360.0. Saturation, value and alpha are numbers
    /// between 0.0 and 1.0.
    #[inline]
    pub fn to_hsva(&self) -> HSVA {
        HSVA::from(self)
    }

    /// Format the color as a HSV-representation string (`hsva(123, 50.3%, 80.1%, 0.4)`). If the
    /// alpha channel is `1.0`, the simplified `hsv()` format will be used instead.
    #[inline]
    pub fn to_hsv_string(&self, format: Format) -> String {
        HSVA::from(self).to_color_string(format)
    }

    /// Convert a `Color` to its hue, whiteness, blackness, and alpha values. The hue is given in
    /// degrees, as a number between 0.0 and 360.0. Whiteness, blackness, and alpha are numbers
    /// between 0.0 and 1.0.
    #[inline]
    pub fn to_hwba(&self) -> HWBA {
        HWBA::from(self)
    }

    /// Format the color as a HWB-representation string (`hwb(123, 50.3%, 80.1%)`).
    #[inline]
    pub fn to_hwb_string(&self, format: Format) -> String {
        HWBA::from(self).to_color_string(format)
    }

    /// Convert a `Color` to its red, green, blue and alpha values. The RGB values are integers in
    /// the range from 0 to 255. The alpha channel is a number between 0.0 and 1.0.
    #[inline]
    pub fn to_rgba(&self) -> RGBA<u8> {
        RGBA::<u8>::from(self)
    }

    /// Format the color as a RGB-representation string (`rgba(255, 127, 0, 0.5)`). If the alpha channel
    /// is `1.0`, the simplified `rgb()` format will be used instead.
    #[inline]
    pub fn to_rgb_string(&self, format: Format) -> String {
        RGBA::<u8>::from(self).to_color_string(format)
    }

    /// Convert a `Color` to its cyan, magenta, yellow, and black values. The CMYK
    /// values are floats smaller than or equal to 1.0.
    #[inline]
    pub fn to_cmyk(&self) -> CMYK {
        CMYK::from(self)
    }

    /// Format the color as a CMYK-representation string (`cmyk(0, 50, 100, 100)`).
    #[inline]
    pub fn to_cmyk_string(&self, format: Format) -> String {
        CMYK::from(self).to_color_string(format)
    }

    /// Format the color as a floating point RGB-representation string (`rgb(1.0, 0.5, 0)`). If the alpha channel
    /// is `1.0`, the simplified `rgb()` format will be used instead.
    #[inline]
    pub fn to_rgb_float_string(&self, format: Format) -> String {
        RGBA::<f64>::from(self).to_color_string(format)
    }

    /// Format the color as a RGB-representation string (`#fc0070`). The output will contain 6 hex
    /// digits if the alpha channel is `1.0`, or 8 hex digits otherwise.
    #[inline]
    pub fn to_rgb_hex_string(&self, leading_hash: bool) -> String {
        RGBA::<u8>::from(self).to_hex_string(leading_hash)
    }

    /// Convert a `Color` to its red, green, blue and alpha values. All numbers are from the range
    /// between 0.0 and 1.0.
    #[inline]
    pub fn to_rgba_float(&self) -> RGBA<Scalar> {
        RGBA::<f64>::from(self)
    }

    /// Return the color as an integer in RGB representation (`0xRRGGBB`)
    #[inline]
    pub fn to_u32(&self) -> u32 {
        self.to_rgba().to_u32()
    }

    /// Get XYZ coordinates according to the CIE 1931 color space.
    ///
    /// See:
    /// - <https://en.wikipedia.org/wiki/CIE_1931_color_space>
    /// - <https://en.wikipedia.org/wiki/SRGB>
    #[inline]
    pub fn to_xyz(&self) -> XYZ {
        XYZ::from(self)
    }

    /// Get coordinates according to the LSM color space
    ///
    /// See <https://en.wikipedia.org/wiki/LMS_color_space> for info on the color space as well as an
    /// algorithm for converting from CIE XYZ
    #[inline]
    pub fn to_lms(&self) -> LMS {
        LMS::from(self)
    }

    /// Get L, a and b coordinates according to the Lab color space.
    ///
    /// See: <https://en.wikipedia.org/wiki/Lab_color_space>
    #[inline]
    pub fn to_lab(&self) -> Lab {
        Lab::from(self)
    }

    /// Format the color as a Lab-representation string (`Lab(41, 83, -93, 0.5)`). If the alpha channel
    /// is `1.0`, it won't be included in the output.
    #[inline]
    pub fn to_lab_string(&self, format: Format) -> String {
        Lab::from(self).to_color_string(format)
    }

    /// Get L, C and h coordinates according to the CIE LCh color space.
    ///
    /// See: <https://en.wikipedia.org/wiki/Lab_color_space>
    #[inline]
    pub fn to_lch(&self) -> LCh {
        LCh::from(self)
    }

    /// Format the color as a LCh-representation string (`LCh(0.3, 0.2, 0.1, 0.5)`). If the alpha channel
    /// is `1.0`, it won't be included in the output.
    #[inline]
    pub fn to_lch_string(&self, format: Format) -> String {
        LCh::from(self).to_color_string(format)
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

    /// Change the alpha value of a Color.
    pub fn with_alpha(&self, alpha: Scalar) -> Color {
        Self::from_hsla(self.hue.value(), self.saturation, self.lightness, alpha)
    }

    /// Rotate along the "hue" axis.
    pub fn rotate_hue(&self, delta: Scalar) -> Color {
        Self::from_hsla(
            self.hue.value() + delta,
            self.saturation,
            self.lightness,
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
        Self::from_hsla(
            self.hue.value(),
            self.saturation,
            self.lightness + f,
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
        Self::from_hsla(
            self.hue.value(),
            self.saturation + f,
            self.lightness,
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
        let hue = self.hue;
        let c = self.to_lch();

        // the desaturation step is only needed to correct minor rounding errors.
        let mut gray = Color::from_lch(c.l, 0.0, 0.0, 1.0).desaturate(1.0);

        // Restore the hue value (does not alter the color, but makes it able to add saturation
        // again)
        gray.hue = hue;

        gray
    }

    /// The perceived brightness of the color (A number between 0.0 and 1.0).
    ///
    /// See: <https://www.w3.org/TR/AERT#color-contrast>
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
    /// See: <https://www.w3.org/TR/2008/REC-WCAG20-20081211/#relativeluminancedef>
    pub fn luminance(&self) -> Scalar {
        fn f(s: Scalar) -> Scalar {
            if s <= 0.03928 {
                s / 12.92
            } else {
                Scalar::powf((s + 0.055) / 1.055, 2.4)
            }
        }

        let c = self.to_rgba_float();
        let r = f(c.r);
        let g = f(c.g);
        let b = f(c.b);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Contrast ratio between two colors as defined by the WCAG. The ratio can range from 1.0
    /// to 21.0. Two colors with a contrast ratio of 4.5 or higher can be used as text color and
    /// background color and should be well readable.
    ///
    /// <https://www.w3.org/TR/2008/REC-WCAG20-20081211/#contrast-ratiodef>
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
    /// standard. A distance below ~2.3 is not noticeable.
    ///
    /// See: <https://en.wikipedia.org/wiki/Color_difference>
    pub fn distance_delta_e_cie76(&self, other: &Color) -> Scalar {
        delta_e::cie76(&self.to_lab(), &other.to_lab())
    }

    /// Compute the perceived 'distance' between two colors according to the CIEDE2000 delta-E
    /// standard.
    ///
    /// See: <https://en.wikipedia.org/wiki/Color_difference>
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

    /// Alpha composite two colors, placing the second over the first.
    pub fn composite(&self, source: &Color) -> Color {
        let backdrop = self.to_rgba();
        let source = source.to_rgba();

        // Composite A over B (see https://en.wikipedia.org/wiki/Alpha_compositing)
        //
        //   αo = αa + αb(1 - αa)
        //
        //        Ca * αa + Cb * αb(1 - αa)
        //   Co = -------------------------
        //                   αo
        //
        //       αo:  output alpha
        //   αa, αb:  A/B alpha
        //       Co:  output color
        //   Ca, Cb:  A/B color
        //
        fn composite_channel(c_a: u8, a_a: f64, c_b: u8, a_b: f64, a_o: f64) -> u8 {
            ((c_a as f64 * a_a + c_b as f64 * a_b * (1.0 - a_a)) / a_o).floor() as u8
        }

        let a = source.alpha + backdrop.alpha * (1.0 - source.alpha);
        let r = composite_channel(source.r, source.alpha, backdrop.r, backdrop.alpha, a);
        let g = composite_channel(source.g, source.alpha, backdrop.g, backdrop.alpha, a);
        let b = composite_channel(source.b, source.alpha, backdrop.b, backdrop.alpha, a);

        Color::from_rgba(r, g, b, a)
    }
}

// by default Colors will be printed into HSLA format
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", HSLA::from(self))
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Color::from_{}", self.to_rgb_string(Format::NoSpaces))
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Color) -> bool {
        self.to_rgba() == other.to_rgba()
    }
}

impl FromStr for Color {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse_color(s).ok_or("invalid color string")
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Spaces,
    NoSpaces,
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

                let index = next_index.unwrap_or(self.color_stops.len());

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
    }

    #[test]
    fn to_u32() {
        assert_eq!(0xff00ff, Color::fuchsia().to_u32());
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
        assert_eq!(0.0, Color::black().brightness());
        assert_eq!(1.0, Color::white().brightness());
        assert_eq!(0.5, Color::graytone(0.5).brightness());
    }

    #[test]
    fn luminance() {
        assert_eq!(1.0, Color::white().luminance());
        let hotpink = Color::from_rgb(255, 105, 180);
        assert_relative_eq!(0.347, hotpink.luminance(), max_relative = 0.01);
        assert_eq!(0.0, Color::black().luminance());
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
    fn to_hwb_string() {
        let c = Color::from_hwb(91.0, 0.541, 0.383);
        // modern CSS functional syntax, no alpha value
        assert_eq!("hwb(91 54.1% 38.3%)", c.to_hwb_string(Format::Spaces));

        let c1 = Color::from_hwba(90.0, 0.5, 0.25, 0.8);
        // non-unit alpha is serialized as a number
        assert_eq!("hwb(90 50% 25% / 0.8)", c1.to_hwb_string(Format::Spaces));
    }

    // Test alternative alpha formats once for shared function.  Each format that
    // uses CSS alpha should test that it is applied.
    #[test]
    fn css_alpha_string() {
        // unit alpha returns empty string regardless of Spaces option
        assert_eq!("", format_css_alpha(1.0, Format::Spaces));
        assert_eq!("", format_css_alpha(1.0, Format::NoSpaces));

        // alpha is serialized as a number, not a percentage
        assert_eq!(" / 0.75", format_css_alpha(0.75, Format::Spaces));
        // spaces are optional around alpha separator, so NoSpaces applies
        assert_eq!("/0.75", format_css_alpha(0.75, Format::NoSpaces));

        // values are rounded to 3 decimal places
        assert_eq!(" / 0.812", format_css_alpha(0.8118, Format::Spaces));
        // no trailing decimal zeros, even after rounding
        assert_eq!(" / 0.8", format_css_alpha(0.799999, Format::Spaces));
    }

    #[test]
    fn to_rgb_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("rgb(255, 127, 4)", c.to_rgb_string(Format::Spaces));
    }

    #[test]
    fn to_rgb_float_string() {
        assert_eq!(
            "rgb(1.000, 1.000, 1.000)",
            Color::white().to_rgb_float_string(Format::Spaces)
        );
    }

    #[test]
    fn to_rgb_hex_string() {
        let c = Color::from_rgb(255, 127, 4);
        assert_eq!("#ff7f04", c.to_rgb_hex_string(true));
    }

    #[test]
    fn to_cmyk_string() {
        let c = Color::from_rgb(136, 117, 78);
        assert_eq!("cmyk(0, 14, 43, 47)", c.to_cmyk_string(Format::Spaces));
    }

    #[test]
    fn mix() {
        assert_eq!(
            Color::purple(),
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

        let mix_in_hsl = |other| input.mix::<HSLA>(&other, Fraction::from(0.5));
        assert_eq!(hue, mix_in_hsl(Color::graytone(0.5)).hue.value());

        let mix_in_hwb = |other| input.mix::<HWBA>(&other, Fraction::from(0.5));
        assert_eq!(hue, mix_in_hwb(Color::graytone(0.5)).hue.value());
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
    fn alpha_roundtrip_hex_to_decimal() {
        // We use a max of 3 decimal places when displaying RGB floating point
        // alpha values. This test insures that is sufficient to "roundtrip"
        // from hex (0 < n < 255) to float (0 < n < 1) and back again,
        // e.g. hex `80` is float `0.502`, which parses to hex `80`, and so on.
        for alpha_int in 0..255 {
            let hex_string = format!("#000000{:02x}", alpha_int);
            let parsed_from_hex = hex_string.parse::<Color>().unwrap();
            let rgba_string = parsed_from_hex.to_rgb_float_string(Format::Spaces);
            let parsed_from_rgba = rgba_string.parse::<Color>().unwrap();
            assert_eq!(hex_string, parsed_from_rgba.to_rgb_hex_string(true));
        }
    }
}
