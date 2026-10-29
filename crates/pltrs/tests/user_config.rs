//! The user configuration, in a process of its own: pltrs reads `pltrsrc`
//! and the user style library of `PLTRSCONFIGDIR` when it starts, and
//! `pltrs::testing::setup` makes it ignore them.
//!
//! A test cannot change the environment of its own process, so the parent
//! test writes a configuration directory and runs the child test in a new
//! process of this test executable, with `PLTRSCONFIGDIR` pointing there.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use pltrs::lines::LineProps;
use pltrs::prelude::*;
use pltrs::rc::{RcError, RcWarningKind};
use pltrs::{config, rc, style, testing};

/// What a part of the child test returns.
type Outcome = Result<(), Box<dyn Error>>;

/// The environment variable telling the child test it runs for the parent.
const CHILD: &str = "PLTRS_USER_CONFIG_CHILD";

/// The user rc file: a visible change and two lines matplotlib skips.
const PLTRSRC: &str = "lines.linewidth: 5\nlines.color: notacolor\nno colon\n";

#[test]
fn the_user_configuration_applies_until_tests_ignore_it() {
    let root = std::env::temp_dir().join(format!("pltrs-user-config-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let stylelib = root.join("config/stylelib");
    fs::create_dir_all(&stylelib).unwrap();
    fs::write(root.join("config/pltrsrc"), PLTRSRC).unwrap();
    fs::write(stylelib.join("mine.mplstyle"), "axes.facecolor: adeade\n").unwrap();
    fs::write(stylelib.join("classic.mplstyle"), "lines.linewidth: 7\n").unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "child_with_a_user_configuration",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("PLTRSCONFIGDIR", root.join("config"))
        .env_remove("PLTRSRC")
        .current_dir(&root)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{printed}");
    assert!(printed.contains("1 passed"), "the child ran: {printed}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
#[ignore = "run by the_user_configuration_applies_until_tests_ignore_it in a process of its own"]
fn child_with_a_user_configuration() {
    let Some(config) = std::env::var_os(CHILD).and(std::env::var_os("PLTRSCONFIGDIR")) else {
        return;
    };
    let config = fs::canonicalize(config).unwrap();
    the_configuration_is_found(&config).unwrap();
    threads_start_from_the_rc_file(&config).unwrap();
    the_style_library_holds_the_user_sheets(&config).unwrap();
    testing_ignores_the_configuration().unwrap();
}

fn the_configuration_is_found(config: &Path) -> Outcome {
    assert_eq!(config::config_dir(), Ok(config.to_owned()));
    assert_eq!(config::cache_dir(), Ok(config.to_owned()));
    assert_eq!(config::rc_file_path(), Some(config.join("pltrsrc")));
    let file = config::user_rc_file()?.ok_or("no user rc file")?;
    let kinds: Vec<_> = file.warnings.iter().map(|w| &w.kind).collect();
    assert!(matches!(
        kinds[..],
        [RcWarningKind::MissingColon, RcWarningKind::InvalidValue(_)]
    ));
    assert!(!config::user_config_ignored());
    Ok(())
}

fn threads_start_from_the_rc_file(config: &Path) -> Outcome {
    let mut expected = pltrs::look::appearance().figures.params();
    expected.lines_linewidth = 5.0;
    assert_eq!(
        *rc::params(),
        expected,
        "the look's, with the rc file on top"
    );
    rc::set("lines.linewidth", "2")?;
    rc::restore_startup();
    assert_eq!(rc::params().lines_linewidth, 5.0);
    rc::reset();
    assert_eq!(rc::params().lines_linewidth, 1.5, "matplotlib's defaults");
    rc::restore_startup();

    fs::write(config.join("pltrsrc"), "lines.linewidth: 8\n")?;
    let in_new_thread = thread::spawn(|| rc::params().lines_linewidth).join();
    assert_eq!(in_new_thread.ok(), Some(5.0), "read once per process");
    Ok(())
}

fn the_style_library_holds_the_user_sheets(config: &Path) -> Outcome {
    assert_eq!(style::user_library_paths(), [config.join("stylelib")]);
    assert!(style::available().contains(&"mine".to_owned()));
    let classic_width =
        style::context("classic", || Ok::<_, RcError>(rc::params().lines_linewidth))?;
    assert_eq!(classic_width, 7.0, "the user sheet adds to the bundled one");
    let in_new_thread = thread::spawn(|| style::sheet("mine").is_some()).join();
    assert_eq!(in_new_thread.ok(), Some(true));
    Ok(())
}

/// A PNG of a line drawn with the current parameters.
fn line_png() -> Result<Vec<u8>, Box<dyn Error>> {
    let fig = Figure::with_size(2.0, 1.5, 50.0);
    fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
    Ok(fig.to_png()?)
}

fn testing_ignores_the_configuration() -> Outcome {
    pltrs::look::set_appearance(pltrs::look::Appearance::MATPLOTLIB);
    rc::restore_startup();
    let configured = line_png()?;

    testing::setup();
    assert!(config::user_config_ignored());
    assert_eq!(config::user_rc_file(), Ok(None));
    assert_eq!(rc::params().lines_linewidth, 1.5);
    assert_ne!(line_png()?, configured, "the rc file's line was wider");
    assert_eq!(style::user_library_paths(), Vec::<PathBuf>::new());
    assert!(!style::available().contains(&"mine".to_owned()));
    let classic_width =
        style::context("classic", || Ok::<_, RcError>(rc::params().lines_linewidth))?;
    assert_eq!(classic_width, 1.0);
    let in_new_thread = thread::spawn(|| {
        let user_sheet = style::sheet("mine").is_some();
        rc::restore_startup();
        (rc::params().lines_linewidth, user_sheet)
    })
    .join();
    assert_eq!(in_new_thread.ok(), Some((1.5, false)));

    let out = std::env::temp_dir().join(format!("pltrs-user-config-out-{}", std::process::id()));
    let comparison = testing::check_figures_equal(
        out.join("default_width.png"),
        0.0,
        |fig| {
            fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
            Ok(())
        },
        |fig| {
            let props = LineProps::default().linewidth(1.5);
            fig.subplot().plot_with([0.0, 1.0], [0.0, 1.0], props)?;
            Ok(())
        },
    )?;
    assert!(comparison.matches(), "{comparison}");
    fs::remove_dir_all(&out)?;
    Ok(())
}
