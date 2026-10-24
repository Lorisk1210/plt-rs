//! Subfigures: clearing them, their boxes, resolution, removal, order and
//! staleness, and the ticks of axes inside them.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, ScatterOptions};
use plt::figure::{Figure, SubfigureOptions};
use plt::gridspec::{GridSpec, SubplotSpec};
use plt::layout::LayoutEngine;

use super::super::mpl_tests::{in_test_style, numbers};
use super::texts::subfigures;
use crate::json::Json;
use crate::probes::encode::bbox;
use crate::probes::{Args, Outcome, ProbeError};

/// `test_figure_clear`'s subfigure scenarios (see the Python module).
pub(super) fn figure_clear(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let method = args.str("method")?;
        let clear = |target: &Figure| match method {
            "clf" => {
                target.clf();
            }
            _ => {
                target.clear();
            }
        };
        let fig = Figure::new();
        let mut out = Vec::new();
        let record = |out: &mut Vec<Json>, axes: &[&Axes]| {
            let listed = fig.axes();
            out.push(Json::Array(vec![
                Json::from(listed.len() as f64),
                Json::from(fig.subfigs().len() as f64),
                Json::from(
                    axes.iter()
                        .map(|ax| listed.contains(ax))
                        .collect::<Vec<_>>(),
                ),
            ]));
        };
        let grid = GridSpec::new(1, 2).map_err(ProbeError::rejected)?;
        let cells = [cell(&grid, 0)?, cell(&grid, 1)?];
        let subfig = fig.add_subfigure(&cells[0]);
        let subaxes = subfig.subplot();
        clear(&fig);
        record(&mut out, &[&subaxes]);
        let subfig = fig.add_subfigure(&cells[0]);
        let subaxes = subfig.subplot();
        let mainaxes = add(&fig, &cells[1])?;
        mainaxes.remove();
        record(&mut out, &[&subaxes]);
        let mainaxes = add(&fig, &cells[1])?;
        subaxes.remove();
        record(&mut out, &[&mainaxes]);
        let subaxes = subfig.subplot();
        record(&mut out, &[&mainaxes, &subaxes]);
        clear(&subfig);
        record(&mut out, &[&mainaxes, &subaxes]);
        out.push(Json::from(subfig.axes().len() as f64));
        let subaxes = subfig.subplot();
        clear(&fig);
        record(&mut out, &[&subaxes]);
        let pair = |fig: &Figure| cells.clone().map(|spec| fig.add_subfigure(&spec));
        let subfigs = pair(&fig);
        let subaxes = subfigs.clone().map(|sub| sub.subplot());
        record(&mut out, &[&subaxes[0], &subaxes[1]]);
        clear(&subfigs[0]);
        record(&mut out, &[&subaxes[0], &subaxes[1]]);
        clear(&subfigs[1]);
        let subfigs = pair(&fig);
        let subaxes = subfigs.map(|sub| sub.subplot());
        record(&mut out, &[&subaxes[0], &subaxes[1]]);
        clear(&fig);
        record(&mut out, &[&subaxes[0], &subaxes[1]]);
        Ok(Json::Array(out))
    })
}

/// Cell `index` of a one-row grid.
fn cell(grid: &GridSpec, index: usize) -> Result<SubplotSpec, ProbeError> {
    grid.cell(0, index).map_err(ProbeError::rejected)
}

/// `fig.add_subplot(spec)`.
fn add(fig: &Figure, spec: &SubplotSpec) -> Result<Axes, ProbeError> {
    fig.add_subplot_spec(spec).map_err(ProbeError::rejected)
}

/// The extents of a grid of subfigures, or of subfigures on blocks of a
/// grid (then given an axes each and laid out).
pub(super) fn subfigure_bbox(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let count = |key: &str| {
            args.opt_i64(key)?
                .map_or(Ok(1), usize::try_from)
                .map_err(ProbeError::rejected)
        };
        let (nrows, ncols) = (count("nrows")?, count("ncols")?);
        let fig = Figure::new();
        if args.opt_str("layout")? == Some("constrained") {
            fig.set_layout_engine(LayoutEngine::constrained())
                .map_err(ProbeError::rejected)?;
        }
        let parts = match args.get("specs") {
            None => {
                let options = SubfigureOptions {
                    wspace: args.opt_f64("wspace")?,
                    hspace: args.opt_f64("hspace")?,
                    ..SubfigureOptions::default()
                };
                fig.subfigures(nrows, ncols, options)
                    .map_err(ProbeError::rejected)?
                    .as_slice()
                    .to_vec()
            }
            Some(specs) => blocks(&fig, nrows, ncols, specs)?,
        };
        let out = Json::Array(parts.iter().map(|part| bbox(&part.bbox())).collect());
        if args.get("specs").is_some() {
            for part in &parts {
                part.subplot();
            }
            fig.layout();
        }
        Ok(out)
    })
}

