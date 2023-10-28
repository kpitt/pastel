use crate::{Color, Srgba};
use approx::assert_relative_eq;

pub fn assert_almost_equal(c1: &Color, c2: &Color) {
    let c1 = c1.to_rgba();
    let c2 = c2.to_rgba();

    assert!((c1.r as i32 - c2.r as i32).abs() <= 1);
    assert!((c1.g as i32 - c2.g as i32).abs() <= 1);
    assert!((c1.b as i32 - c2.b as i32).abs() <= 1);
}

pub fn assert_rgb_almost_equal(c1: &Srgba<f64>, c2: &Srgba<f64>) {
    const EPS: f64 = f32::EPSILON as f64;

    assert_relative_eq!(c1.r, c2.r, epsilon = EPS);
    assert_relative_eq!(c1.g, c2.g, epsilon = EPS);
    assert_relative_eq!(c1.b, c2.b, epsilon = EPS);
}
