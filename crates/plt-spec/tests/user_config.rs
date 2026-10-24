//! The harness ignores the developer's configuration: a `pltrc` and a
//! user style sheet in `PLTCONFIGDIR` change what plt-rs draws by default
//! (wider lines, and wider ones still in the `classic` style), but not the
//! facts or pixels of figure cases in the default and in the `classic`
//! style.
//!
//! A test cannot change the environment of its own process, so the parent
//! test writes a configuration directory and runs the child test in a new
//! process of this test executable, with `PLTCONFIGDIR` pointing there.

use std::fs;
use std::process::Command;

use plt_spec::figures::{self, CaseVerdict};

/// The environment variable telling the child test it runs for the parent.
const CHILD: &str = "PLT_SPEC_USER_CONFIG_CHILD";

/// Figure cases with lines, in the default style and in `classic`.
const CASES: [&str; 2] = ["lines/xy_two_series", "mpl_tests_lines/linestyle_variants"];

/// The prefix of the lines the child prints its pixel reports on.
const REPORT: &str = "REPORT ";

/// Checks the facts of [`CASES`] and describes their pixel reports.
fn reports() -> Result<Vec<String>, String> {
    let cases: Vec<_> = figures::translated_cases()
        .into_iter()
        .filter(|case| CASES.contains(&case.name))
        .collect();
    assert_eq!(cases.len(), CASES.len());
    cases
        .iter()
        .map(|case| {
            assert_eq!(figures::judge(case), CaseVerdict::Pass, "{}", case.name);
            let report = figures::pixel_report(case)?;
            assert!(report.rms <= 1.0, "{}: {report}", case.name);
            Ok(format!("{} {report:?}", case.name))
        })
        .collect()
}

#[test]
fn the_harness_ignores_the_user_configuration() {
    let root = std::env::temp_dir().join(format!("plt-spec-user-config-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let stylelib = root.join("config/stylelib");
    fs::create_dir_all(&stylelib).unwrap();
    fs::write(root.join("config/pltrc"), "lines.linewidth: 5\n").unwrap();
    fs::write(stylelib.join("classic.mplstyle"), "lines.linewidth: 7\n").unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "child_with_a_user_configuration",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("PLTCONFIGDIR", root.join("config"))
        .env_remove("PLTRC")
        .current_dir(&root)
        .output()
        .unwrap();
    let printed = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{printed}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let configured: Vec<&str> = printed
        .lines()
        .filter_map(|line| line.strip_prefix(REPORT))
        .collect();
    assert_eq!(configured, reports().unwrap());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
#[ignore = "run by the_harness_ignores_the_user_configuration in a process of its own"]
fn child_with_a_user_configuration() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    // The configuration is live in this process...
    assert_eq!(plt::rc::params().lines_linewidth, 5.0);
    let classic = plt::style::context("classic", || plt::rc::params().lines_linewidth);
    assert_eq!(classic, Ok(7.0));
    // ...but not in the harness.
    for report in reports().unwrap() {
        println!("{REPORT}{report}");
    }
}
