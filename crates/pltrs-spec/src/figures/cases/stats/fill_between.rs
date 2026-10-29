//! `fill_between` cases of `figures/cases_stats.py`.

use pltrs::axes::FillBetweenOptions;
use pltrs::collections::CollectionProps;
use pltrs::figure::Share;
use pltrs::lines::{LineProps, StepWhere};
use pltrs::numeric::{arange, linspace};

use super::map;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/fill_between_default", |fig| {
        let x = fb_x();
        let y = map(&x, f64::sin);
        fig.subplot()
            .fill_between(&x, y, [0.0], FillBetweenOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/fill_between_two_curves_alpha", |fig| {
        let ax = fig.subplot();
        let x = fb_x();
        let y = map(&x, f64::sin);
        ax.plot_fmt(&x, &y, "k")?;
        let band = FillBetweenOptions {
            props: CollectionProps::default()
                .alpha(0.3)
                .color("tab:blue")
                .edgecolor("tab:blue")
                .linewidth(1.0),
            ..FillBetweenOptions::default()
        };
        ax.fill_between(&x, map(&y, |y| y - 0.3), map(&y, |y| y + 0.3), band)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/fill_betweenx", |fig| {
        let y = linspace(0.0, 2.0, 50);
        let wave = map(&y, |y| (4.0 * y).sin());
        let options = FillBetweenOptions {
            mask: Some(y.iter().map(|&y| y < 1.5).collect()),
            props: CollectionProps::default()
                .hatch("\\\\")
                .facecolor("lightyellow")
                .edgecolor("k"),
            ..FillBetweenOptions::default()
        };
        fig.subplot()
            .fill_betweenx(&y, &wave, map(&wave, |w| 1.5 * w + 1.0), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/fill_between_where_interpolate", |fig| {
        let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
        let x = linspace(0.0, 2.0 * std::f64::consts::PI, 13);
        let (y1, y2) = (map(&x, f64::sin), map(&x, f64::cos));
        for (ax, interpolate) in axs.as_slice().iter().zip([false, true]) {
            ax.plot_fmt(&x, &y1, "k")?;
            ax.plot_fmt(&x, &y2, "k--")?;
            for (above, color) in [(true, "green"), (false, "red")] {
                let options = FillBetweenOptions {
                    mask: Some(y1.iter().zip(&y2).map(|(a, b)| (a >= b) == above).collect()),
                    interpolate,
                    props: CollectionProps::default().color(color).alpha(0.4),
                    ..FillBetweenOptions::default()
                };
                ax.fill_between(&x, &y1, &y2, options)?;
            }
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("stats/fill_between_step", |fig| {
        let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
        let x = arange(0.0, 8.0, 1.0);
        let y = [1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 2.0, 3.0];
        let steps = [StepWhere::Pre, StepWhere::Mid, StepWhere::Post];
        for (ax, step) in axs.as_slice().iter().zip(steps) {
            let options = FillBetweenOptions {
                step: Some(step),
                props: CollectionProps::default().alpha(0.5),
                ..FillBetweenOptions::default()
            };
            ax.fill_between(&x, y, [0.0], options)?;
            ax.step(&x, y, step, LineProps::default())?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];

/// `FB_X = np.linspace(0, 4 * np.pi, 60)`.
fn fb_x() -> Vec<f64> {
    linspace(0.0, 4.0 * std::f64::consts::PI, 60)
}
