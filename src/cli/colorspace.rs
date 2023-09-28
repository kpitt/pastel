use pastel::Color;
use pastel::{Fraction, Hsla, Hwba, Lab, Lch, Rgba};

pub type ColorMixingFn = dyn Fn(&Color, &Color, Fraction) -> Color;

pub fn get_mixing_function(colorspace_name: &str) -> Box<ColorMixingFn> {
    match colorspace_name.to_lowercase().as_ref() {
        "rgb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Rgba<f64>>(c2, f)),
        "hsl" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Hsla>(c2, f)),
        "hwb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Hwba>(c2, f)),
        "lab" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Lab>(c2, f)),
        "lch" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Lch>(c2, f)),
        _ => unreachable!("Unknown color space"),
    }
}
