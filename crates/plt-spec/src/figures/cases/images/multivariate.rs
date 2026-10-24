//! The multivariate cases: `pcolormesh` and quad-mesh `pcolorfast` of data
//! of several variates through bivariate and multivariate colormaps.

use plt::axes::{Mesh, PcolorOptions, PcolorfastOptions, Shading};
use plt::collections::CollectionProps;
use plt::colors::multivar::MultiColormap;
use plt::colors::{NormName, NormSpec, Normalize, PowerNorm};
use plt::figure::Figure;
use plt::image::{MultiField, MultivarColorizerOptions};
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};

use super::data::{grid, zip_with};
use crate::figures::CaseResult;

/// `_variate_fields()`: a wave, a positive exponential and a saddle on the
/// 8 × 12 grid of `_grid(12, 8)`.
fn fields() -> [Grid<f64>; 3] {
    let (xx, yy) = grid(12, 8);
    [
        zip_with(&xx, &yy, |x, y| x.sin() * y.cos()),
        zip_with(&xx, &yy, |x, y| f64::midpoint(x, y).exp()),
        zip_with(&xx, &yy, |x, y| x * y / 6.0),
    ]
}

/// Colouring through the colormap `cmap` with `options` applied.
fn colors(
    cmap: &str,
    options: impl FnOnce(MultivarColorizerOptions) -> MultivarColorizerOptions,
) -> Result<MultivarColorizerOptions, Box<dyn std::error::Error>> {
    Ok(options(MultivarColorizerOptions::new(MultiColormap::get(
        cmap,
    )?)))
}

/// `pcolormesh` options with only the colouring set.
fn mesh_options(multivariate: MultivarColorizerOptions) -> PcolorOptions {
    PcolorOptions {
        multivariate: Some(multivariate),
        ..PcolorOptions::default()
    }
}

/// A scale-named norm.
fn named(name: NormName) -> NormSpec {
    NormSpec::Name(name)
}

pub(super) fn pcolormesh_bivariate(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let [a, b, _] = fields();
    let both = MultiField::new([a, b])?;
    axes[(0, 0)].pcolormesh(
        Mesh::new(both.clone()),
        mesh_options(colors("BiPeak", |o| o)?),
    )?;
    let (xx, yy) = grid(13, 9);
    let options = colors("BiOrangeBlue", |o| {
        o.norms([named(NormName::Linear), named(NormName::Log)])
            .limits(Some(vec![Some(-0.5), None]), Some(vec![Some(0.5), None]))
    })?;
    axes[(0, 1)].pcolormesh(Mesh::from_coords(xx, yy, both), mesh_options(options))?;
    Ok(())
}

pub(super) fn pcolormesh_bicone_masked_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(
        [-3.0, 3.0],
        [-2.0, 2.0],
        LineProps::default().color("k").linewidth(4.0),
    )?;
    let [a, _, c] = fields();
    let holed = Grid::from_fn(8, 12, |r, col| {
        if (r * 12 + col) % 7 == 0 {
            f64::NAN
        } else {
            a[(r, col)]
        }
    });
    let options = colors("BiCone", |o| {
        o.norms([
            NormSpec::norm(Normalize::new(-1.0, 1.0)),
            named(NormName::Linear),
        ])
    })?;
    let options = PcolorOptions {
        shading: Some(Shading::Nearest),
        props: CollectionProps::default().alpha(0.6),
        ..mesh_options(options)
    };
    let mesh = Mesh::from_coords(
        linspace(-3.0, 3.0, 12),
        linspace(-2.0, 2.0, 8),
        MultiField::new([holed, c])?,
    );
    ax.pcolormesh(mesh, options)?;
    Ok(())
}

pub(super) fn pcolormesh_multivariate(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 3)?;
    let [a, b, c] = fields();
    let add = MultiField::new([a.clone(), c.clone()])?;
    axes[(0, 0)].pcolormesh(Mesh::new(add), mesh_options(colors("2VarAddA", |o| o)?))?;
    let sub = colors("2VarSubA", |o| {
        o.norms([NormSpec::norm(PowerNorm::new(0.5)), named(NormName::Linear)])
    })?;
    let swapped = MultiField::new([b.clone(), a.clone()])?;
    axes[(0, 1)].pcolormesh(Mesh::new(swapped), mesh_options(sub))?;
    let three = colors("3VarAddA", |o| {
        o.norms([
            named(NormName::Linear),
            named(NormName::Log),
            named(NormName::Linear),
        ])
        .limits(
            Some(vec![Some(-0.5), None, None]),
            Some(vec![Some(0.5), None, Some(0.5)]),
        )
    })?;
    axes[(0, 2)].pcolormesh(Mesh::new(MultiField::new([a, b, c])?), mesh_options(three))?;
    Ok(())
}

pub(super) fn pcolorfast_multivariate_quadmesh(fig: &Figure) -> CaseResult {
    let (xx, yy) = grid(13, 9);
    let skewed = zip_with(&xx, &yy, |x, y| x + 0.3 * y);
    let options = PcolorfastOptions {
        multivariate: Some(colors("3VarAddA", |o| {
            o.norms([
                named(NormName::Linear),
                named(NormName::Symlog),
                named(NormName::Asinh),
            ])
        })?),
        ..PcolorfastOptions::default()
    };
    fig.subplot().pcolorfast(
        Some((skewed.into(), yy.into())),
        MultiField::new(fields())?,
        options,
    )?;
    Ok(())
}
