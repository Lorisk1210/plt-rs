//! Angle annotations: an arc between two vectors that is circular on
//! screen, labelled inside, outside or at its edge (`angle_annotation`).
//!
//! The example's `AngleAnnotation` subclasses `Arc` to recompute its
//! centre and angles in pixels at every draw. Here the arc and its label
//! are placed around the display position of the centre (a translation
//! by the data point), so they follow the resolution of every canvas like
//! matplotlib's; the angles, which the resolution leaves alone, are
//! computed once the figure is laid out, and so is the offset of a label
//! outside its arc, from the label's extent on the canvas the case is
//! built for (matplotlib measures it again at the 72 dpi of vector
//! output, where it comes out a fraction of a point apart).
//!
//! As in matplotlib 3.11, whose `Ellipse` draws its stored `_width` rather
//! than the overridden `width` property, the arc's diameter is the size in
//! pixels whatever its unit, while the label is placed with the size in
//! its unit.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use plt::coords::Coords;
use plt::figure::Share;
use plt::numeric::interp;
use plt::patches::Arc;
use plt::prelude::*;
use plt::text::{AnnotateOptions, Annotation, ArrowProps, HAlign, VAlign};
use plt::transforms::{Point, Transform};

use crate::figures::cases::common::{arrow, styled_box};
use crate::figures::{CaseResult, FigureCase};

/// What the size of an angle annotation is measured in.
#[derive(Clone, Copy)]
enum SizeUnit {
    Pixels,
    Points,
    /// The smaller side of the axes.
    AxesMin,
    /// The larger side of the axes.
    AxesMax,
}

/// Where the label of an angle annotation goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextPosition {
    /// Inside the arc, closer for wider angles.
    Inside,
    /// Just outside the arc, clear of it.
    Outside,
    /// On the arc, offset as the annotation says.
    Edge,
}

/// An angle annotation: the arc at `center` from the vector towards `p1`
/// to the one towards `p2`, all in data coordinates, `size` across.
struct AngleAnnotation<'a> {
    center: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    size: f64,
    unit: SizeUnit,
    text: &'a str,
    position: TextPosition,
}

impl<'a> AngleAnnotation<'a> {
    /// The example's defaults: 75 points across, labelled inside.
    fn new(center: (f64, f64), p1: (f64, f64), p2: (f64, f64), text: &'a str) -> Self {
        Self {
            center,
            p1,
            p2,
            size: 75.0,
            unit: SizeUnit::Points,
            text,
            position: TextPosition::Inside,
        }
    }

    /// The same annotation `size` across in `unit`.
    fn sized(self, size: f64, unit: SizeUnit) -> Self {
        Self { size, unit, ..self }
    }

    /// The same annotation labelled at `position`.
    fn placed(self, position: TextPosition) -> Self {
        Self { position, ..self }
    }

    /// Draws the arc with `arc` on `ax` of the laid-out `fig` and its
    /// label with `label` (offset from its point and with an arrow, if
    /// any), styled by `style` before it is measured.
    fn draw(
        &self,
        fig: &Figure,
        ax: &Axes,
        arc: PatchProps,
        label: AnnotateOptions,
        style: impl FnOnce(&Annotation) -> CaseResult,
    ) -> CaseResult {
        let to_pixels = fig.resolve_transform(&ax.trans_data())?;
        let center = to_pixels.transform_point(self.center.into());
        let theta = |p: (f64, f64)| {
            let v = to_pixels.transform_point(p.into()) - center;
            v.y.atan2(v.x).to_degrees()
        };
        let (theta1, theta2) = (theta(self.p1), theta(self.p2));
        let at_center = Transform::Translation {
            dx: self.center.0,
            dy: self.center.1,
            units: Box::new(ax.trans_data()),
        };
        let mut props = arc;
        props.base.transform = Some(at_center.clone());
        ax.add_patch_with(
            Arc::new((0.0, 0.0), self.size, self.size).between(theta1, theta2),
            props,
        );
        let s = self.size * self.factor(fig, ax)?;
        let span = (theta2 - theta1).rem_euclid(360.0);
        let angle = (theta1 + span / 2.0).to_radians();
        let r = if self.position == TextPosition::Inside {
            s / interp(
                span,
                &[60.0, 90.0, 135.0, 180.0],
                &[3.3, 3.5, 3.8, 4.0],
                None,
            )
        } else {
            s / 2.0
        };
        let (sin, cos) = angle.sin_cos();
        // Pixels stay pixels at any resolution; points and axes fractions
        // are lengths, scaled from inches.
        let (radius, around_center) = match self.unit {
            SizeUnit::Pixels => (r, at_center),
            _ => (r / fig.dpi(), Transform::Inches.then(at_center)),
        };
        let note = ax.annotate(
            self.text,
            (radius * cos, radius * sin),
            AnnotateOptions {
                xycoords: Coords::Transform(around_center),
                xytext: label.xytext.or(Some(Point::ORIGIN)),
                textcoords: Some("offset points".parse()?),
                annotation_clip: Some(true),
                ..label
            },
        )?;
        note.set_halign(HAlign::Center).set_valign(VAlign::Center);
        style(&note)?;
        if self.position == TextPosition::Outside {
            let extent = note.window_extent().ok_or("a label without extent")?;
            let x = outside_distance(angle, r, extent.width(), extent.height());
            let offset = (x - s / 2.0) / fig.dpi() * 72.0;
            note.set_x(offset * cos).set_y(offset * sin);
        }
        Ok(())
    }

