//! Style sheets: the same plot in plt-rs' two looks and in several of
//! matplotlib's built-in styles, and the xkcd sketch style.

use plt::axes::{BarOptions, ScatterOptions};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::pyplot;
use plt::style::{self, Style, XkcdSketch};
use plt::text::{AnnotateOptions, FancyArrowProps};
use plt::transforms::Point;

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("style_modern", "plt-modern: the modern look", || {
        styled("plt-modern")
    }),
    Entry::figure(
        "style_modern_dark",
        "plt-modern-dark: the dark look",
        || styled("plt-modern-dark"),
    ),
    Entry::figure("style_ggplot", "ggplot", || styled("ggplot")),
    Entry::figure("style_seaborn", "seaborn-v0_8-darkgrid", || {
        styled("seaborn-v0_8-darkgrid")
    }),
    Entry::figure("style_bmh", "bmh", || styled("bmh")),
    Entry::figure("style_solarize", "Solarize_Light2", || {
        styled("Solarize_Light2")
    }),
    Entry::figure("style_dark", "dark_background", || {
        styled("dark_background")
    }),
    Entry::figure("style_fivethirtyeight", "fivethirtyeight", || {
        styled("fivethirtyeight")
    }),
    Entry::figure(
        "style_petroff10",
        "petroff10: an accessible palette",
        || styled("petroff10"),
    ),
    Entry::figure("xkcd", "The xkcd sketch style", xkcd),
];

/// Lines, a scatter and bars drawn in the style sheet `name`, applied
/// over matplotlib's defaults (the `"default"` style) so that it shows as
/// designed whatever the look in force: a sheet applied over a look
/// changes only the parameters it sets.
fn styled(name: &str) -> Built {
    let styles = [Style::Named("default".into()), Style::Named(name.into())];
    style::context_with(&styles, || {
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
    })
}

/// A comic-style chart: every path is drawn with a hand-made wobble
/// (the sketch parameters), inside `pyplot::xkcd`, which restores the
/// previous parameters afterwards. Like the sheets above, it starts from
/// matplotlib's defaults.
fn xkcd() -> Built {
    style::context("default", || {
        pyplot::xkcd(XkcdSketch::default(), draw_comic)
    })
}

/// The figure of [`xkcd`], drawn in the parameters in force.
fn draw_comic() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let days = linspace(0.0, 100.0, 101);
    let health: Vec<f64> = days
        .iter()
        .map(|d| {
            if *d < 70.0 {
                1.0
            } else {
                1.0 - (d - 70.0) / 30.0
            }
        })
        .collect();
    ax.plot_with(&days, health, LineProps::default().linewidth(2.0))?;
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    ax.set_ylim(-0.3, 1.3)?;
    let mut arrow: FancyArrowProps = "->".parse()?;
    arrow.connectionstyle = "arc3,rad=-0.3".parse()?;
    let note = AnnotateOptions {
        xytext: Some(Point::new(25.0, 0.3).into()),
        arrowprops: Some(arrow.into()),
        ..AnnotateOptions::default()
    };
    ax.annotate(
        "the day I realised\nI could plot in Rust",
        Point::new(70.0, 1.0),
        note,
    )?;
    ax.set_xlabel("time").set_ylabel("my overall health");
    ax.set_title("Stove ownership, revisited");
    Ok(fig)
}
