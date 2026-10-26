//! Regions put back in part and elsewhere, artists of every kind drawn
//! one by one over the drawn figure, clipped and unclipped, and an axes
//! redrawn in its frame.

use std::error::Error;

use plt::artist::RawHandle;
use plt::figure::PixelBox;
use plt::legend::{LegendLoc, LegendOptions};
use plt::offsetbox::AnchorLoc;
use plt::patches::Circle;
use plt::prelude::*;
use plt::transforms::{Bbox, Point};

use super::{blitting, shifted, wave_x};
use crate::figures::{CaseResult, FigureCase};

/// The axes scrolled 60 pixels to the left (`restore_region`'s docstring:
/// `bbox=(x1 + dx, y1, x2, y2), xy=(x1 - dx, y1)`, both in canvas pixels
/// from the top), then the box around the legend copied to the top-left
/// corner (`xy` alone).
fn restore_region_bbox_xy(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = wave_x();
    ax.plot_with(
        &x,
        shifted(&x, 0.0, f64::sin),
        LineProps::default().label("sin"),
    )?;
    ax.plot_with(
        &x,
        shifted(&x, 0.0, f64::cos),
        LineProps::default().label("cos"),
    )?;
    ax.legend(LegendOptions {
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperRight)),
        ..LegendOptions::default()
    })?;
    let canvas = fig.canvas();
    canvas.draw()?;
    let region = canvas.copy_from_bbox(&ax.bbox())?;
    let PixelBox {
        x0: x1,
        y0: y1,
        x1: x2,
        y1: y2,
    } = region.extents();
    let dx = 60;
    let part = Bbox::from_extents((x1 + dx) as f64, y1 as f64, x2 as f64, y2 as f64);
    let corner = Point::new((x1 - dx) as f64, y1 as f64);
    canvas.restore_region_with(&region, Some(part), Some(corner))?;
    let legend = canvas.copy_from_bbox(&Bbox::from_bounds(480.5, 360.5, 100.0, 60.0))?;
    canvas.restore_region_with(&legend, None, Some(Point::new(10.0, 10.0)))?;
    Ok(())
}

/// An axes over the middle of the figure showing [0, 1]².
fn overhanging_axes(fig: &Figure) -> Result<Axes, Box<dyn Error>> {
    let ax = fig.add_axes([0.2, 0.2, 0.6, 0.6])?;
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    Ok(ax)
}

/// A line, a patch, a text and a collection reaching past the axes,
/// animated.
fn overhanging(ax: &Axes) -> Result<Vec<RawHandle>, Box<dyn Error>> {
    let line = ax.plot_with(
        vec![-0.3, 1.3],
        vec![0.1, 0.8],
        LineProps::default().linewidth(4.0),
    )?;
    let patch = ax.add_patch_with(
        Circle::new((0.9, 0.15), 0.3),
        PatchProps::default().facecolor("tab:orange").alpha(0.8),
    );
    let text = ax.text(0.6, 0.55, "text past the edge");
    text.set_fontsize(14.0).set_clip_on(true);
    let dots = ax.scatter_with(
        vec![0.05, 0.5, 0.97],
        vec![0.97, 1.0, 0.4],
        ScatterOptions {
            sizes: Some(vec![600.0]),
            colors: ScatterColors::PerPoint(
                ["tab:green", "tab:red", "tab:purple"]
                    .map(Color::from)
                    .to_vec(),
            ),
            ..ScatterOptions::default()
        },
    )?;
    let artists = vec![
        line.raw().clone(),
        patch.raw().clone(),
        text.raw().clone(),
        dots.raw().clone(),
    ];
    for artist in &artists {
        artist.set_animated(true);
    }
    Ok(artists)
}

fn draw_artist_clipped(fig: &Figure) -> CaseResult {
    let ax = overhanging_axes(fig)?;
    let artists = overhanging(&ax)?;
    fig.canvas().draw()?;
    for artist in &artists {
        ax.draw_artist(artist)?;
    }
    Ok(())
}

/// The artists of `draw_artist_clipped` with clipping off, and a text of
/// the figure drawn by the figure.
fn draw_artist_unclipped(fig: &Figure) -> CaseResult {
    let ax = overhanging_axes(fig)?;
    let artists = overhanging(&ax)?;
    for artist in &artists {
        artist.set_clip_on(false);
    }
    let note = fig.text(0.02, 0.03, "figure text");
    note.set_fontsize(12.0).set_animated(true);
    fig.canvas().draw()?;
    for artist in &artists {
        ax.draw_artist(artist)?;
    }
    fig.draw_artist(&note)?;
    Ok(())
}

/// After a new background colour, data and title, `redraw_in_frame`
/// draws the background, spines and line again; the title, ticks and
/// labels keep their first drawing.
fn redraw_in_frame(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = wave_x();
    let line = ax.plot(x.clone(), shifted(&x, 0.0, f64::sin))?;
    ax.set_title("before");
    ax.set_xlabel("x");
    fig.canvas().draw()?;
    ax.set_facecolor("lightyellow");
    line.set_data(&x, shifted(&x, 0.0, f64::cos));
    ax.set_title("after");
    ax.redraw_in_frame()?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    blitting("blitting/restore_region_bbox_xy", restore_region_bbox_xy),
    blitting("blitting/draw_artist_clipped", draw_artist_clipped).sized(4.0, 3.0, 100.0),
    blitting("blitting/draw_artist_unclipped", draw_artist_unclipped).sized(4.0, 3.0, 100.0),
    blitting("blitting/redraw_in_frame", redraw_in_frame),
];