    /// The pixels in a unit of the size (the example's `get_size`).
    fn factor(&self, fig: &Figure, ax: &Axes) -> Result<f64, Box<dyn std::error::Error>> {
        let axes_box = || -> Result<(f64, f64), Box<dyn std::error::Error>> {
            let to_pixels = fig.resolve_transform(&ax.trans_axes())?;
            let size = to_pixels.transform_point(Point::new(1.0, 1.0))
                - to_pixels.transform_point(Point::ORIGIN);
            Ok((size.x, size.y))
        };
        Ok(match self.unit {
            SizeUnit::Pixels => 1.0,
            SizeUnit::Points => fig.dpi() / 72.0,
            SizeUnit::AxesMin => {
                let (width, height) = axes_box()?;
                width.min(height)
            }
            SizeUnit::AxesMax => {
                let (width, height) = axes_box()?;
                width.max(height)
            }
        })
    }
}

/// How far from the centre a `w × h` label at `angle` (radians) must be
/// to clear the arc of radius `r` (the example's `R`, folding the angle
/// into the first octant, and `R90`).
fn outside_distance(angle: f64, r: f64, w: f64, h: f64) -> f64 {
    let (folded, quadrant) = (angle.rem_euclid(FRAC_PI_4), angle.rem_euclid(FRAC_PI_2));
    let rising = if quadrant <= FRAC_PI_4 { folded } else { 0.0 };
    let falling = if quadrant >= FRAC_PI_4 {
        FRAC_PI_4 - folded
    } else {
        0.0
    };
    let a = rising + falling;
    let (w, h) = if (2.0 * angle).cos() < 0.0 {
        (h, w)
    } else {
        (w, h)
    };
    if a < (h / 2.0 / (r + w / 2.0)).atan() {
        ((r + w / 2.0).powi(2) + (a.tan() * (r + w / 2.0)).powi(2)).sqrt()
    } else {
        let c = ((w / 2.0).powi(2) + (h / 2.0).powi(2)).sqrt();
        let t = (c * (FRAC_PI_2 - a + (h / 2.0 / c).asin()).cos() / r).asin();
        let (sin, cos) = (a + t).sin_cos();
        let (x, y) = (r * cos + w / 2.0, r * sin + h / 2.0);
        (x.powi(2) + y.powi(2)).sqrt()
    }
}

/// An arc with the default style.
fn plain_arc() -> PatchProps {
    PatchProps::default()
}

/// The label of the example without keywords of its own.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a label style, fallible like the others"
)]
fn no_style(_: &Annotation) -> CaseResult {
    Ok(())
}

fn angle_annotation_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("AngleLabel example");
    let center = (4.5, 650.0);
    let p1 = [(2.5, 710.0), (6.0, 605.0)];
    let p2 = [(3.0, 275.0), (5.5, 900.0)];
    for [a, b] in [p1, p2] {
        ax.plot([a.0, b.0], [a.1, b.1])?;
    }
    ax.plot_with(
        [center.0],
        [center.1],
        LineProps::default().marker(Marker::from(MarkerSymbol::Circle)),
    )?;
    let p = [(6.0, 400.0), (5.3, 410.0), (5.6, 300.0)];
    ax.plot(p.map(|q| q.0), p.map(|q| q.1))?;
    fig.layout();
    let angles = [
        (p1[1], p2[1], 75.0, r"$\alpha$"),
        (p2[1], p1[0], 35.0, r"$\beta$"),
        (p1[0], p2[0], 75.0, r"$\gamma$"),
        (p2[0], p1[1], 35.0, r"$\theta$"),
    ];
    for (from, to, size, text) in angles {
        AngleAnnotation::new(center, from, to, text)
            .sized(size, SizeUnit::Points)
            .draw(fig, &ax, plain_arc(), AnnotateOptions::default(), no_style)?;
    }
    AngleAnnotation::new(p[1], p[0], p[2], r"$\Phi$")
        .sized(40.0, SizeUnit::Points)
        .placed(TextPosition::Outside)
        .draw(
            fig,
            &ax,
            PatchProps::default()
                .linestyle(LineStyle::Dashed)
                .color("gray"),
            AnnotateOptions::default(),
            |note| {
                note.set_fontsize(16.0).set_color("gray");
                Ok(())
            },
        )?;
    Ok(())
}

