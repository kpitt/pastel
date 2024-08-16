use std::io;

use crate::{cli::build_cli, commands::prelude::*};

use clap::{crate_name, Arg, Command};
use clap_complete::{generate, Shell};

pub struct CompletionsCommand;

pub fn cli() -> Command {
    Command::new("completions")
        .about("Generate a tab-completion script for your shell")
        .long_about(color_print::cstr!(
            "Generate a script to enable tab completion for Bash, Fish, Zsh, Elvish, or PowerShell. \
             The script is written to <bold>stdout</>, and can be re-directed to the appropriate \
             completion file.\n\
             \n\
             <green,bold>Examples:</>\
             \n  <cyan,bold>pastel completions bash > ~/.bash_completions/pastel.bash</>\
             \n  <cyan,bold>pastel completions fish > ~/.config/fish/completions/pastel.fish</>"
        ))
        .arg(
            Arg::new("shell")
                .value_name("SHELL")
                .value_parser(["bash", "elvish", "fish", "powershell", "zsh"])
                .required(true)
        )
}

impl GenericCommand for CompletionsCommand {
    fn run(&self, _: &mut Output, matches: &ArgMatches, _: &Config) -> Result<()> {
        let shell_type = matches
            .get_one::<String>("shell")
            .expect("required argument")
            .to_lowercase();

        let shell = match shell_type.as_ref() {
            "bash" => Shell::Bash,
            "elvish" => Shell::Elvish,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "zsh" => Shell::Zsh,
            &_ => {
                unreachable!("Unknown format type");
            }
        };

        generate(shell, &mut build_cli(), crate_name!(), &mut io::stdout());

        Ok(())
    }
}
