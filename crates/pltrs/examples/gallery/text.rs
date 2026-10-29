//! Text: annotations, mathtext, text turned into paths, and path
//! effects.

use std::f64::consts::{FRAC_PI_2, PI};

use pltrs::coords::Coords;
use pltrs::fonts::{FontProperties, FontWeight, Fontset};
use pltrs::patches::{BoxStyle, Circle, PathShape};
use pltrs::patheffects::{GcUpdate, LineShadow, PatchShadow, PathEffect, Stroke, TickedStroke};
use pltrs::prelude::*;
use pltrs::rc::{self, Aspect};
use pltrs::text::{AnnotateOptions, HAlign, TextBoxProps, VAlign, text_path};
use pltrs::transforms::{Affine2D, Point};

use super::lines::map;
use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "annotations",
        "Annotations with arrows and boxes",
        annotations,
    ),
    Entry::figure(
        "mathtext",
        "Mathtext: TeX-like formulas without TeX",
        mathtext,
    ),
    Entry::figure(
        "text_path",
        "Text as paths, laid along a curve (text_path)",
        text_path_demo,
    ),
    Entry::figure(
        "patheffects",
        "Path effects: strokes, shadows and ticks",
        patheffects,
    ),
];

/// A curve with its maximum and minimum pointed out.
fn annotations() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 5.0, 300);
    let y = map(&x, |x| (2.0 * PI * x / 2.5).cos() * (-x / 3.0).exp());
    ax.plot(x, y)?;
    ax.set_ylim(-1.2, 1.5)?;
    ax.annotate(
        "local max",
        Point::new(2.5, (-2.5_f64 / 3.0).exp()),
        AnnotateOptions {
            xytext: Some(Point::new(3.3, 1.2).into()),
            arrowprops: Some("->".parse()?),
            ..AnnotateOptions::default()
        },
    )?;
    let minimum = ax.annotate(
        "local min",
        Point::new(1.25, -(-1.25_f64 / 3.0).exp()),
        AnnotateOptions {
            xytext: Some(Point::new(0.15, 0.1).into()),
            textcoords: Some("axes fraction".parse::<Coords>()?),
            arrowprops: Some("fancy".parse()?),
            ..AnnotateOptions::default()
        },
    )?;
    // A tinted, translucent box reads on light and dark backgrounds alike.
    let tint = PatchProps::default()
        .facecolor("C3")
        .alpha(0.25)
        .edgecolor("C3");
    minimum.set_bbox(Some(
        TextBoxProps::new("round".parse::<BoxStyle>()?).with_patch(tint),
    ));
    ax.set_title("Annotating a damped cosine");
    Ok(fig)
}

/// Formulas typeset by pltrs' own mathtext engine: fractions, roots,
/// big operators, accents, Greek and the math alphabets, in the look's
/// fonts and in Computer Modern.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every gallery builder has the same signature"
)]
fn mathtext() -> Built {
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    let ax = fig.subplot();
    ax.set_axis_off();
    let formulas = [
        r"$\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}$",
        r"$\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}$",
        r"$\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}$",
        r"$\hat{f}(\xi) = \int f(x)\, e^{-2\pi i x \xi}\, dx$",
        r"$\alpha^2 + \beta_{ij} \leq \sqrt[3]{\gamma}\quad \vec{v} \cdot \dot{x}$",
        r"$\mathbb{R}\ \mathcal{L}\ \mathfrak{g}\ \mathsf{Sans}\ \mathtt{mono}$",
    ];
    for (i, formula) in formulas.iter().enumerate() {
        let y = 0.92 - 0.17 * i as f64;
        ax.text(0.0, y, *formula).set_fontsize(15.0);
        // The same formula in Computer Modern, the TeX look.
        let tex = ax.text(0.56, y, *formula);
        tex.set_fontsize(15.0).set_math_fontfamily(Fontset::Cm);
    }
    ax.set_title("mathtext: the look's fonts (left) and Computer Modern (right)");
    Ok(fig)
}

