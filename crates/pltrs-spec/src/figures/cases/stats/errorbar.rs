//! Error bar cases of `figures/cases_stats.py`.

use pltrs::axes::{ErrorSize, ErrorbarOptions};
use pltrs::colors::Color;
use pltrs::lines::{LineProps, LineStyle, MarkEvery};
use pltrs::numeric::{arange, linspace};

use super::map;
use crate::figures::FigureCase;

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
                lower: map(&x, |x| 0.1 * x).into(),
                upper: vec![0.3; 6].into(),
            }),
            yerr: Some(ErrorSize::Asymmetric {
                lower: map(&x, |x| 0.5 * x).into(),
                upper: x.clone().into(),
            }),
            fmt: "o".parse()?,
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
            fmt: "s--".parse()?,
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
            fmt: "none".parse()?,
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
];
