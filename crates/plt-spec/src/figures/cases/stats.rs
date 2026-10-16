//! `figures/cases_stats.py`.

use plt::axes::{ErrorSize, ErrorbarOptions, FillBetweenOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::lines::{LineProps, LineStyle, MarkEvery};
use plt::numeric::{arange, linspace};

use crate::figures::FigureCase;

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/errorbar_yerr_symmetric", |fig| {
        let x = arange(0.0, 8.0, 1.0);
        let options = ErrorbarOptions {
            yerr: Some(map(&x, |x| 0.1 * (x + 1.0)).into()),
            ..ErrorbarOptions::default()
        };
        fig.subplot()
            .errorbar(&x, map(&x, |x| (x + 1.0).sqrt()), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/errorbar_xerr_yerr_asymmetric", |fig| {
        let x = arange(1.0, 7.0, 1.0);
        let options = ErrorbarOptions {
            xerr: Some(ErrorSize::Asymmetric {
                lower: map(&x, |x| 0.1 * x),
                upper: vec![0.3; 6],
            }),
            yerr: Some(ErrorSize::Asymmetric {
                lower: map(&x, |x| 0.5 * x),
                upper: x.clone(),
            }),
            fmt: "o".into(),
            ..ErrorbarOptions::default()
        };
        fig.subplot()
            .errorbar(&x, map(&x, |x| x.powf(1.5)), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/errorbar_capsize_fmt_elinewidth", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 6.0, 1.0);
        let styled = ErrorbarOptions {
            yerr: Some(0.8.into()),
            fmt: "s--".into(),
            capsize: Some(6.0),
            capthick: Some(2.0),
            elinewidth: Some(3.0),
            ecolor: Some(Color::from("tab:gray")),
            line: LineProps::default().markersize(7.0),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, &x, styled)?;
        let bars_only = ErrorbarOptions {
            yerr: Some(0.5.into()),
            fmt: "none".into(),
            ecolor: Some(Color::from("tab:red")),
            elinestyle: Some(LineStyle::Dotted),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |x| x + 4.0), bars_only)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/errorbar_limits", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 10.0, 1.0);
        let wave = |offset: f64| map(&x, |x| (x / 2.0).sin() + offset);
        let upper = ErrorbarOptions {
            yerr: Some(0.2.into()),
            uplims: vec![true],
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, wave(0.0), upper)?;
        let lower = ErrorbarOptions {
            yerr: Some(0.2.into()),
            lolims: vec![true],
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, wave(1.0), lower)?;
        let mixed = ErrorbarOptions {
            yerr: Some(0.2.into()),
            xerr: Some(0.3.into()),
            lolims: x.iter().map(|x| x % 2.0 == 0.0).collect(),
            xuplims: x.iter().map(|&x| x > 5.0).collect(),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, wave(2.0), mixed)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/errorbar_errorevery", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 4.0, 40);
        let every_fifth = ErrorbarOptions {
            yerr: Some(0.05.into()),
            errorevery: Some(MarkEvery::Every { start: 0, step: 5 }),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |x| (-x).exp()), every_fifth)?;
        let offset = ErrorbarOptions {
            yerr: Some(0.05.into()),
            errorevery: Some(MarkEvery::Every { start: 2, step: 6 }),
            capsize: Some(3.0),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |x| (-x).exp() + 0.5), offset)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
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
];

/// `FB_X = np.linspace(0, 4 * np.pi, 60)`.
fn fb_x() -> Vec<f64> {
    linspace(0.0, 4.0 * std::f64::consts::PI, 60)
}
