//! The tests of `axisartist/tests/test_axislines.py`: axes drawn by axis
//! artists, axis lines through zero with arrow heads, and parasite axes
//! of an axisartist host.

use std::f64::consts::TAU;

use pltrs::axes::{Mesh, PcolorOptions};
use pltrs::axisartist::{self, AuxTransform, AxisArtist, AxislineStyle};
use pltrs::contour::{ContourOptions, QuadCoords, QuadField};
use pltrs::figure::Figure;
use pltrs::layout::LayoutEngine;
use pltrs::numeric::Grid;
use pltrs::prelude::ArtistHandle;

use super::remove_ticks_and_titles;
use crate::figures::{CaseResult, FigureCase};

/// What the axisartist tests set "until the test image is regenerated".
const TICKS_IN: &[(&str, &str)] = &[("xtick.direction", "in"), ("ytick.direction", "in")];

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_toolkits/SubplotZero", subplot_zero),
    FigureCase::new("mpl_toolkits/Subplot", subplot),
    FigureCase::new("mpl_toolkits/ParasiteAxesAuxTrans", parasite_axes_aux_trans).with_rc(TICKS_IN),
    FigureCase::new("mpl_toolkits/axisline_style", axisline_style)
        .sized(2.0, 2.0, 100.0)
        .with_rc(TICKS_IN),
    FigureCase::new(
        "mpl_toolkits/axisline_style_size_color",
        axisline_style_size_color,
    )
    .sized(2.0, 2.0, 100.0)
    .with_rc(TICKS_IN),
    FigureCase::new("mpl_toolkits/axisline_style_tight", axisline_style_tight)
        .sized(2.0, 2.0, 100.0)
        .with_rc(TICKS_IN),
    FigureCase::new("mpl_toolkits/subplotzero_ylabel", subplotzero_ylabel),
];

/// `ax.axis[name]`.
pub(super) fn artist(ax: &axisartist::AxisArtistAxes, name: &str) -> Result<AxisArtist, String> {
    ax.axis(name)
        .ok_or_else(|| format!("no axis artist {name:?}"))
}

/// `xx = np.arange(0, 2 * np.pi, 0.01)` and `np.sin(xx)`.
fn sine() -> (Vec<f64>, Vec<f64>) {
    let xx: Vec<f64> = (0..=u16::MAX)
        .map(|i| f64::from(i) * 0.01)
        .take_while(|&x| x < TAU)
        .collect();
    let sin = xx.iter().map(|x| x.sin()).collect();
    (xx, sin)
}

fn subplot_zero(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    let xzero = artist(&ax, "xzero")?;
    xzero.set_visible(true);
    xzero.label().set_text("Axis Zero");
    for name in ["top", "right"] {
        artist(&ax, name)?.set_visible(false);
    }
    let (xx, sin) = sine();
    ax.ax().plot(xx, sin)?;
    ax.ax().set_ylabel("Test");
    Ok(())
}

fn subplot(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::new(fig.subplot());
    let (xx, sin) = sine();
    ax.ax().plot(xx, sin)?;
    ax.ax().set_ylabel("Test");
    for name in ["left", "right"] {
        artist(&ax, name)?.major_ticks().set_tick_out(false);
    }
    artist(&ax, "bottom")?.label().set_text("Tk0");
    Ok(())
}

/// `test_ParasiteAxesAuxTrans`'s data: ones, a two at `(2, 2)`, zeros in
/// the first and the second-to-last rows and columns.
fn aux_trans_data() -> Grid<f64> {
    Grid::from_fn(6, 6, |r, c| {
        if r == 0 || r == 4 || c == 0 || c == 4 {
            0.0
        } else if (r, c) == (2, 2) {
            2.0
        } else {
            1.0
        }
    })
}

fn parasite_axes_aux_trans(fig: &Figure) -> CaseResult {
    let data = aux_trans_data();
    let coords: Vec<f64> = (0..6).map(f64::from).collect();
    let field = || {
        QuadField::new(data.clone()).with_coords(QuadCoords::Axes {
            x: coords.clone(),
            y: coords.clone(),
        })
    };
    let cells = Grid::from_fn(5, 5, |r, c| data[(r, c)]);
    let mesh = || Mesh::from_coords(coords.clone(), coords.clone(), cells.clone());
    let mut last = None;
    for index in 1..=3 {
        let ax1 = axisartist::AxisArtistAxes::new(fig.add_subplot(1, 3, index)?);
        // `get_aux_axes(IdentityTransform(), viewlim_mode=None)`: the view
        // limits of a parasite with a transform do not place what it
        // draws, its host's do.
        let ax2 = ax1.aux_axes(&AuxTransform::identity());
        match index {
            1 => {
                ax2.pcolor(mesh(), PcolorOptions::default())?;
            }
            2 => {
                ax2.pcolormesh(mesh(), PcolorOptions::default())?;
            }
            _ => {
                ax2.contourf(&field(), ContourOptions::default())?;
            }
        }
        ax1.ax().set_xlim(0.0, 5.0)?.set_ylim(0.0, 5.0)?;
        last = Some(ax2);
    }
    let ax2 = last.ok_or("three axes were made")?;
    ax2.contour(
        &field(),
        ContourOptions {
            colors: Some(vec!["k".into()]),
            ..ContourOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The `test_axisline_style*` tests: a filled arrow head `xzero_size`
/// large in `xzero_face` on `xzero`, `yzero_style` on `yzero`, the edges
/// hidden.
fn zero_arrows(
    fig: &Figure,
    xzero_size: f64,
    xzero_face: Option<&str>,
    yzero_style: &str,
) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    let xzero = artist(&ax, "xzero")?;
    xzero.set_axisline_style(Some(AxislineStyle::FilledArrow {
        size: xzero_size,
        facecolor: xzero_face.map(Into::into),
    }));
    xzero.set_visible(true);
    let yzero = artist(&ax, "yzero")?;
    yzero.set_axisline_style(Some(yzero_style.parse()?));
    yzero.set_visible(true);
    for name in ["left", "right", "bottom", "top"] {
        artist(&ax, name)?.set_visible(false);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

fn axisline_style(fig: &Figure) -> CaseResult {
    zero_arrows(fig, 1.0, None, "->")
}

fn axisline_style_size_color(fig: &Figure) -> CaseResult {
    zero_arrows(fig, 2.0, Some("r"), "->, size=1.5")
}

fn axisline_style_tight(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::tight())?;
    zero_arrows(fig, 5.0, Some("g"), "->, size=8")
}

fn subplotzero_ylabel(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    ax.ax().set_xlim(-3.0, 7.0)?.set_ylim(-3.0, 7.0)?;
    ax.ax().set_xlabel("x").set_ylabel("y");
    let zero_axes = [artist(&ax, "xzero")?, artist(&ax, "yzero")?];
    for axis in &zero_axes {
        axis.set_visible(true);
    }
    for name in ["left", "right", "bottom", "top"] {
        artist(&ax, name)?.set_visible(false);
    }
    for axis in &zero_axes {
        axis.set_axisline_style(Some(AxislineStyle::SimpleArrow { size: 1.0 }));
    }
    Ok(())
}
