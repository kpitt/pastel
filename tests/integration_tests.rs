use assert_cmd::Command;

fn pastel() -> Command {
    let mut cmd = Command::cargo_bin("pastel").unwrap();
    cmd.env_remove("PASTEL_COLOR_MODE");
    cmd
}

#[test]
fn color_reads_colors_from_args() {
    pastel()
        .arg("color")
        .arg("red")
        .assert()
        .success()
        .stdout("xyz(0.41239079926596,0.21263900587151,0.019330818715592)\n");

    pastel()
        .arg("color")
        .arg("red")
        .arg("blue")
        .assert()
        .success()
        .stdout("xyz(0.41239079926596,0.21263900587151,0.019330818715592)\nxyz(0.18048078840183,0.072192315360734,0.95053215224966)\n");

    pastel().arg("color").arg("no color").assert().failure();
}

#[test]
fn color_reads_colors_from_stdin() {
    pastel()
        .arg("color")
        .write_stdin("red\nblue\n")
        .assert()
        .success()
        .stdout("xyz(0.41239079926596,0.21263900587151,0.019330818715592)\nxyz(0.18048078840183,0.072192315360734,0.95053215224966)\n");

    pastel()
        .arg("color")
        .write_stdin("no color")
        .assert()
        .failure();
}

#[test]
fn format_basic() {
    pastel()
        .arg("format")
        .arg("hex")
        .arg("red")
        .assert()
        .success()
        .stdout("#ff0000\n");

    pastel()
        .arg("format")
        .arg("rgb")
        .arg("red")
        .arg("blue")
        .assert()
        .success()
        .stdout("rgb(255, 0, 0)\nrgb(0, 0, 255)\n");
}

#[test]
fn pipe_into_format_command() {
    let first = pastel()
        .arg("color")
        .arg("red")
        .arg("teal")
        .arg("hotpink")
        .assert()
        .success();

    pastel()
        .arg("format")
        .arg("name")
        .write_stdin(String::from_utf8(first.get_output().stdout.clone()).unwrap())
        .assert()
        .success()
        .stdout("red\nteal\nhotpink\n");
}

#[test]
fn sort_by_basic() {
    pastel()
        .arg("sort-by")
        .arg("luminance")
        .arg("gray")
        .arg("white")
        .arg("black")
        .assert()
        .success()
        .stdout("xyz(0,0,0)\nxyz(0.20516589174959,0.2158605001139,0.23508455073195)\nxyz(0.95045592705167,1,1.0890577507599)\n");
}

#[test]
fn set_basic() {
    pastel()
        .arg("set")
        .arg("hsl-hue")
        .arg("120")
        .arg("red")
        .assert()
        .success()
        .stdout("xyz(0.35758433938388,0.71516867876776,0.11919477979463)\n");

    pastel()
        .arg("set")
        .arg("hsl-saturation")
        .arg("0.1")
        .arg("red")
        .assert()
        .success()
        .stdout("xyz(0.20038962053427,0.19034136215324,0.18763277419037)\n");

    pastel()
        .arg("set")
        .arg("hsl-lightness")
        .arg("0.5")
        .arg("white")
        .assert()
        .success()
        .stdout("xyz(0.20343667060424,0.21404114048223,0.23310316302366)\n");
}