/// Subfigures on the blocks `[[r0, r1], [c0, c1]]` of an
/// `nrows × ncols` grid.
fn blocks(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
    specs: &Json,
) -> Result<Vec<Figure>, ProbeError> {
    let grid = GridSpec::new(nrows, ncols).map_err(ProbeError::rejected)?;
    let range = |value: &Json| match value.as_f64_vec().as_deref() {
        Some(&[start, end]) => Ok(start as usize..end as usize),
        _ => Err(ProbeError::BadArgs(format!("bad range {value}"))),
    };
    specs
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|spec| match spec.as_array().unwrap_or(&[]) {
            [rows, cols] => {
                let block = grid
                    .span(range(rows)?, range(cols)?)
                    .map_err(ProbeError::rejected)?;
                Ok(fig.add_subfigure(&block))
            }
            _ => Err(ProbeError::BadArgs(format!("bad block {spec}"))),
        })
        .collect()
}

/// A subfigure's and its figure's resolution before and after setting
/// the subfigure's.
pub(super) fn subfigure_dpi(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let [width, height] = plt::rc::params().figure_figsize;
        let fig = Figure::with_size(width, height, 100.0);
        let sub = subfigures(&fig, 1, 1)?.remove(0);
        let mut out = vec![sub.dpi(), fig.dpi()];
        sub.set_dpi(200.0);
        out.extend([sub.dpi(), fig.dpi()]);
        Ok(numbers(out))
    })
}

/// The subfigures left after removing one, and whether `subfigs` lists a
/// grid in row order.
pub(super) fn subfigure_remove(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let parts = subfigures(&fig, 2, 2)?;
        parts[3].subplot();
        parts[3].remove();
        let left = fig.subfigs().len();
        let fig = Figure::new();
        let grid = subfigures(&fig, 4, 3)?;
        let listed = fig.subfigs();
        let in_order =
            grid.len() == listed.len() && grid.iter().zip(&listed).all(|(a, b)| a.id() == b.id());
        Ok(Json::Array(vec![
            Json::from(left as f64),
            Json::from(in_order),
        ]))
    })
}

/// The staleness of a figure and its nested subfigures along
/// `test_subfigure_stale_propagation`.
pub(super) fn stale(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let draw = || fig.canvas().draw().map_err(ProbeError::rejected);
        draw()?;
        let mut out = vec![fig.is_stale()];
        let sfig1 = subfigures(&fig, 1, 1)?.remove(0);
        out.push(fig.is_stale());
        draw()?;
        out.extend([fig.is_stale(), sfig1.is_stale()]);
        let sfig2 = subfigures(&sfig1, 1, 1)?.remove(0);
        out.extend([fig.is_stale(), sfig1.is_stale()]);
        draw()?;
        out.extend([fig.is_stale(), sfig1.is_stale(), sfig2.is_stale()]);
        Ok(Json::from(out))
    })
}

/// The x ticks of `test_subfigure_ticks` at 120 and 300 dpi.
pub(super) fn subfigure_ticks(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::with_size(10.0, 3.0, plt::rc::params().figure_dpi);
        fig.set_layout_engine(LayoutEngine::constrained())
            .map_err(ProbeError::rejected)?;
        let options = SubfigureOptions {
            wspace: Some(0.01),
            width_ratios: Some(vec![7.0, 2.0]),
            ..SubfigureOptions::default()
        };
        let parts = fig
            .subfigures(1, 2, options)
            .map_err(ProbeError::rejected)?;
        let left = &parts.as_slice()[0];
        let grid = GridSpec::new(1, 14).map_err(ProbeError::rejected)?;
        let block = |cols| grid.span(0..1, cols).map_err(ProbeError::rejected);
        let ax1 = add(left, &block(0..1)?)?;
        scatter(&ax1, [-56.468_815_048_217_76, 24.179_891_162_109_396])?;
        let ax2 = add(left, &block(1..3)?)?;
        ax2.share_y(&ax1).map_err(ProbeError::rejected)?;
        scatter(&ax2, [-126.535_727_005_004_9, 94.684_567_367_553_68])?;
        let ax3 = add(left, &block(3..14)?)?;
        ax3.share_y(&ax1).map_err(ProbeError::rejected)?;
        let mut out = Vec::new();
        for dpi in [120.0, 300.0] {
            fig.set_dpi(dpi);
            fig.layout();
            out.push(numbers(
                ax2.xaxis().ticklocs(false).map_err(ProbeError::rejected)?,
            ));
        }
        Ok(Json::Array(out))
    })
}

/// A scatter of two points at heights 1500 and 3600.
fn scatter(ax: &Axes, x: [f64; 2]) -> Result<(), ProbeError> {
    ax.scatter_with(x.to_vec(), vec![1500.0, 3600.0], ScatterOptions::default())
        .map(drop)
        .map_err(ProbeError::rejected)
}
