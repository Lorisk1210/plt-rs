//! Bars and histograms: `3d_bars`, `bars3d` and `hist3d`.

use plt::axes::BarOptions;
use plt::colors::Color;
use plt::figure::Figure;
use plt::mplot3d::{Axes3DOptions, Axis3, Bar3DOptions, Boxes};
use plt::patches::PatchProps;
use plt::stats::{AxisBins, Histogram2dOptions, histogram2d};

use super::{SEED, label_xyz};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/3d_bars", bars_3d).sized(8.0, 3.0, 100.0),
    FigureCase::new("gallery_3d/bars3d", bars3d),
    FigureCase::new("gallery_3d/hist3d", hist3d),
];

fn bars_3d(fig: &Figure) -> CaseResult {
    // np.meshgrid(np.arange(4), np.arange(5)), raveled.
    let (x, y): (Vec<f64>, Vec<f64>) = (0..5)
        .flat_map(|row| (0..4).map(move |col| (f64::from(col), f64::from(row))))
        .unzip();
    let top: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let boxes = Boxes::new(&x, &y, [0.0], [1.0], [1.0], &top);
    for (index, shade, title) in [(1, true, "Shaded"), (2, false, "Not Shaded")] {
        let ax = fig.add_subplot_3d(1, 2, index, Axes3DOptions::default())?;
        let options = Bar3DOptions {
            shade,
            ..Bar3DOptions::default()
        };
        ax.bar3d(boxes.clone(), options)?;
        ax.set_title(title);
    }
    Ok(())
}

fn bars3d(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let ax = fig.subplot_3d();
    let yticks = [3.0, 2.0, 1.0, 0.0];
    let xs: Vec<f64> = (0..20).map(f64::from).collect();
    for (c, k) in ["r", "g", "b", "y"].into_iter().zip(yticks) {
        let ys = rng.uniform(20, 0.0, 1.0);
        let mut color = vec![Color::from(c); xs.len()];
        color[0] = "c".into();
        let options = BarOptions {
            color,
            props: PatchProps::default().alpha(0.8),
            ..BarOptions::default()
        };
        ax.bar(&xs, &ys, [k], Axis3::Y, options)?;
    }
    label_xyz(&ax, ["X", "Y", "Z"]);
    ax.yaxis().set_ticks(yticks);
    Ok(())
}

fn hist3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let samples = SpecRng::new(SEED).uniform(200, 0.0, 4.0);
    let (x, y) = samples.split_at(100);
    let options = Histogram2dOptions {
        bins: [AxisBins::Count(4), AxisBins::Count(4)],
        range: [Some((0.0, 4.0)), Some((0.0, 4.0))],
        ..Histogram2dOptions::default()
    };
    let hist = histogram2d(x, y, options)?;
    // np.meshgrid(xedges[:-1] + 0.25, yedges[:-1] + 0.25, indexing="ij"),
    // raveled like the counts.
    let (xpos, ypos): (Vec<f64>, Vec<f64>) = hist.xedges[..4]
        .iter()
        .flat_map(|x| hist.yedges[..4].iter().map(move |y| (x + 0.25, y + 0.25)))
        .unzip();
    let boxes = Boxes::new(&xpos, &ypos, [0.0], [0.5], [0.5], hist.counts.as_slice());
    ax.bar3d(boxes, Bar3DOptions::default())?;
    Ok(())
}
