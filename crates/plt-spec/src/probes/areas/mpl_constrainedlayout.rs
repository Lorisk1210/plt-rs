//! Probe operations for the `mpl_constrainedlayout` area: constrained
//! layout, ported from matplotlib's `tests/test_constrainedlayout.py`
//! (see `spec/reference/probes/test_mpl_constrainedlayout.py`, which
//! documents every op).

use plt::artist::ArtistHandle;
use plt::axes::{Axes, AxisMode};
use plt::figure::{Figure, Mosaic, MosaicOptions};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::pyplot;
use plt_core::Bbox;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Figure.suptitle", suptitle),
    Op::new("Text.get_position", compressed_suptitle),
    Op::new("Figure.supxlabel", suplabels),
    Op::new("Figure.align_ylabels", align_labels),
    Op::new("ConstrainedLayoutEngine.set", submerged_gap),
    Op::new("Figure.get_constrained_layout", get_constrained_layout),
    Op::new("Figure.set_constrained_layout", set_constrained_layout),
    Op::new("pyplot.figure", reused_figure),
];

/// `[x0, y0, x1, y1]` of a box.
fn extents(bbox: &Bbox) -> Json {
    numbers([bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()])
}

/// A figure of the rc size laid out by the constrained engine
/// (`plt.figure(layout="constrained")`).
fn constrained_figure() -> Result<Figure, ProbeError> {
    let fig = Figure::new();
    fig.set_layout_engine(LayoutEngine::constrained())
        .map_err(ProbeError::rejected)?;
    Ok(fig)
}

/// A figure of `width` × `height` inches at the rc resolution.
fn sized_figure(width: f64, height: f64) -> Figure {
    Figure::with_size(width, height, plt::rc::params().figure_dpi)
}

/// The engine `constrained_layout=value` asks for: `true` the constrained
/// engine, `false` none, and an object the constrained engine with the
/// parameters it sets (`null` keeps a default, as `set` does).
fn constrained_engine(value: &Json) -> Result<Option<LayoutEngine>, ProbeError> {
    let entries = match value {
        Json::Bool(on) => return Ok(on.then(LayoutEngine::constrained)),
        Json::Object(entries) => entries,
        _ => return Err(ProbeError::BadArgs(format!("bad layout {value}"))),
    };
    let mut params = ConstrainedLayout::from_rc(&plt::rc::params());
    for (key, value) in entries.iter().filter(|(_, value)| !value.is_null()) {
        let number = || {
            value
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad {key}")))
        };
        match key.as_str() {
            "w_pad" => params.w_pad = number()?,
            "h_pad" => params.h_pad = number()?,
            "wspace" => params.wspace = number()?,
            "hspace" => params.hspace = number()?,
            "rect" => {
                let rect = value.as_f64_vec().unwrap_or_default();
                params.rect = rect
                    .try_into()
                    .map_err(|_| ProbeError::BadArgs("bad rect".into()))?;
            }
            _ => return Err(ProbeError::BadArgs(format!("unknown parameter {key}"))),
        }
    }
    Ok(Some(LayoutEngine::Constrained(params)))
}

/// The axes' extents after a draw, then after each suptitle call
/// `[text, y]` and a draw.
fn suptitle(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = constrained_figure()?;
        let ax = fig.subplot();
        fig.layout();
        let mut out = vec![extents(&ax.position())];
        for call in args.json("calls")?.as_array().unwrap_or(&[]) {
            let [Json::String(text), y] = call.as_array().unwrap_or(&[]) else {
                return Err(ProbeError::BadArgs(format!("bad call {call}")));
            };
            fig.suptitle(text.as_str());
            if let Some(y) = y.as_f64() {
                fig.set_suptitle_y(y);
            }
            fig.layout();
            out.push(extents(&ax.position()));
        }
        Ok(Json::Array(out))
    })
}

/// The height of `test_compressed_suptitle`'s title after each call
/// `suptitle("Title", **kwargs)` and a draw.
fn compressed_suptitle(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = sized_figure(4.0, 10.0);
        fig.set_layout_engine(LayoutEngine::compressed())
            .map_err(ProbeError::rejected)?;
        let rows = SubplotOverrides {
            hspace: Some(0.0),
            ..SubplotOverrides::default()
        };
        let gs = GridSpec::with_ratios(2, 1, None, Some(vec![1.0 / 4.0, 3.0 / 4.0]), rows)
            .map_err(ProbeError::rejected)?;
        for (row, box_aspect) in [(0, 1.0 / 3.0), (1, 1.0)] {
            let ax = add_cell(&fig, &gs, row, 0)?;
            ax.axis_mode(AxisMode::Equal)
                .map_err(ProbeError::rejected)?;
            ax.set_box_aspect(Some(box_aspect))
                .map_err(ProbeError::rejected)?;
        }
        let mut out = Vec::new();
        for call in args.json("calls")?.as_array().unwrap_or(&[]) {
            let title = fig.suptitle("Title");
            if let Some(y) = call.get("y").and_then(Json::as_f64) {
                fig.set_suptitle_y(y);
            }
            if let Some(in_layout) = call.get("in_layout").and_then(Json::as_bool) {
                title.set_in_layout(in_layout);
            }
            fig.layout();
            out.push(title.position().y);
        }
        Ok(numbers(out))
    })
}

