use crate::commands::prelude::*;

pub struct ContrastCommand;

impl ColorCommand for ContrastCommand {
    fn run(
        &self,
        out: &mut Output,
        matches: &ArgMatches,
        config: &Config,
        color: &Color,
    ) -> Result<()> {
        let mut print_spectrum = PrintSpectrum::Yes;

        let base = ColorArgIterator::from_color_arg(
            config,
            matches.value_of("base").expect("required argument"),
            &mut print_spectrum,
        )?;

        let output = format!("{:.1}:1", base.contrast_ratio(color));
        writeln!(
            out.handle,
            "{}",
            config
                .brush
                .paint(output, color.ansi_style().on(base))
        )?;

        Ok(())
    }
}
