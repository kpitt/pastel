pub mod cmyk;
pub mod hsl;
pub mod hsv;
pub mod hwb;
pub mod lab;
pub mod lch;
pub mod lms;
pub mod rgb;
pub mod xyz;

use std::cmp::Ordering;

use crate::{
    helper::{mod_positive, Fraction},
    types::Scalar,
    Color,
};

pub trait ColorSpace {
    fn from_color(c: &Color) -> Self;
    fn into_color(self) -> Color;

    fn mix(&self, other: &Self, fraction: Fraction) -> Self;
}

/// Linearly interpolate between two values.
fn interpolate(a: Scalar, b: Scalar, fraction: Fraction) -> Scalar {
    a + fraction.value() * (b - a)
}

/// Linearly interpolate between two angles. Always take the shortest path
/// along the circle.
fn interpolate_angle(a: Scalar, b: Scalar, fraction: Fraction) -> Scalar {
    let paths = [(a, b), (a, b + 360.0), (a + 360.0, b)];

    let dist = |&(x, y): &(Scalar, Scalar)| (x - y).abs();
    let shortest = paths
        .iter()
        .min_by(|p1, p2| dist(p1).partial_cmp(&dist(p2)).unwrap_or(Ordering::Less))
        .unwrap();

    mod_positive(interpolate(shortest.0, shortest.1, fraction), 360.0)
}

#[test]
fn test_interpolate() {
    assert_eq!(0.0, interpolate_angle(0.0, 90.0, Fraction::from(0.0)));
    assert_eq!(45.0, interpolate_angle(0.0, 90.0, Fraction::from(0.5)));
    assert_eq!(90.0, interpolate_angle(0.0, 90.0, Fraction::from(1.0)));
    assert_eq!(90.0, interpolate_angle(0.0, 90.0, Fraction::from(1.1)));
}

#[test]
fn test_interpolate_angle() {
    assert_eq!(15.0, interpolate_angle(0.0, 30.0, Fraction::from(0.5)));
    assert_eq!(20.0, interpolate_angle(0.0, 100.0, Fraction::from(0.2)));
    assert_eq!(0.0, interpolate_angle(10.0, 350.0, Fraction::from(0.5)));
    assert_eq!(0.0, interpolate_angle(350.0, 10.0, Fraction::from(0.5)));
}
