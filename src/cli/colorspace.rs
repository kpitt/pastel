use pastel::Color;
use pastel::{Fraction, Hsla, Hwba, Lab, Lch, Srgba};

pub fn get_mixing_function(
    colorspace_name: &str,
) -> Box<dyn Fn(&Color, &Color, Fraction) -> Color> {
    match colorspace_name.to_lowercase().as_ref() {
        "rgb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Srgba<f64>>(c2, f)),
        "hsl" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Hsla>(c2, f)),
        "hwb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Hwba>(c2, f)),
        "lab" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Lab>(c2, f)),
        "lch" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Lch>(c2, f)),
        _ => unreachable!("Unknown color space"),
    }
}
