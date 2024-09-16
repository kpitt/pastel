use clap::{builder::PossibleValue, ValueEnum};

use pastel::ansi::Brush;

#[derive(Debug, Clone)]
pub struct Config<'p> {
    pub padding: usize,
    pub colorpicker_width: usize,
    pub colorcheck_width: usize,
    pub colorpicker: Option<&'p str>,
    pub interactive_mode: bool,
    pub brush: Brush,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorMode {
    Ansi8Bit,
    TrueColor,
    Off,
    Auto,
}

impl ValueEnum for ColorMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::TrueColor, Self::Ansi8Bit, Self::Off, Self::Auto]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Ansi8Bit => PossibleValue::new("8bit"),
            Self::TrueColor => PossibleValue::new("24bit"),
            Self::Off => PossibleValue::new("off"),
            Self::Auto => PossibleValue::new("auto"),
        })
    }
}