/// The axes in cell `(row, col)` of `gs`.
fn add_cell(fig: &Figure, gs: &GridSpec, row: usize, col: usize) -> Result<Axes, ProbeError> {
    let cell = gs.cell(row, col).map_err(ProbeError::rejected)?;
    fig.add_subplot_spec(&cell).map_err(ProbeError::rejected)
}

/// The axes' tight box after a draw, then after the figure-wide labels
/// and a draw.
fn suplabels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = constrained_figure()?;
        let ax = fig.subplot();
        let tight = || {
            ax.tight_bbox(false)
                .ok_or_else(|| ProbeError::rejected("no tight box"))
        };
        fig.layout();
        let before = tight()?;
        let xlabel = fig.supxlabel("Boo");
        if let Some(x) = args.opt_f64("x")? {
            xlabel.set_x(x);
        }
        let ylabel = fig.supylabel(args.str("ylabel")?);
        if let Some(y) = args.opt_f64("y")? {
            ylabel.set_y(y);
        }
        fig.layout();
        Ok(Json::Array(vec![extents(&before), extents(&tight()?)]))
    })
}

/// The left edges of `test_align_labels`'s y labels after aligning them
/// and a draw.
fn align_labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = sized_figure(6.4, 8.0);
        fig.set_layout_engine(LayoutEngine::constrained())
            .map_err(ProbeError::rejected)?;
        let gs = GridSpec::with_ratios(
            3,
            1,
            None,
            Some(vec![1.0, 1.0, 0.7]),
            SubplotOverrides::default(),
        )
        .map_err(ProbeError::rejected)?;
        let [ax3, ax1, ax2] = [0, 1, 2].map(|row| add_cell(&fig, &gs, row, 0));
        let (ax1, ax2, ax3) = (ax1?, ax2?, ax3?);
        for (ax, (low, high)) in [(&ax1, (0.0, 1.0)), (&ax2, (-1.5, 1.5)), (&ax3, (0.0, 1.0))] {
            ax.set_ylim(low, high)
                .map_err(ProbeError::rejected)?
                .set_ylabel("Label");
        }
        fig.align_ylabels(Some(&[ax3.clone(), ax1.clone(), ax2.clone()]));
        fig.layout();
        let left = |ax: &Axes| ax.yaxis().label_extent().map_or(f64::NAN, |b| b.x0());
        Ok(numbers([left(&ax1), left(&ax2), left(&ax3)]))
    })
}

/// The bounds of the axes `"A"` and `"B"` of a constrained mosaic whose
/// padding changed.
fn submerged_gap(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = constrained_figure()?;
        let mosaic = args
            .str("mosaic")?
            .parse::<Mosaic>()
            .map_err(ProbeError::rejected)?;
        let axes = fig
            .subplot_mosaic(&mosaic, MosaicOptions::default())
            .map_err(ProbeError::rejected)?;
        fig.set_layout_engine(constrained_engine(args.json("pads")?)?)
            .map_err(ProbeError::rejected)?;
        fig.layout();
        let bounds = |label: &str| {
            let ax = axes
                .get(label)
                .ok_or_else(|| ProbeError::BadArgs(format!("no axes {label}")))?;
            let (x, y, width, height) = ax.position().bounds();
            Ok::<_, ProbeError>(numbers([x, y, width, height]))
        };
        Ok(object([("A", bounds("A")?), ("B", bounds("B")?)]))
    })
}

/// Whether the figure is laid out by the constrained engine.
fn uses_constrained(fig: &Figure) -> Json {
    Json::from(matches!(
        fig.layout_engine(),
        Some(LayoutEngine::Constrained(_))
    ))
}

/// Whether `plt.subplots(constrained_layout=...)` is laid out by the
/// constrained engine.
fn get_constrained_layout(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.set_layout_engine(constrained_engine(args.json("constrained_layout")?)?)
            .map_err(ProbeError::rejected)?;
        fig.subplot();
        Ok(uses_constrained(&fig))
    })
}

/// The same after each `set_constrained_layout(value)`.
fn set_constrained_layout(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot();
        let mut out = Vec::new();
        for value in args.json("calls")?.as_array().unwrap_or(&[]) {
            fig.set_layout_engine(constrained_engine(value)?)
                .map_err(ProbeError::rejected)?;
            out.push(uses_constrained(&fig));
        }
        Ok(Json::Array(out))
    })
}

/// `test_constrained_layout23`: the constrained figure `"123"` made
/// twice, cleared the second time, with a suptitle each time.
fn reused_figure(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mut fig = None;
        for i in 0..2 {
            let reused = pyplot::figure_label("123");
            reused.clear();
            reused
                .set_layout_engine(LayoutEngine::constrained())
                .map_err(ProbeError::rejected)?;
            // The test's grid nested in a grid holds no axes, so it takes
            // no part in the layout.
            reused.suptitle(format!("Suptitle{i}"));
            fig = Some(reused);
        }
        let fig = fig.ok_or_else(|| ProbeError::rejected("no figure"))?;
        fig.layout();
        let texts = u32::try_from(fig.texts().len()).map_err(ProbeError::rejected)?;
        Ok(object([
            (
                "fignums",
                numbers(pyplot::get_fignums().into_iter().map(f64::from)),
            ),
            (
                "figlabels",
                Json::Array(
                    pyplot::get_figlabels()
                        .into_iter()
                        .map(Json::from)
                        .collect(),
                ),
            ),
            ("suptitle", Json::from(fig.suptitle_text())),
            ("texts", Json::from(f64::from(texts))),
        ]))
    })
}
