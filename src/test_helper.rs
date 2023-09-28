use crate::Color;

pub fn assert_almost_equal(c1: &Color, c2: &Color) {
    let c1 = c1.to_rgba();
    let c2 = c2.to_rgba();

    assert!((c1.r as i32 - c2.r as i32).abs() <= 1);
    assert!((c1.g as i32 - c2.g as i32).abs() <= 1);
    assert!((c1.b as i32 - c2.b as i32).abs() <= 1);
}
