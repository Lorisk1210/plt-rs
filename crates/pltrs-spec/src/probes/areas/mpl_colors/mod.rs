//! Probe operations for the `mpl_colors` area: colour conversion,
//! colormaps, norms and shading ported from matplotlib's
//! `tests/test_colors.py` (see `spec/reference/probes/test_mpl_colors.py`).
//!
//! Most operations are those of the `colors`, `colormaps`, `norms` and
//! `lightsource` areas; the colormap calls they lack are in
//! [`colormap_ops`], the norm and colorbar ones in [`norm_ops`] and the
//! registries in [`registries`].

mod colormap_ops;
mod norm_ops;
mod registries;

use super::text_metrics::with_rc;
use super::{colormaps, colors, lightsource, norms};
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("colors.to_rgba", colors::to_rgba),
    Op::new("colors.to_rgb", |args| {
        with_rc(args, || colors::to_rgb(args))
    }),
    Op::new("colors.to_hex", |args| {
        with_rc(args, || colors::to_hex_op(args))
    }),
    Op::new("colors.to_rgba_array", colors::to_rgba_array_op),
    Op::new("colors.is_color_like", colors::is_color_like),
    Op::new("colors.same_color", colors::same_color_op),
    Op::new("colors.rgb_to_hsv", colors::rgb_to_hsv_op),
    Op::new("colors.hsv_to_rgb", colors::hsv_to_rgb_op),
    Op::new("colors.color_sequences", colors::color_sequences),
    Op::new("colors.color_sequence", colors::color_sequence_op),
    Op::new("colormaps.info", colormaps::info),
    Op::new("colormaps.sample", colormaps::sample),
    Op::new("colormaps.sample_alpha", colormaps::sample_alpha),
    Op::new("colormaps.get_cmap", colormaps::get_cmap),
    Op::new("colors.ListedColormap", colormaps::listed),
    Op::new("colors.LinearSegmentedColormap", colormaps::segmented),
    Op::new(
        "colors.LinearSegmentedColormap.from_list",
        colormaps::from_list,
    ),
    Op::new("colors.Colormap.resampled", colormap_ops::resampled),
    Op::new("colors.ListedColormap.with_alpha", colormap_ops::with_alpha),
    Op::new("colors.Colormap.__eq__", colormap_ops::equal_after),
    Op::new(
        "colors.from_levels_and_colors.with_extremes",
        colormap_ops::levels_with_extremes,
    ),
    Op::new("colormaps.register", registries::register_cmap),
    Op::new("colormaps.register.get", registries::register_and_get_cmap),
    Op::new("color_sequences.register", registries::register_sequence),
    Op::new("color_sequences.__getitem__", registries::get_sequence),
    Op::new(
        "color_sequences.unregister",
        registries::unregister_sequence,
    ),
    Op::new("colors.Normalize.__call__", |args| {
        norms::call("Normalize", args)
    }),
    Op::new("colors.Normalize.inverse", |args| {
        norms::inverse("Normalize", args)
    }),
    Op::new("colors.LogNorm.__call__", |args| {
        norms::call("LogNorm", args)
    }),
    Op::new("colors.LogNorm.inverse", |args| {
        norms::inverse("LogNorm", args)
    }),
    Op::new("colors.SymLogNorm.__call__", |args| {
        norms::call("SymLogNorm", args)
    }),
    Op::new("colors.SymLogNorm.inverse", |args| {
        norms::inverse("SymLogNorm", args)
    }),
    Op::new("colors.PowerNorm.__call__", |args| {
        norms::call("PowerNorm", args)
    }),
    Op::new("colors.PowerNorm.inverse", |args| {
        norms::inverse("PowerNorm", args)
    }),
    Op::new("colors.BoundaryNorm.__call__", |args| {
        norms::call("BoundaryNorm", args)
    }),
    Op::new("colors.TwoSlopeNorm.__call__", |args| {
        norms::call("TwoSlopeNorm", args)
    }),
    Op::new("colors.TwoSlopeNorm.inverse", |args| {
        norms::inverse("TwoSlopeNorm", args)
    }),
    Op::new("colors.TwoSlopeNorm.autoscale", |args| {
        norms::autoscale("TwoSlopeNorm", args, false)
    }),
    Op::new("colors.TwoSlopeNorm.autoscale_None", |args| {
        norms::autoscale("TwoSlopeNorm", args, true)
    }),
    Op::new("colors.TwoSlopeNorm.scaled", norm_ops::two_slope_scaled),
    Op::new("colors.CenteredNorm.__call__", |args| {
        norms::call("CenteredNorm", args)
    }),
    Op::new("colors.CenteredNorm.autoscale_None", |args| {
        norms::autoscale("CenteredNorm", args, true)
    }),
    Op::new("colors.AsinhNorm.__call__", |args| {
        norms::call("AsinhNorm", args)
    }),
    Op::new("colors.CenteredNorm", norm_ops::centered_steps),
    Op::new("colors.FuncNorm.__call__", norm_ops::func_call),
    Op::new("colors.FuncNorm.inverse", norm_ops::func_inverse),
    Op::new(
        "cm.ScalarMappable.to_rgba",
        norm_ops::scalar_mappable_to_rgba,
    ),
    Op::new("colors.MultiNorm", norms::multi::construct),
    Op::new("colors.MultiNorm.__call__", norms::multi::call),
    Op::new("colors.MultiNorm.inverse", norms::multi::inverse),
    Op::new("colors.MultiNorm.autoscale", |args| {
        norms::multi::autoscale(args, false)
    }),
    Op::new("colors.MultiNorm.autoscale_None", |args| {
        norms::multi::autoscale(args, true)
    }),
    Op::new(
        "colorizer.Colorizer.to_rgba",
        norms::multi::colorizer_to_rgba,
    ),
    Op::new("colorizer.Colorizer", norm_ops::colorizer_limits),
    Op::new("colorbar.Colorbar.get_ticks", norm_ops::colorbar_ticks),
    Op::new("colorbar.Colorbar.ax.get_ylim", norm_ops::colorbar_ylim),
    Op::new("lightsource.hillshade", lightsource::hillshade),
    Op::new("lightsource.shade", lightsource::shade),
];
