//! `figures/cases_usetex.py`: text typeset by LaTeX (`text.usetex`).
//! Every case needs TeX and is skipped without it.

mod fonts;
mod mpl_tests;

use plt::axes::ImshowOptions;
use plt::colorbar::ColorbarOptions;
use plt::dates::{AutoDateLocator, ConciseDateFormatter, DateTime};
use plt::figure::Figure;
use plt::image::{ImageData, ScalarField};
use plt::layout::TightLayout;
use plt::numeric::Grid;
use plt::patches::BoxStyle;
use plt::patheffects::{GcUpdate, PathEffect, Stroke};
use plt::prelude::*;
use plt::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, RotationMode, TextBoxProps, VAlign,
};
use plt::transforms::Point;

use crate::figures::{CaseResult, FigureCase};

/// `TEX`: the rcParams of every case.
const TEX: &[(&str, &str)] = &[("text.usetex", "True")];

/// `usetex_case(name)`: a 3.2 × 2 in case at 100 dpi needing TeX, with
/// `text.usetex` on.
const fn usetex_case(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
        .sized(3.2, 2.0, 100.0)
        .with_rc(TEX)
        .needing_tex()
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [CASES, fonts::CASES, mpl_tests::CASES].concat()
}

/// `np.linspace(start, stop, n)`.
fn linspace(start: f64, stop: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| start + (stop - start) * i as f64 / (n - 1) as f64)
        .collect()
}

