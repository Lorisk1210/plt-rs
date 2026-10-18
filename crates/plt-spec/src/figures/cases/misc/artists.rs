//! Figure-level artist and property cycle cases of
//! `figures/cases_axes_misc.py`.

use plt::artist::{AggFilter, FilterImage, Filtered};
use plt::patches::{Circle, Rectangle};
use plt::prelude::*;
use plt::rc::{CycleValue, Cycler};
use plt::transforms::Transform;

use super::wave;
use crate::figures::FigureCase;

/// `_fade_and_shift`: inverted colours at half opacity, `dpi / 10` to the
/// right and `dpi / 20` down.
fn fade_and_shift(image: &FilterImage, dpi: f64) -> Filtered {
    let mut out = image.clone();
    for pixel in out.pixels_mut() {
        for channel in &mut pixel[..3] {
            *channel = 1.0 - *channel;
        }
        pixel[3] *= 0.5;
    }
    Filtered {
        image: out,
        offset: (dpi / 10.0, -dpi / 20.0),
    }
}

/// The mean of three pixels, summed in numpy's order.
fn mean3(a: [f64; 4], b: [f64; 4], c: [f64; 4]) -> [f64; 4] {
    [0, 1, 2, 3].map(|i| (a[i] + b[i] + c[i]) / 3.0)
}

/// `_box_blur`: a separable 3-pixel box blur over a 3-pixel transparent
/// border.
fn box_blur(image: &FilterImage, _dpi: f64) -> Filtered {
    let (width, height) = (image.width() + 6, image.height() + 6);
    let padded = |x: usize, y: usize| {
        x.checked_sub(3)
            .zip(y.checked_sub(3))
            .and_then(|(x, y)| image.pixel(x, y))
            .unwrap_or([0.0; 4])
    };
    let rows: Vec<[f64; 4]> = (0..height - 2)
        .flat_map(|y| {
            (0..width).map(move |x| mean3(padded(x, y), padded(x, y + 1), padded(x, y + 2)))
        })
        .collect();
    let blurred = (0..height - 2)
        .flat_map(|y| {
            let row = &rows[y * width..(y + 1) * width];
            (0..width - 2).map(move |x| mean3(row[x], row[x + 1], row[x + 2]))
        })
        .collect();
    Filtered {
        image: FilterImage::new(width - 2, height - 2, blurred).expect("one pixel per position"),
        offset: (-2.0, -2.0),
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/fig_add_artist", |fig| {
        fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
        fig.add_line(
            &[0.05, 0.95],
            &[0.5, 0.5],
            LineProps::default().color("tab:red").linewidth(3.0),
        )?;
        fig.add_patch(Circle::new((0.5, 0.5), 0.1)).update_props(
            PatchProps::default()
                .facecolor("none")
                .edgecolor("k")
                .linewidth(2.0),
        );
        fig.add_patch(Rectangle::new((10.0, 10.0), 50.0, 30.0))
            .set_facecolor("gold")
            .set_transform(Transform::Display);
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/set_prop_cycle", |fig| {
        let ax = fig.subplot();
        let colors = ["tab:red", "tab:green", "k"].map(|c| CycleValue::Color(Color::from(c)));
        let styles =
            [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted].map(CycleValue::LineStyle);
        let widths = [1.0, 2.0, 3.0].map(CycleValue::Number);
        let cycle = Cycler::new("color", colors.to_vec())
            .zip(Cycler::new("linestyle", styles.to_vec()))?
            .zip(Cycler::new("linewidth", widths.to_vec()))?;
        ax.set_prop_cycle(Some(cycle));
        for i in 0..5 {
            let shift = f64::from(i);
            ax.plot_y(wave(20).into_iter().map(|y| y + shift).collect::<Vec<_>>());
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/agg_filter", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 2.0 * std::f64::consts::PI, 50);
        let sine: Vec<f64> = x.iter().map(|v| v.sin()).collect();
        let line = |width: f64| LineProps::default().linewidth(width).color("tab:blue");
        ax.plot_with(x.clone(), sine.clone(), line(4.0))?
            .set_agg_filter(Some(AggFilter::new(fade_and_shift)));
        ax.plot_with(x, sine, line(2.0))?;
        let blur = AggFilter::new(box_blur);
        let bars = ax.bar(
            [1.0, 3.0, 5.0],
            [0.5, -0.4, 0.8],
            BarOptions {
                color: vec![Color::from("tab:orange")],
                ..BarOptions::default()
            },
        )?;
        for bar in &bars.patches {
            bar.set_agg_filter(Some(blur.clone()));
        }
        ax.text(3.0, -0.8, "filtered")
            .set_fontsize(14.0)
            .set_agg_filter(Some(blur));
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
