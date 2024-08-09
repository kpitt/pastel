use clap::{builder, crate_description, crate_name, crate_version, Arg, ArgAction, Command};

// Only include `colorpicker_tools` for normal builds (not when compiling `build.rs` where
// the module machinery does not work)
#[cfg(pastel_normal_build)]
use crate::colorpicker_tools::COLOR_PICKER_TOOL_NAMES;

const SORT_OPTIONS: &[&str] = &["brightness", "luminance", "hue", "chroma", "random"];
const DEFAULT_SORT_ORDER: &str = "hue";

pub fn build_cli() -> Command {
    let color_arg_help =
        "Colors can be specified in many different formats, such as '#RRGGBB', RRGGBB, \
         '#RGB', 'rgb(…, …, …)', 'hsl(…, …, …)', 'gray(…)', or simply by the name of the \
         color. The identifier '-' can be used to read a single color from standard input. \
         Also, the special identifier 'pick' can be used to run an external color picker \
         to choose a color. If no color argument is specified, colors will be read from \
         standard input.";
    let color_arg_long_help = color_print::cstr!(
        "Examples (all of these specify the same color):\
         \n  - <cyan>lightslategray</>\
         \n  - <cyan>'#778899'</>\
         \n  - <cyan>778899</>\
         \n  - <cyan>789</>\
         \n  - <cyan>'rgb(119, 136, 153)'</>\
         \n  - <cyan>'119,136,153'</>\
         \n  - <cyan>'hsl(210, 14.3%, 53.3%)'</>\n\
         \n\
         Alpha transparency is also supported:\
         \n  - <cyan>'#77889980'</>\
         \n  - <cyan>'rgba(119, 136, 153, 0.5)'</>\
         \n  - <cyan>'hsla(210, 14.3%, 53.3%, 50%)'</>"
    );
    let color_arg = Arg::new("color")
        .help(color_arg_help)
        .long_help(format!(
            "{color_arg_help}\n\
             \n\
             {color_arg_long_help}"
        ))
        .value_name("COLOR")
        .action(ArgAction::Append)
        .num_args(0..)
        .trailing_var_arg(true);

    let colorspace_arg = Arg::new("colorspace")
        .long("colorspace")
        .short('s')
        .value_name("NAME")
        .help("Colorspace in which to interpolate")
        .value_parser(["Lab", "LCh", "RGB", "HSL", "HWB"])
        .ignore_case(true)
        .default_value("Lab");

    Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .color(clap::ColorChoice::Auto)
        .allow_negative_numbers(true)
        .dont_collapse_args_in_usage(true)
        .max_term_width(100)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("color")
                .alias("colour")
                .alias("take")
                .alias("show")
                .alias("display")
                .about("Display information about the given color")
                .long_about(color_print::cstr!(
                    "Show and display some information about the given color(s).\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel color 556270 4ecdc4 c7f484 ff6b6b c44d58</>"
                ))
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("list")
                .about("Show a list of available color names")
                .arg(
                    Arg::new("sort-order")
                        .short('s')
                        .long("sort")
                        .help("Sort order")
                        .value_parser(builder::PossibleValuesParser::new(SORT_OPTIONS))
                        .default_value(DEFAULT_SORT_ORDER)
                        .value_name("ORDER")
                ),
        )
        .subcommand(
            Command::new("random")
                .about("Generate a list of random colors")
                .long_about(color_print::cstr!(
                    "Generate a list of random colors.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel random -n 20 --strategy lch_hue</>"
                ))
                .arg(
                    Arg::new("strategy")
                        .long("strategy")
                        .short('s')
                        .help("Randomization strategy: vivid, rgb, gray, lch_hue \
                            [default: vivid]")
                        .long_help(color_print::cstr!(
                            "Randomization strategy:\
                             \n  vivid:    random hue, limited saturation and lightness values\
                             \n  rgb:      samples uniformly in RGB space\
                             \n  gray:     random gray tone (uniform)\
                             \n  lch_hue:  random hue, fixed lightness and chroma\n\
                             \n\
                             [default: vivid]"
                        ))
                        .value_parser(["vivid", "rgb", "gray", "lch_hue"])
                        .hide_default_value(true)
                        .hide_possible_values(true)
                        .default_value("vivid")
                        .value_name("STRATEGY")
                )
                .arg(
                    Arg::new("number")
                        .long("number")
                        .short('n')
                        .help("Number of colors to generate")
                        .default_value("10")
                        .value_name("COUNT"),
                ),
        )
        .subcommand(
            Command::new("distinct")
                .about("Generate a set of visually distinct colors")
                .long_about("Generate a set of visually distinct colors by maximizing \
                             the perceived color difference between pairs of colors.\n\n\
                             The default parameters for the optimization procedure \
                             (simulated annealing) should work fine for up to 10-20 colors.")
                .arg(
                    Arg::new("number")
                        .help("Number of distinct colors in the set")
                        .default_value("10")
                        .value_name("COUNT"),
                )
                .arg(
                    Arg::new("metric")
                        .long("metric")
                        .short('m')
                        .help("Distance metric for color distances")
                        .long_help("Distance metric to use for computing mutual color distances. \
                            The CIEDE2000 metric is more accurate, but also much slower.")
                        .value_parser(["CIEDE2000", "CIE76"])
                        .value_name("METRIC")
                        .default_value("CIE76")
                )
                .arg(
                    Arg::new("print-minimal-distance")
                        .long("print-minimal-distance")
                        .action(ArgAction::SetTrue)
                        .help("Only show the optimized minimal distance")
                        .hide(true)
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(ArgAction::SetTrue)
                        .help("Print simulation output to STDERR")
                ).
                arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("sort-by")
                .about("Sort colors by the given property")
                .long_about(color_print::cstr!(
                    "Sort a list of colors by the given property.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel random -n 20 | pastel sort-by hue | pastel format hex</>"
                ))
                .alias("sort")
                .arg(
                    Arg::new("sort-order")
                        .help("Sort order")
                        .value_parser(builder::PossibleValuesParser::new(SORT_OPTIONS))
                        .default_value(DEFAULT_SORT_ORDER)
                        .value_name("ORDER")
                )
                .arg(
                    Arg::new("reverse")
                        .long("reverse")
                        .short('r')
                        .action(ArgAction::SetTrue)
                        .help("Reverse the sort order"),
                )
                .arg(
                    Arg::new("unique")
                        .long("unique")
                        .short('u')
                        .action(ArgAction::SetTrue)
                        .help("Remove duplicate colors (equality is determined via RGB values)"),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("pick")
                .about("Interactively choose a color using an external color picker tool")
                .long_about(color_print::cstr!(
                    "Use an external color picker tool to interactively choose a color. \
                     Prints a spectrum of colors that can be used to choose a color if \
                     the color picker tool provides a pipette (eyedropper) function.\n\
                     \n\
                     This command requires a supported external tool:\
                     \n  - <bold>gpick</> (<cyan>https://github.com/thezbyg/gpick</>)\
                     \n  - <bold>xcolor</> (<cyan>https://github.com/Soft/xcolor</>)\
                     \n  - <bold>wcolor</> (<cyan>https://github.com/Elvyria/wcolor</>)\
                     \n  - <bold>grabc</> (<cyan>https://www.muquit.com/muquit/software/grabc/grabc.html</>)\
                     \n  - <bold>colorpicker</> (<cyan>https://github.com/Jack12816/colorpicker</>)\
                     \n  - <bold>chameleon</> (<cyan>https://github.com/seebye/chameleon</>)\
                     \n  - <bold>KColorChooser</> (<cyan>https://kde.org/applications/graphics/org.kde.kcolorchooser</>)\
                     \n  - <bold>zenity</> (<cyan>https://wiki.gnome.org/Projects/Zenity</>)\
                     \n  - <bold>yad</> (<cyan>https://github.com/v1cont/yad</>)\
                     \n  - <bold>macOS</> (built-in color picker)\n\
                     \n\
                     Use the global '<cyan,bold>--color-picker</>' option to select a specific color picker tool. \
                     Run `<cyan,bold>pastel help</>` for more detailed information."
                ))
                .arg(
                    Arg::new("count")
                        .help("Number of colors to pick")
                        .default_value("1")
                        .value_name("COUNT")
                )
        )
        .subcommand(
            Command::new("format")
                .about("Convert a color to the given format")
                .long_about(color_print::cstr!(
                    "Convert the given color(s) to a specific format.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel random -n 20 | pastel format rgb</>"
                ))
                .arg(
                    Arg::new("type")
                        .help("Output format type. Note that the 'ansi-*-escapecode' formats print \
                               ansi escape sequences to the terminal that will not be visible \
                               unless something else is printed in addition.")
                        .value_parser(["rgb", "rgb-float", "hex",
                                       "hsl", "hsl-hue", "hsl-saturation", "hsl-lightness",
                                       "hsv", "hsv-hue", "hsv-saturation", "hsv-value",
                                       "hwb", "hwb-hue", "hwb-whiteness", "hwb-blackness",
                                       "lch", "lch-lightness", "lch-chroma", "lch-hue",
                                       "lab", "lab-a", "lab-b",
                                       "luminance", "brightness",
                                       "ansi-8bit", "ansi-24bit",
                                       "ansi-8bit-escapecode", "ansi-24bit-escapecode",
                                       "cmyk", "name"])
                        .ignore_case(true)
                        .default_value("hex")
                        .value_name("FORMAT")
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("paint")
                .about("Print colored text using ANSI escape sequences")
                .arg(
                    Arg::new("color")
                        .help("The foreground color. Use '-' to read the color from STDIN.")
                        .value_name("COLOR")
                        .required(true),
                )
                .arg(
                    Arg::new("text")
                        .help("The text to be printed in color. If no argument is given, \
                               the input is read from STDIN.")
                        .value_name("TEXT")
                        .action(ArgAction::Append)
                )
                .arg(
                    Arg::new("on")
                        .short('o')
                        .long("on")
                        .help("Use the specified background color")
                        .value_name("COLOR"),
                )
                .arg(
                    Arg::new("bold")
                        .short('b')
                        .long("bold")
                        .action(ArgAction::SetTrue)
                        .help("Print the text in bold face"),
                )
                .arg(
                    Arg::new("italic")
                        .short('i')
                        .long("italic")
                        .action(ArgAction::SetTrue)
                        .help("Print the text in italic font"),
                )
                .arg(
                    Arg::new("underline")
                        .short('u')
                        .long("underline")
                        .action(ArgAction::SetTrue)
                        .help("Draw a line below the text"),
                )
                .arg(
                    Arg::new("no-newline")
                        .short('n')
                        .long("no-newline")
                        .action(ArgAction::SetTrue)
                        .help("Do not print a trailing newline character"),
                ),
        )
        .subcommand(
            Command::new("gradient")
                .about("Generate an interpolating sequence of colors")
                .long_about(color_print::cstr!(
                    "Generate a sequence of colors that interpolates between the specified colors.\n\
                     The interpolation is performed in the specified color space.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel gradient --colorspace=HSL ffffcc fd8d3c</>\
                     \n  <cyan,bold>pastel gradient 555ee4 white d84341 -n 15</>"
                ))
                .arg(
                    Arg::new("color")
                        .value_name("color")
                        .help("Color stops in the color gradient")
                        .action(ArgAction::Append)
                        .required(true),
                )
                .arg(
                    Arg::new("number")
                        .long("number")
                        .short('n')
                        .help("Number of colors to generate")
                        .default_value("10")
                        .value_name("COUNT"),
                )
                .arg(
                    colorspace_arg.clone()
                )
        )
        .subcommand(
            Command::new("mix")
                .about("Mix two colors in the given colorspace")
                .long_about(color_print::cstr!(
                    "Create new colors by interpolating between two colors in the given colorspace.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel mix --colorspace=RGB red blue</>"
                ))
                .arg(
                    colorspace_arg.clone()
                )
                .arg(
                    Arg::new("fraction")
                        .long("fraction")
                        .short('f')
                        .help("Fraction of base color to mix in [between 0.0 and 1.0]")
                        .default_value("0.5")
                        .value_name("FRACTION")
                )
                .arg(
                    Arg::new("base")
                        .value_name("BASE")
                        .help("The base color which will be mixed with the other colors")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("colorblind")
                .about("Simulate a color under a certain colorblindness profile")
                .long_about(color_print::cstr!(
                    "Convert the given color to how it would look to a person with protanopia, \
                     deuteranopia, or tritanopia.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel distinct 3 | pastel colorblind deuter</>"
                ))
                .arg(
                    Arg::new("type")
                        .help("The type of colorblindness that should be simulated (protanopia, \
                               deuteranopia, tritanopia)")
                        .value_parser(["prot", "deuter", "trit"])
                        .value_name("TYPE")
                        .ignore_case(true)
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("set")
                .about("Set a color property to a specific value")
                .long_about(color_print::cstr!(
                    "Set the given property to a specific value.\n\n\
                     <green,bold>Example:</>\
                     \n  <cyan,bold>pastel random | pastel set luminance 0.9</>"
                ))
                .arg(
                    Arg::new("property")
                        .help("The property that should be changed")
                        .value_parser(["lightness", "hue", "chroma",
                                       "lab-a", "lab-b",
                                       "red", "green", "blue",
                                       "hsl-hue", "hsl-saturation", "hsl-lightness",
                                       "hwb-hue", "hwb-whiteness", "hwb-blackness"])
                        .value_name("PROPERTY")
                        .ignore_case(true)
                        .required(true),
                )
                .arg(
                    Arg::new("value")
                        .help("The new numerical value of the property")
                        .value_name("VALUE")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("saturate")
                .long_about(
                    "Increase the saturation of a color by adding a certain amount to the HSL \
                     saturation channel. If the amount is negative, the color will be desaturated \
                     instead.",
                )
                .about("Increase color saturation by a specified amount")
                .arg(
                    Arg::new("amount")
                        .help("Amount of saturation to add [between 0.0 and 1.0]")
                        .value_name("AMOUNT")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("desaturate")
                .long_about(
                    "Decrease the saturation of a color by subtracting a certain amount from the \
                     HSL saturation channel. If the amount is negative, the color will be saturated \
                     instead.",
                )
                .about("Decrease color saturation by a specified amount")
                .arg(
                    Arg::new("amount")
                        .help("Amount of saturation to subtract [between 0.0 and 1.0]")
                        .value_name("AMOUNT")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("lighten")
                .long_about(
                    "Lighten a color by adding a certain amount to the HSL lightness channel. \
                     If the amount is negative, the color will be darkened.",
                )
                .about("Lighten color by a specified amount")
                .arg(
                    Arg::new("amount")
                        .help("Amount of lightness to add [between 0.0 and 1.0]")
                        .value_name("AMOUNT")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("darken")
                .long_about(
                    "Darken a color by subtracting a certain amount from the lightness channel. \
                     If the amount is negative, the color will be lightened.",
                )
                .about("Darken color by a specified amount")
                .arg(
                    Arg::new("amount")
                        .help("Amount of lightness to subtract [between 0.0 and 1.0]")
                        .value_name("AMOUNT")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("rotate")
                .about("Rotate the hue channel by the specified angle")
                .long_about(
                    "Rotate the HSL hue channel of a color by the specified angle (in \
                     degrees). A rotation by 180° returns the complementary color. A \
                     rotation by 360° returns to the original color.",
                )
                .arg(
                    Arg::new("degrees")
                        .help("Angle by which to rotate (in degrees, can be negative)")
                        .value_name("ANGLE")
                        .required(true),
                )
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("complement")
                .about("Get the complementary color (hue rotated by 180°)")
                .long_about(color_print::cstr!(
                    "Compute the complementary color by rotating the HSL hue channel by 180°.\n\
                     \n\
                     This command is equivalent to `<cyan,bold>pastel rotate 180</> <cyan>[COLOR]...</>`."
                ))
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("gray")
                .about("Create a gray tone from a given lightness")
                .long_about("Create a gray tone from a given lightness value.")
                .arg(
                    Arg::new("lightness")
                        .help("Lightness of the gray tone [between 0.0 and 1.0]")
                        .value_name("LIGHTNESS")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("to-gray")
                .about("Completely desaturate a color (preserving luminance)")
                .long_about(color_print::cstr!(
                    "Completely desaturate the given color while preserving the luminance.\n\
                     \n\
                     For a definition of 'luminance', see:\
                     \n  <cyan>https://www.w3.org/TR/2008/REC-WCAG20-20081211/#relativeluminancedef</>"
                ))
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("textcolor")
                .about("Get a readable text color for the given background color")
                .long_about("Return a readable foreground text color (either black or white) for a \
                            given background color. This can also be used in the opposite way, \
                            i.e. to create a background color for a given text color.")
                .arg(color_arg.clone()),
        )
        .subcommand(
            Command::new("colorcheck")
                .about("Check if your terminal emulator supports 24-bit colors"),
        )
        .arg(
            Arg::new("color-mode")
                .long("color-mode")
                .short('m')
                .value_name("MODE")
                .help("Specify the terminal color mode: 24bit, 8bit, off, *auto*")
                .value_parser(["24bit", "8bit", "off", "auto"])
                .default_value(if output_vt100::try_init().is_ok() {"auto"} else {"off"})
                .hide_possible_values(true)
                .hide_default_value(true)
        )
        .arg(
            Arg::new("force-color")
                .short('f')
                .long("force-color")
                .action(ArgAction::SetTrue)
                .help("Alias for --color-mode=24bit")
        )
        .arg(
            Arg::new("color-picker")
                .long("color-picker")
                .value_parser(builder::PossibleValuesParser::new(COLOR_PICKER_TOOL_NAMES.iter()))
                .value_name("TOOL")
                .ignore_case(true)
                .help("Use a specific tool to pick the colors")
        )
}

#[test]
fn verify_cmd() {
    build_cli().debug_assert();
}