const CASES: &[FigureCase] = &[
    usetex_case("usetex/text_basic", |fig| {
        fig.text(0.05, 0.8, "Plain usetex text, 10 pt");
        fig.text(0.05, 0.6, r"Inline math $\alpha^2 + \beta_i = \gamma$")
            .set_fontsize(12.0);
        fig.text(
            0.05,
            0.4,
            r"\textbf{Bold} \textit{italic} \texttt{typewriter}",
        )
        .set_fontsize(9.0);
        fig.text(0.05, 0.15, r"Large $\sum_{n=1}^{\infty} \frac{1}{n^2}$")
            .set_fontsize(16.0);
        Ok(())
    }),
    usetex_case("usetex/math_display", |fig| {
        fig.text(
            0.5,
            0.75,
            r"$\displaystyle\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}$",
        )
        .set_halign(HAlign::Center)
        .set_fontsize(12.0);
        fig.text(
            0.5,
            0.4,
            r"$\left[\frac{\partial^2 u}{\partial t^2}\right] = c^2 \nabla^2 u$",
        )
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(12.0);
        fig.text(0.5, 0.1, r"$\sqrt[3]{x} \ne \overline{y}$, $-1$, $10^{-3}$")
            .set_halign(HAlign::Center);
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/multiline", |fig| {
        fig.text(0.3, 0.5, "foo\nbar")
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
        fig.text(0.7, 0.5, "first line\n\n$x^2$ after a blank")
            .set_halign(HAlign::Right)
            .set_valign(VAlign::Center)
            .set_multialign(Some(HAlign::Center));
        fig.text(0.5, 0.1, " ").set_halign(HAlign::Center);
        Ok(())
    }),
    usetex_case("usetex/rotation", |fig| {
        // matplotlib's test_usetex.py::test_rotation.
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
        ax.set_xlim(-0.5, 5.0)?.set_ylim(-0.5, 3.0)?;
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new())
            .set_frame_on(false);
        let first = |name: &str| name[..1].to_owned();
        let valigns = ["top", "center", "bottom", "baseline", "center_baseline"];
        for (i, va) in valigns.into_iter().enumerate() {
            for (j, ha) in ["left", "center", "right"].into_iter().enumerate() {
                for (k, angle) in [0.0, 90.0, 180.0, 270.0].into_iter().enumerate() {
                    let k = k / 2;
                    let (x, y) = (i as f64 + k as f64 / 2.0, j as f64 + k as f64 / 2.0);
                    ax.plot_fmt([x], [y], "+")?
                        .set_color(format!("C{k}"))
                        .set_markersize(20.0)
                        .set_markeredgewidth(0.5);
                    let va_name = match va {
                        "baseline" => "B".to_owned(),
                        "center_baseline" => "C".to_owned(),
                        other => first(other),
                    };
                    ax.text(
                        x,
                        y,
                        format!("$\\mathrm{{My {}{va_name} {angle}}}$", first(ha)),
                    )
                    .set_rotation(angle)
                    .set_halign(ha.parse::<HAlign>()?)
                    .set_valign(va.parse::<VAlign>()?);
                }
            }
        }
        Ok(())
    })
    .sized(6.4, 4.8, 100.0),
    usetex_case("usetex/rotation_anchor", |fig| {
        for (angle, x) in [15.0, 45.0, 120.0, 200.0]
            .into_iter()
            .zip([0.15, 0.4, 0.65, 0.85])
        {
            fig.text(x, 0.5, format!(r"$\theta = {angle}^\circ$"))
                .set_rotation(angle)
                .set_rotation_mode(RotationMode::Anchor)
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center);
        }
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/ticks", |fig| {
        let axs = fig.subplots(1, 2)?;
        let x = linspace(-2.0, 2.0, 21);
        let cubes: Vec<f64> = x.iter().map(|v| v.powi(3) * 1e4).collect();
        axs[(0, 0)].plot(&x, cubes)?;
        let shifted: Vec<f64> = x.iter().map(|v| v + 2.5).collect();
        let growth: Vec<f64> = x.iter().map(|v| (v * 2.0).exp()).collect();
        axs[(0, 1)].semilogy(shifted, growth)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    usetex_case("usetex/date_ticks", |fig| {
        let ax = fig.subplot();
        let start = DateTime::new(2020, 1, 1, 0, 0, 0, 0)?.timestamp_micros();
        let day = 86_400_000_000_i64;
        let days = (0..60)
            .step_by(6)
            .map(|i| DateTime::from_timestamp_micros(start + i * day))
            .collect::<Result<Vec<_>, _>>()?;
        let n = days.len();
        ax.plot(days, (0..n).map(|i| i as f64).collect::<Vec<_>>())?;
        ax.xaxis()
            .set_major_locator(AutoDateLocator::new())
            .set_major_formatter(ConciseDateFormatter::new());
        Ok(())
    })
    .sized(4.0, 2.0, 100.0),
    usetex_case("usetex/titles_labels", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
        ax.set_title_at(plt::axes::TitleLocation::Left, r"Title with $\mathcal{L}$");
        ax.set_title_at(plt::axes::TitleLocation::Right, "right")
            .set_fontsize(8.0);
        ax.set_xlabel(r"time $t$ (s)");
        ax.set_ylabel(r"$\|v\|^2$");
        fig.suptitle(r"\textsc{Suptitle}");
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    usetex_case("usetex/legend", |fig| {
        // matplotlib's test_usetex_with_underscore, plus labels with math.
        let ax = fig.subplot();
        ax.plot_with(
            [0.0, 1.0, 2.0, 3.0, 4.0],
            [4.0, 3.0, 2.0, 1.0, 0.0],
            LineProps::default().label("a_b"),
        )?;
        ax.plot_with(
            [0.0, 4.0],
            [1.0, 3.0],
            LineProps::default().label(r"$y = \frac{x}{2} + 1$"),
        )?;
        ax.legend(LegendOptions {
            title: Some(r"\emph{Legend}".into()),
            ..LegendOptions::default()
        })?;
        ax.text(0.0, 0.0, "foo_bar").set_usetex(true);
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/annotations", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 1.0], [0.0, 1.0])?;
        let note = AnnotateOptions {
            xytext: Some(Point::new(0.2, 0.7).into()),
            arrowprops: Some(ArrowProps::Fancy("->".parse::<FancyArrowProps>()?)),
            ..AnnotateOptions::default()
        };
        ax.annotate(r"$\max$ here", (1.0, 1.0), note)?
            .set_bbox(Some(
                TextBoxProps::new("round".parse::<BoxStyle>()?)
                    .with_patch(PatchProps::default().facecolor("w")),
            ));
        let boxed = TextBoxProps::new("square,pad=0".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().edgecolor("k").facecolor("none"));
        ax.text(0.6, 0.2, r"boxed $\epsilon$").set_bbox(Some(boxed));
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/colorbar", |fig| {
        let ax = fig.subplot();
        let values = Grid::from_fn(4, 5, |r, c| (r * 5 + c) as f64 / 7.0);
        let image = ax.imshow(
            ImageData::from(ScalarField::new(values)),
            ImshowOptions::default(),
        )?;
        let options = ColorbarOptions {
            label: r"$\rho$ (kg/m$^3$)".into(),
            ..ColorbarOptions::beside(&ax)
        };
        fig.colorbar(&image, options)?;
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/tight_layout", |fig| {
        let axs = fig.subplots(2, 2)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            ax.plot([0.0, 1.0], [0.0, i as f64])?;
            ax.set_title(format!("Panel ${i}$"));
            ax.set_xlabel(format!("$x_{{{i}}}$"));
            ax.set_ylabel(format!(r"$\int f_{{{i}}}$"));
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("usetex/constrained_layout", |fig| {
        let axs = fig.subplots(1, 2)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            ax.plot([0.0, 1.0], [1.0, i as f64])?;
            ax.set_title(format!(r"\LaTeX\ {i}"));
            ax.set_xlabel(r"$\xi$");
        }
        fig.supxlabel(r"shared $x$");
        Ok(())
    })
    .sized(4.0, 3.0, 100.0)
    .with_rc(&[
        ("text.usetex", "True"),
        ("figure.constrained_layout.use", "True"),
    ])
    .needing_tex(),
    usetex_case("usetex/path_effects", |fig| {
        let orange = Stroke::new(GcUpdate::default().linewidth(3.0).foreground("tab:orange"));
        fig.text(0.1, 0.4, r"Stroke $\Omega$")
            .set_fontsize(20.0)
            .set_path_effects(vec![PathEffect::with_stroke(orange)]);
        Ok(())
    })
    .sized(3.2, 1.6, 100.0),
    FigureCase::new("usetex/dvipng_engine", |fig| {
        fig.text(0.05, 0.7, r"dvipng: $\frac{a}{b} + \sqrt{2}$")
            .set_fontsize(12.0);
        fig.text(0.05, 0.3, r"rasterized by \texttt{dvipng}")
            .set_color("tab:blue");
        fig.text(0.75, 0.15, r"turned $\int_0^1$")
            .set_rotation(30.0);
        fig.text(0.95, 0.1, r"$x^2$ up").set_rotation(90.0);
        Ok(())
    })
    .sized(3.2, 2.0, 100.0)
    .with_rc(&[
        ("text.usetex", "True"),
        ("text.latex.engine", "latex+dvipng"),
    ])
    .needing_tex(),
];
