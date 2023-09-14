pub mod cmyk;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod lab;
pub mod lch;
pub mod lms;
pub mod rgb;
pub mod xyz;

use crate::helper::Fraction;
use crate::Color;

pub trait ColorSpace {
    fn from_color(c: &Color) -> Self;
    fn into_color(self) -> Color;

    fn mix(&self, other: &Self, fraction: Fraction) -> Self;
}
