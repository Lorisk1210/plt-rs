//! The harness of the degenerate-input sweeps: the data sets, rendering
//! to every output format and the check that no case panics.

use std::panic::{AssertUnwindSafe, catch_unwind};

use pltrs::figure::{Figure, SaveFormat, SaveOptions};
use pltrs::numeric::Grid;

/// What a case returns: errors are allowed, panics are not.
pub(crate) type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The data sets every case is run with.
pub(crate) fn data_sets() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("empty", vec![]),
        ("single", vec![1.0]),
        ("constant", vec![2.0; 5]),
        ("nan", vec![f64::NAN; 5]),
        (
            "mixed",
            vec![1.0, f64::NAN, 3.0, f64::INFINITY, f64::NEG_INFINITY, 2.0],
        ),
        ("huge", vec![1e300, -1e300, 5e299, 1e308]),
        ("tiny", vec![1e-300, 2e-300, -1e-300, 5e-324]),
        ("normal", vec![0.0, 1.0, 4.0, 9.0, 16.0]),
    ]
}

/// Two rows of `y`, for the functions that need a 2-D field.
pub(crate) fn field(y: &[f64]) -> Result<Grid<f64>, Box<dyn std::error::Error>> {
    Ok(Grid::from_rows(vec![
        y.to_vec(),
        y.iter().map(|v| v * 2.0).collect(),
    ])?)
}

/// Any result as an [`Outcome`], dropping the value.
pub(crate) fn done<T, E: Into<Box<dyn std::error::Error>>>(result: Result<T, E>) -> Outcome {
    result.map(drop).map_err(Into::into)
}

/// Renders `fig` to every output format, ignoring errors.
fn render(fig: &Figure) {
    for format in [
        SaveFormat::Png,
        SaveFormat::Svg,
        SaveFormat::Pdf,
        SaveFormat::Eps,
    ] {
        let _ = fig.to_format(format, SaveOptions::default());
    }
}

/// Runs every case with `run` on a fresh figure for every data set (`x =
/// 0, 1, 2, ...`) and renders it, and fails if any panics or rejects
/// ordinary data.
pub(crate) fn assert_no_panics<C>(
    cases: &[(&'static str, C)],
    run: impl Fn(&C, &Figure, &[f64], &[f64]) -> Outcome,
) {
    let mut panics = Vec::new();
    let mut failures = Vec::new();
    for (case, item) in cases {
        for (name, y) in data_sets() {
            let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let fig = Figure::with_size(2.0, 1.5, 30.0);
                let result = run(item, &fig, &x, &y).map_err(|err| err.to_string());
                render(&fig);
                result
            }));
            match outcome {
                Err(_) => panics.push(format!("{case} with {name} data")),
                // Ordinary data must plot, so that the sweep exercises
                // every function rather than its argument checks.
                Ok(Err(err)) if name == "normal" => failures.push(format!("{case}: {err}")),
                Ok(_) => {}
            }
        }
    }
    assert!(panics.is_empty(), "panicked: {panics:#?}");
    assert!(
        failures.is_empty(),
        "failed on ordinary data: {failures:#?}"
    );
}
