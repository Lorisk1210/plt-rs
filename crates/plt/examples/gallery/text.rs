//! Text: annotations with arrows and boxes, and mathtext.

use plt::coords::Coords;
use plt::patches::{ArrowStyle, BoxStyle};
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, TextBoxProps};
use plt::transforms::Point;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "annotations",
        caption: "Annotations with arrows and boxes",
        build: annotations,
    },
    Entry {
        name: "mathtext",
        caption: "Mathtext: TeX-like formulas without TeX",
        build: mathtext,
    },
];

/// A curve with its maximum and minimum pointed out.
fn annotations() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 5.0, 300);
    let y: Vec<f64> = x
        .iter()
        .map(|x| (2.0 * std::f64::consts::PI * x / 2.5).cos() * (-x / 3.0).exp())
        .collect();
    ax.plot(x, y)?;
    ax.set_ylim(-1.2, 1.5)?;
    let arrow = |style: &str| -> Result<Option<ArrowProps>, Box<dyn std::error::Error>> {
        Ok(Some(ArrowProps::Fancy(FancyArrowProps::new(
            style.parse::<ArrowStyle>()?,
        ))))
    };
    ax.annotate(
        "local max",
        Point::new(2.5, (-2.5_f64 / 3.0).exp()),
        AnnotateOptions {
            xytext: Some(Point::new(3.3, 1.2).into()),
            arrowprops: arrow("->")?,
            ..AnnotateOptions::default()
        },
    )?;
    let minimum = ax.annotate(
        "local min",
        Point::new(1.25, -(-1.25_f64 / 3.0).exp()),
        AnnotateOptions {
            xytext: Some(Point::new(0.15, 0.1).into()),
            textcoords: Some("axes fraction".parse::<Coords>()?),
            arrowprops: arrow("fancy")?,
            ..AnnotateOptions::default()
        },
    )?;
    minimum.set_bbox(Some(
        TextBoxProps::new("round".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().facecolor("lightyellow")),
    ));
    ax.set_title("Annotating a damped cosine");
    Ok(fig)
}

/// Formulas typeset by plt-rs' own mathtext engine.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every gallery builder has the same signature"
)]
fn mathtext() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    ax.set_axis_off();
    let formulas = [
        r"$\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}$",
        r"$\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}$",
        r"$\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}$",
        r"$\mathcal{F}(\omega) = \int f(t)\, e^{-i \omega t}\, dt$",
        r"$\alpha^2 + \beta_{ij} \leq \sqrt[3]{\gamma}$",
    ];
    for (i, formula) in formulas.iter().enumerate() {
        let y = 0.9 - 0.2 * i as f64;
        let text = ax.text(0.05, y, *formula);
        text.set_fontsize(16.0);
    }
    ax.set_title("mathtext");
    Ok(fig)
}