/// The outlines of glyphs as paths: a word filled and outlined like any
/// patch, and words laid glyph by glyph over and under it along a circle.
fn text_path_demo() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let font = FontProperties {
        weight: FontWeight::BOLD,
        ..FontProperties::from_rc(&rc::params())
    };
    let word = text_path(Point::new(-2.0, -0.45), "plt-rs", Some(1.3), &font)?;
    let paint = PatchProps::default()
        .facecolor("C0")
        .edgecolor("C1")
        .linewidth(1.5);
    ax.add_patch_with(PathShape(word), paint);
    along_circle(&ax, "TEXT ALONG A PATH", &font, 1.0)?;
    along_circle(&ax, "glyph by glyph", &font, -1.0)?;
    ax.set_xlim(-3.2, 3.2)?.set_ylim(-2.6, 2.6)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_axis_off();
    Ok(fig)
}

/// Lays `message` along a circle of radius 2 about the origin, centred on
/// the vertical: over the top, clockwise, for `side = 1`, and under the
/// bottom, anticlockwise, for `side = -1`. Each glyph's path is turned to
/// the tangent and moved to its place on the circle.
fn along_circle(
    ax: &Axes,
    message: &str,
    font: &FontProperties,
    side: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let (radius, size) = (2.0, 0.38);
    let mut glyphs = Vec::new();
    for letter in message.chars() {
        let glyph = text_path(Point::new(0.0, 0.0), &letter.to_string(), Some(size), font)?;
        let advance = if letter == ' ' {
            0.3 * size
        } else {
            glyph.extents().width() + 0.1 * size
        };
        glyphs.push((glyph, advance));
    }
    let mut s = -glyphs.iter().map(|(_, advance)| advance).sum::<f64>() / 2.0;
    for (glyph, advance) in glyphs {
        let angle = side * (FRAC_PI_2 - s / radius);
        let place = Affine2D::identity()
            .rotate(angle - side * FRAC_PI_2)
            .translate(radius * angle.cos(), radius * angle.sin());
        ax.add_patch_with(
            PathShape(glyph.transformed(&place)),
            PatchProps::default().facecolor("0.5").linewidth(0.0),
        );
        s += advance;
    }
    Ok(())
}

/// Effects drawn around the paths of artists: a halo that keeps text
/// readable over a busy background, drop shadows under a line and a
/// patch, and a ticked stroke marking the forbidden side of a constraint.
fn patheffects() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 200);
    for k in 0..8 {
        let wave = map(&x, |x| (x + f64::from(k) * 0.4).sin() * 2.0);
        let faint = LineProps::default().color("0.5").alpha(0.35).linewidth(3.0);
        ax.plot_with(&x, wave, faint)?;
    }
    let halo = || {
        PathEffect::with_stroke(Stroke::new(
            GcUpdate::default().linewidth(5.0).foreground(background()),
        ))
    };
    let label = ax.text(5.0, 0.0, "a halo keeps text readable");
    label
        .set_fontsize(18.0)
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center);
    label.set_path_effects(vec![halo()]);
    let shadow = LineShadow {
        offset: (3.0, -3.0),
        ..LineShadow::default()
    };
    let shadowed = ax.plot_with(
        &x,
        map(&x, |x| 2.6 - 0.1 * x),
        LineProps::default().linewidth(3.0),
    )?;
    shadowed.set_path_effects(vec![
        PathEffect::simple_line_shadow(shadow),
        PathEffect::normal(),
    ]);
    let disc = ax.add_patch_with(
        Circle::new((9.0, 0.0), 0.5),
        PatchProps::default().facecolor("C2").zorder(3.0),
    );
    let drop = PatchShadow {
        offset: (4.0, -4.0),
        ..PatchShadow::default()
    };
    disc.set_path_effects(vec![PathEffect::with_simple_patch_shadow(drop)]);
    let fenced = ax.plot_with(
        &x,
        map(&x, |x| -2.6 + 0.1 * x),
        LineProps::default().color("C1"),
    )?;
    let ticks = TickedStroke {
        angle: -135.0,
        length: 0.8,
        ..TickedStroke::default()
    };
    fenced.set_path_effects(vec![PathEffect::ticked_stroke(ticks)]);
    ax.text(0.3, -3.3, "a ticked stroke marks the side out of bounds")
        .set_path_effects(vec![halo()]);
    ax.set_ylim(-3.6, 3.2)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("Path effects");
    Ok(fig)
}