/// The example's `plot_angle`: the two arms of `angle` degrees at `pos`
/// plotted, and the angle annotated between them.
fn plot_angle(
    ax: &Axes,
    pos: (f64, f64),
    angle: f64,
    text: &'static str,
) -> Result<AngleAnnotation<'static>, Box<dyn std::error::Error>> {
    const LENGTH: f64 = 0.95;
    let (sin, cos) = angle.to_radians().sin_cos();
    let arms = [
        (LENGTH + pos.0, pos.1),
        pos,
        (cos * LENGTH + pos.0, sin * LENGTH + pos.1),
    ];
    ax.plot_with(
        arms.map(|p| p.0),
        arms.map(|p| p.1),
        LineProps::default().color("C0"),
    )?;
    Ok(AngleAnnotation::new(pos, arms[0], arms[2], text))
}

/// The example's `label_angles`: the name of each annotation of a row in
/// a small box on the bottom spine.
fn label_angles(ax: &Axes, texts: [&str; 4]) -> CaseResult {
    for (x, text) in [2.0, 3.5, 5.0, 6.5].into_iter().zip(texts) {
        ax.annotate(
            text,
            (x, 0.0),
            AnnotateOptions {
                xycoords: Coords::Transform(ax.trans_xaxis()),
                annotation_clip: Some(true),
                ..AnnotateOptions::default()
            },
        )?
        .set_bbox(styled_box("round", PatchProps::default().facecolor("w"))?)
        .set_halign(HAlign::Left)
        .set_fontsize(8.0);
    }
    Ok(())
}

fn angle_annotation_2(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    fig.suptitle("AngleLabel keyword arguments");
    ax1.set_margins(None, 0.4)?;
    ax1.set_title("textposition");
    ax2.set_margins(None, 0.4)?;
    ax2.set_title("unit");
    let text = r"$60°$";
    let rows = [ax1, ax1, ax1, ax1, ax2, ax2, ax2, ax2];
    let mut angles = Vec::new();
    for (ax, x) in rows.into_iter().zip([2.0, 3.5, 5.0, 6.5].repeat(2)) {
        angles.push(plot_angle(ax, (x, 0.0), 60.0, text)?);
    }
    fig.layout();
    let mut angles = angles.into_iter();
    let mut next = || angles.next().ok_or("an angle per arm");
    next()?.draw(fig, ax1, plain_arc(), AnnotateOptions::default(), no_style)?;
    next()?.placed(TextPosition::Outside).draw(
        fig,
        ax1,
        plain_arc(),
        AnnotateOptions::default(),
        no_style,
    )?;
    let boxed = |note: &Annotation| {
        note.set_bbox(styled_box("round", PatchProps::default().facecolor("w"))?);
        Ok(())
    };
    next()?.placed(TextPosition::Edge).draw(
        fig,
        ax1,
        plain_arc(),
        AnnotateOptions::default(),
        boxed,
    )?;
    let custom_arrow = AnnotateOptions {
        xytext: Some(Point::new(30.0, 20.0)),
        arrowprops: Some(ArrowProps::Fancy(arrow("->", "arc3,rad=-0.2")?)),
        ..AnnotateOptions::default()
    };
    next()?
        .placed(TextPosition::Edge)
        .draw(fig, ax1, plain_arc(), custom_arrow, no_style)?;
    label_angles(
        ax1,
        [
            "\"inside\"",
            "\"outside\"",
            "\"edge\"",
            "\"edge\", custom arrow",
        ],
    )?;
    let units = [
        (50.0, SizeUnit::Pixels),
        (50.0, SizeUnit::Points),
        (0.25, SizeUnit::AxesMin),
        (0.25, SizeUnit::AxesMax),
    ];
    for (size, unit) in units {
        next()?
            .sized(size, unit)
            .placed(TextPosition::Outside)
            .draw(fig, ax2, plain_arc(), AnnotateOptions::default(), no_style)?;
    }
    label_angles(
        ax2,
        ["\"pixels\"", "\"points\"", "\"axes min\"", "\"axes max\""],
    )?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/angle_annotation_1", angle_annotation_1),
    FigureCase::new("gallery_text/angle_annotation_2", angle_annotation_2),
];
