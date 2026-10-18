//! Style sheets: the same plot in four of matplotlib's built-in styles.

use plt::axes::{BarOptions, ScatterOptions};
use plt::figure::Figure;
use plt::numeric::linspace;
use plt::style;

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "style_ggplot",
        caption: "ggplot",
        build: || styled("ggplot"),
    },
    Entry {
        name: "style_seaborn",
        caption: "seaborn-v0_8-darkgrid",
        build: || styled("seaborn-v0_8-darkgrid"),
    },
    Entry {
        name: "style_dark",
        caption: "dark_background",
        build: || styled("dark_background"),
    },
    Entry {
        name: "style_fivethirtyeight",
        caption: "fivethirtyeight",
        build: || styled("fivethirtyeight"),
    },
];

/// Lines, a scatter and bars drawn in the style sheet `name`.
fn styled(name: &str) -> Built {
    style::context(name, || {
        let fig = Figure::with_size(6.4, 3.6, 100.0);
        let grid = fig.subplots(1, 2)?;
        let x = linspace(0.0, 10.0, 100);
        for shift in [0.0, 1.0, 2.0] {
            let y: Vec<f64> = x.iter().map(|x| (x + shift).sin() + shift).collect();
            grid[(0, 0)].plot(x.clone(), y)?;
        }
        let mut samples = Samples::new(5);
        let px = samples.normals(60, 0.0, 1.0);
        let py = samples.normals(60, 0.0, 1.0);
        grid[(0, 0)].scatter_with(px, py, ScatterOptions::default())?;
        grid[(0, 1)].bar(
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            [3.0, 5.0, 2.0, 6.0, 4.0],
            BarOptions::default(),
        )?;
        grid[(0, 0)].set_title(name);
        Ok(fig)
    })?
}
