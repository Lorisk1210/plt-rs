//! Layout: layout engines and their parameters, tight boxes, aligned
//! labels and rotated date labels.

use plt::artist::ArtistHandle;
use plt::axes::{Mesh, PcolorOptions, TickLevel};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::figure::{BboxInches, Figure, SaveOptions};
use plt::layout::{ConstrainedLayout, LayoutEngine, TightLayout};
use plt::numeric::Grid;
use plt::text::HAlign;
use plt::ticker::{AutoMinorLocator, FixedFormatter, MinorDivisions};

use super::super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// The name of a figure's layout engine.
fn engine_name(fig: &Figure) -> Json {
    Json::from(match fig.layout_engine() {
        None | Some(LayoutEngine::PlaceHolder(_)) => "none",
        Some(LayoutEngine::Tight(_)) => "tight",
        Some(LayoutEngine::Constrained(engine)) if engine.compress => "compressed",
        Some(LayoutEngine::Constrained(_)) => "constrained",
    })
}

/// Sets the layout engine named `name` (`"none"` removes the engine).
fn set_engine(fig: &Figure, name: &str) -> Result<(), ProbeError> {
    let engine = match name {
        "none" => None,
        name => Some(name.parse::<LayoutEngine>().map_err(ProbeError::rejected)?),
    };
    fig.set_layout_engine(engine)
        .map_err(ProbeError::rejected)?;
    Ok(())
}

/// The engine of `Figure(layout=layout)` after each call.
pub(super) fn layout_engine(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        if let Some(name) = args.opt_str("layout")? {
            set_engine(&fig, name)?;
        }
        let mut out = vec![engine_name(&fig)];
        for call in args.get("calls").and_then(Json::as_array).unwrap_or(&[]) {
            match call.as_array().unwrap_or(&[]) {
                [Json::String(name), Json::String(layout)] if name == "set_layout_engine" => {
                    set_engine(&fig, layout)?;
                }
                [Json::String(name)] if name == "colorbar" => {
                    let values = Grid::from_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0])
                        .map_err(ProbeError::rejected)?;
                    let mesh = fig
                        .subplot()
                        .pcolormesh(Mesh::new(values), PcolorOptions::default())
                        .map_err(ProbeError::rejected)?;
                    fig.colorbar(&mesh, ColorbarOptions::default())
                        .map_err(ProbeError::rejected)?;
                }
                [Json::String(name), kwargs] if name == "tight_layout" => {
                    fig.subplots(1, 2).map_err(ProbeError::rejected)?;
                    let params = TightLayout {
                        w_pad: kwargs.get("w_pad").and_then(Json::as_f64),
                        ..TightLayout::default()
                    };
                    fig.tight_layout(&params).map_err(ProbeError::rejected)?;
                }
                [Json::String(name)] if name == "savefig_tight" => {
                    let options = SaveOptions {
                        bbox_inches: Some(BboxInches::Tight),
                        ..SaveOptions::default()
                    };
                    fig.to_png_with(options).map_err(ProbeError::rejected)?;
                }
                _ => return Err(ProbeError::BadArgs(format!("unknown call {call}"))),
            }
            out.push(engine_name(&fig));
        }
        Ok(Json::Array(out))
    })
}

/// The pads of a figure laid out by a constrained engine with `params`.
pub(super) fn constrained_pads(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let params = args.json("params")?;
        let param = |key: &str| {
            params
                .get(key)
                .and_then(Json::as_f64)
                .ok_or_else(|| ProbeError::BadArgs(format!("missing {key}")))
        };
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::Constrained(ConstrainedLayout {
            w_pad: param("w_pad")?,
            h_pad: param("h_pad")?,
            wspace: param("wspace")?,
            hspace: param("hspace")?,
            ..ConstrainedLayout::default()
        }))
        .map_err(ProbeError::rejected)?;
        let Some(LayoutEngine::Constrained(engine)) = fig.layout_engine() else {
            return Err(ProbeError::rejected("the engine was not kept"));
        };
        Ok(numbers([
            engine.w_pad,
            engine.h_pad,
            engine.wspace,
            engine.hspace,
        ]))
    })
}

/// The tight boxes of `test_tightbbox`.
pub(super) fn tightbbox(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 1.0).map_err(ProbeError::rejected)?;
        let text = ax.text(1.0, 0.5, "This dangles over end");
        let axes_x1 = || ax.tight_bbox(false).map_or(f64::NAN, |b| b.x1());
        let text_x1 = text.window_extent().map_or(f64::NAN, |b| b.x1());
        let (axes, figure) = (axes_x1(), fig.tight_bbox());
        text.set_in_layout(false);
        let (axes_out, figure_out) = (axes_x1(), fig.tight_bbox());
        text.set_in_layout(true);
        Ok(object([
            ("text_x1", Json::from(text_x1)),
            ("axes_x1", Json::from(axes)),
            ("fig_x1", Json::from(figure.x1())),
            ("fig_x0", Json::from(figure.x0())),
            ("axes_x1_out", Json::from(axes_out)),
            ("fig_x1_out", Json::from(figure_out.x1())),
            ("axes_x1_back", Json::from(axes_x1())),
        ]))
    })
}

