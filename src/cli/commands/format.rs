use crate::commands::prelude::*;
use crate::utility::similar_colors;

use pastel::ansi::Mode;

pub struct FormatCommand;

impl ColorCommand for FormatCommand {
    fn run(
        &self,
        out: &mut Output,
        matches: &ArgMatches,
        config: &Config,
        color: &Color,
    ) -> Result<()> {
        let format_type = matches.value_of("type").expect("required argument");
        let format_type = format_type.to_lowercase();

        let replace_escape = |code: &str| code.replace("\x1b", "\\x1b");

        let output = match format_type.as_ref() {
            "rgb" => color.to_rgb_string(),
            "rgb-float" => color.to_rgb_float_string(),
            "hex" => color.to_rgb_hex_string(true),
            "hsl" => color.to_hsl_string(),
            "hsl-hue" => format!("{:.0}", color.to_hsla().h),
            "hsl-saturation" => format!("{:.4}", color.to_hsla().s),
            "hsl-lightness" => format!("{:.4}", color.to_hsla().l),
            "hsv" => color.to_hsv_string(),
            "hsv-hue" => format!("{:.0}", color.to_hsva().h),
            "hsv-saturation" => format!("{:.4}", color.to_hsva().s),
            "hsv-value" => format!("{:.4}", color.to_hsva().v),
            "hwb" => color.to_hwb_string(),
            "hwb-hue" => format!("{:.0}", color.to_hwba().h),
            "hwb-whiteness" => format!("{:.4}", color.to_hwba().w),
            "hwb-blackness" => format!("{:.4}", color.to_hwba().b),
            "xyz" => color.to_xyz_string(),
            "lab" => color.to_lab_string(),
            "lab-lightness" => format!("{:.2}", color.to_lab().l),
            "lab-a" => format!("{:.2}", color.to_lab().a),
            "lab-b" => format!("{:.2}", color.to_lab().b),
            "lch" => color.to_lch_string(),
            "lab-chroma" => format!("{:.2}", color.to_lch().c),
            "lab-hue" => format!("{:.2}", color.to_lch().h),
            "luv" => color.to_luv_string(),
            "luv-lightness" => format!("{:.2}", color.to_luv().l),
            "luv-u" => format!("{:.2}", color.to_luv().u),
            "luv-v" => format!("{:.2}", color.to_luv().v),
            "lchuv" => color.to_lchuv_string(),
            "luv-chroma" => format!("{:.2}", color.to_lchuv().c),
            "luv-hue" => format!("{:.2}", color.to_lchuv().h),
            "hcl" => color.to_hcl_string(),
            "luminance" => format!("{:.3}", color.luminance()),
            "brightness" => format!("{:.3}", color.brightness()),
            "ansi-8bit" => replace_escape(&color.to_ansi_sequence(Mode::Ansi8Bit)),
            "ansi-24bit" => replace_escape(&color.to_ansi_sequence(Mode::TrueColor)),
            "ansi-8bit-escapecode" => color.to_ansi_sequence(Mode::Ansi8Bit),
            "ansi-24bit-escapecode" => color.to_ansi_sequence(Mode::TrueColor),
            "cmyk" => color.to_cmyk_string(),
            "name" => similar_colors(color)[0].name.to_owned(),
            &_ => {
                unreachable!("Unknown format type");
            }
        };

        let write_colored_line = !matches!(
            format_type.as_ref(),
            "ansi-8bit-escapecode" | "ansi-24bit-escapecode"
        );

        if write_colored_line {
            writeln!(
                out.handle,
                "{}",
                config
                    .brush
                    .paint(output, color.text_color().ansi_style().on(color))
            )?;
        } else {
            write!(out.handle, "{}", output)?;
        }

        Ok(())
    }
}
