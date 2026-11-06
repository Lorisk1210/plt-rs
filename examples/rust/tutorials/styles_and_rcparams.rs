//! matplotlib's "Customizing Matplotlib with style sheets and rcParams"
//! in pltrs (Rust): style sheets, composed and temporary styles, rcParams
//! set at run time, property cycles and the rc file.
//!
//! ```text
//! cargo run -p pltrs-examples --example styles_and_rcparams   # writes styles_*.png here
//! ```
#![expect(clippy::print_stdout, reason = "the tutorial reports what it made")]

use pltrs::colors::Color;
use pltrs::prelude::*;
use pltrs::rc::{self, Cycler};
use pltrs::style::{self, Style};

fn main() -> Outcome {
    // `print(plt.style.available)`
    let available = style::available();
    println!("{} style sheets: {}", available.len(), available.join(", "));

    // `plt.style.use("ggplot")`: a sheet changes the rcParams of the
    // thread from then on, so the context keeps it to this figure.
    rc::context(|| -> Outcome {
        style::use_style("ggplot")?;
        draw("styles_ggplot.png")
    })?;

    // `plt.style.use(["dark_background", {"axes.grid": True}])`: sheets,
    // files and parameters applied in order.
    rc::context(|| -> Outcome {
        style::use_styles(&[
            Style::Named("dark_background".into()),
            Style::params(&[("axes.grid", "True"), ("grid.alpha", "0.3")]),
        ])?;
        draw("styles_composed.png")
    })?;

    // `with plt.style.context("Solarize_Light2"): ...`
    style::context("Solarize_Light2", || draw("styles_temporary.png"))?;

    // `mpl.rcParams["lines.linewidth"] = 4` and `mpl.rc("lines",
    // linestyle="--")`: by key, parsed as matplotlib parses rc files, or
    // through the typed fields.
    rc::context(|| -> Outcome {
        rc::set("lines.linewidth", "4")?;
        rc::update(|params| params.lines_linestyle = LineStyle::Dashed);
        assert_eq!(rc::params().lines_linewidth, 4.0);
        draw("styles_runtime.png")
    })?;

    // `mpl.rcParams["axes.prop_cycle"] = cycler(color=["r", "g", "b", "y"])`
    rc::context(|| {
        let colors = Cycler::colors_of(["r", "g", "b", "y"].map(Color::from));
        rc::update(|params| params.axes_prop_cycle = colors);
        draw("styles_cycle.png")
    })?;

    // `mpl.matplotlib_fname()`: pltrs reads `pltrsrc`, found as
    // matplotlib finds `matplotlibrc` (PLTRSRC, the working directory,
    // the configuration directory); `mpl.rcdefaults()` is `rc::reset`.
    match pltrs::config::rc_file_path() {
        Some(path) => println!("rc file: {}", path.display()),
        None => println!("no rc file: the defaults apply"),
    }
    let entries = rc::parse_rc_text("lines.linewidth: 3  # thicker\naxes.grid: True\n")?;
    println!("an rc file with {} settings", entries.len());
    rc::reset();
    Ok(())
}

/// What the drawing steps can fail with.
type Outcome = Result<(), Box<dyn std::error::Error>>;

/// A few lines in the current style, saved as `name`.
fn draw(name: &str) -> Outcome {
    let fig = Figure::with_size(5.0, 3.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 100);
    for shift in 0..4 {
        let phase = f64::from(shift) * 0.6;
        ax.plot(&x, x.iter().map(|x| (x + phase).sin() + f64::from(shift)))?;
    }
    ax.set_title(name.trim_end_matches(".png"));
    fig.savefig(name)?;
    Ok(())
}