/// The width of the tight box of a constrained figure of two empty
/// subfigures.
pub(super) fn subfigure_tightbbox(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::constrained())
            .map_err(ProbeError::rejected)?;
        super::texts::subfigures(&fig, 1, 2)?;
        Ok(Json::from(fig.tight_bbox().width()))
    })
}

/// The label extents of `test_align_labels_stray_axes`.
pub(super) fn align_labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let constrained = args.opt_str("layout")? == Some("constrained");
        if constrained {
            fig.set_layout_engine(LayoutEngine::constrained())
                .map_err(ProbeError::rejected)?;
        }
        let axs = fig.subplots(2, 2).map_err(ProbeError::rejected)?;
        let seed = args.opt_i64("seed")?.unwrap_or_default();
        let mut mesh = None;
        for (nn, ax) in axs.as_slice().iter().enumerate() {
            ax.set_xlabel("Boo");
            ax.set_ylabel("Who");
            if constrained {
                let seed = u64::try_from(seed).map_err(ProbeError::rejected)? + nn as u64;
                let values = SpecRng::new(seed).normal(100, 0.0, 1.0);
                let grid = Grid::from_vec(10, 10, values).map_err(ProbeError::rejected)?;
                let pc = ax
                    .pcolormesh(Mesh::new(grid), PcolorOptions::default())
                    .map_err(ProbeError::rejected)?;
                mesh = Some((pc, ax.clone()));
            } else {
                let power = |v: f64| v.powi(i32::try_from(nn).unwrap_or(0));
                let x: Vec<f64> = (0..4).map(|v| power(f64::from(v))).collect();
                ax.plot(x.clone(), x).map_err(ProbeError::rejected)?;
            }
        }
        if let Some((pc, ax)) = mesh {
            let options = ColorbarOptions {
                axes: ColorbarAxes::Parent(ax),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&pc, options).map_err(ProbeError::rejected)?;
        }
        fig.align_ylabels(None);
        fig.align_xlabels(None);
        fig.layout();
        let edge = |extent: Option<plt_core::Bbox>, x: bool| {
            extent.map_or(f64::NAN, |b| if x { b.x0() } else { b.y0() })
        };
        let axes = axs.as_slice();
        Ok(object([
            (
                "ylabel_x0",
                numbers(axes.iter().map(|ax| edge(ax.yaxis().label_extent(), true))),
            ),
            (
                "xlabel_y0",
                numbers(axes.iter().map(|ax| edge(ax.xaxis().label_extent(), false))),
            ),
        ]))
    })
}

/// The dates of `test_autofmt_xdate` (`datestr2num`): 3 to 14 January
/// 2013.
const FIRST_DAY: f64 = 15708.0;

/// The times of day of `test_autofmt_xdate`, in minutes after 16:44.
const MINUTES: [f64; 12] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 8.0, 9.0, 11.0, 12.0, 13.0,
];

/// The rotations of the x tick labels after `autofmt_xdate`.
pub(super) fn autofmt_xdate(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        if args.opt_bool("colorbar")? == Some(true) {
            return autofmt_with_colorbar();
        }
        let which = match args.str("which")? {
            "major" => TickLevel::Major,
            "minor" => TickLevel::Minor,
            "both" => TickLevel::Both,
            other => return Err(ProbeError::BadArgs(format!("bad which {other}"))),
        };
        let fig = Figure::new();
        let ax = fig.subplot();
        let x: Vec<f64> = (0..12).map(|day| FIRST_DAY + f64::from(day)).collect();
        let y: Vec<f64> = MINUTES
            .iter()
            .map(|minutes| FIRST_DAY + (16.0 * 60.0 + 44.0 + minutes) / 1440.0)
            .collect();
        ax.plot(x, y).map_err(ProbeError::rejected)?;
        ax.yaxis_date(None);
        ax.xaxis_date(None);
        ax.xaxis()
            .set_minor_locator(AutoMinorLocator::new(Some(MinorDivisions::Count(2))))
            .set_minor_formatter(FixedFormatter::new(
                (1..=7).map(|n| n.to_string()).collect(),
            ));
        fig.autofmt_xdate(0.2, 60.0, HAlign::Right, which)
            .map_err(ProbeError::rejected)?;
        let rotation = |minor| ax.xaxis().tick_style(minor).label_rotation;
        Ok(object([
            ("major", numbers([rotation(false)])),
            ("minor", numbers([rotation(true)])),
        ]))
    })
}

/// `test_autofmt_xdate_colorbar_constrained`: the rotation of the major x
/// tick labels of an image with a colorbar.
fn autofmt_with_colorbar() -> Outcome {
    let fig = Figure::new();
    fig.set_layout_engine(LayoutEngine::constrained())
        .map_err(ProbeError::rejected)?;
    let ax = fig.subplot();
    let data =
        Grid::from_vec(2, 3, vec![1.0, 4.0, 6.0, 2.0, 3.0, 5.0]).map_err(ProbeError::rejected)?;
    let image = ax
        .imshow(data, plt::axes::ImshowOptions::default())
        .map_err(ProbeError::rejected)?;
    fig.colorbar(&image, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    fig.autofmt_xdate(0.2, 30.0, HAlign::Right, TickLevel::Major)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(numbers([ax.xaxis().tick_style(false).label_rotation]))
}
