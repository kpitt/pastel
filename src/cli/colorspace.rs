use pastel::Color;
use pastel::{Fraction, Lab, LCh, Luv, LChuv, OKLab, OKLCh, HSLA, HSVA, HWBA, RGBA};

pub fn get_mixing_function(
    colorspace_name: &str,
) -> Box<dyn Fn(&Color, &Color, Fraction) -> Color> {
    match colorspace_name.to_lowercase().as_ref() {
        "rgb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<RGBA<f64>>(c2, f)),
        "hsl" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<HSLA>(c2, f)),
        "hsv" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<HSVA>(c2, f)),
        "hwb" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<HWBA>(c2, f)),
        "lab" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Lab>(c2, f)),
        "lch" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<LCh>(c2, f)),
        "luv" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<Luv>(c2, f)),
        "lchuv" | "hcl" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<LChuv>(c2, f)),
        "oklab" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<OKLab>(c2, f)),
        "oklch" => Box::new(|c1: &Color, c2: &Color, f: Fraction| c1.mix::<OKLCh>(c2, f)),
        _ => unreachable!("Unknown color space"),
    }
}
