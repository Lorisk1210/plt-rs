//! `figures/cases_images.py`: imshow (interpolation, stages,
//! origin/extent/aspect, RGB(A), alpha, norms, extreme colours, masks),
//! matshow, spy, figimage, pcolor, pcolormesh, pcolorfast, `NonUniformImage`
//! and hexbin.

mod data;
mod imshow;
mod meshes;
mod nonuniform;
mod placement;
mod values;

use plt::image::{Interpolation, InterpolationStage};

use crate::figures::FigureCase;

/// `images/interp_<name>` on a 5 × 2.5 in canvas.
const fn interp(
    name: &'static str,
    build: fn(&plt::figure::Figure) -> crate::figures::CaseResult,
) -> FigureCase {
    FigureCase::new(name, build).sized(5.0, 2.5, 100.0)
}

/// A case on a 4 × 3 in canvas.
const fn small(
    name: &'static str,
    build: fn(&plt::figure::Figure) -> crate::figures::CaseResult,
) -> FigureCase {
    FigureCase::new(name, build).sized(4.0, 3.0, 100.0)
}

pub(crate) const CASES: &[FigureCase] = &[
    interp("images/interp_none", |fig| {
        imshow::interpolation(fig, Interpolation::None)
    }),
    interp("images/interp_nearest", |fig| {
        imshow::interpolation(fig, Interpolation::Nearest)
    }),
    interp("images/interp_bilinear", |fig| {
        imshow::interpolation(fig, Interpolation::Bilinear)
    }),
    interp("images/interp_bicubic", |fig| {
        imshow::interpolation(fig, Interpolation::Bicubic)
    }),
    interp("images/interp_spline16", |fig| {
        imshow::interpolation(fig, Interpolation::Spline16)
    }),
    interp("images/interp_spline36", |fig| {
        imshow::interpolation(fig, Interpolation::Spline36)
    }),
    interp("images/interp_hanning", |fig| {
        imshow::interpolation(fig, Interpolation::Hanning)
    }),
    interp("images/interp_hamming", |fig| {
        imshow::interpolation(fig, Interpolation::Hamming)
    }),
    interp("images/interp_hermite", |fig| {
        imshow::interpolation(fig, Interpolation::Hermite)
    }),
    interp("images/interp_kaiser", |fig| {
        imshow::interpolation(fig, Interpolation::Kaiser)
    }),
    interp("images/interp_quadric", |fig| {
        imshow::interpolation(fig, Interpolation::Quadric)
    }),
    interp("images/interp_catrom", |fig| {
        imshow::interpolation(fig, Interpolation::Catrom)
    }),
    interp("images/interp_gaussian", |fig| {
        imshow::interpolation(fig, Interpolation::Gaussian)
    }),
    interp("images/interp_bessel", |fig| {
        imshow::interpolation(fig, Interpolation::Bessel)
    }),
    interp("images/interp_mitchell", |fig| {
        imshow::interpolation(fig, Interpolation::Mitchell)
    }),
    interp("images/interp_sinc", |fig| {
        imshow::interpolation(fig, Interpolation::Sinc)
    }),
    interp("images/interp_lanczos", |fig| {
        imshow::interpolation(fig, Interpolation::Lanczos)
    }),
    interp("images/interp_blackman", |fig| {
        imshow::interpolation(fig, Interpolation::Blackman)
    }),
    interp("images/interp_antialiased", |fig| {
        imshow::interpolation(fig, Interpolation::Antialiased)
    }),
    interp("images/interp_auto", |fig| {
        imshow::interpolation(fig, Interpolation::Auto)
    }),
    interp("images/interp_stage_data", |fig| {
        imshow::stage(fig, InterpolationStage::Data)
    }),
    interp("images/interp_stage_rgba", |fig| {
        imshow::stage(fig, InterpolationStage::Rgba)
    }),
    interp("images/filterrad_sinc", imshow::filterrad_sinc),
    FigureCase::new("images/imshow_default", imshow::imshow_default),
    FigureCase::new("images/origin_lower", imshow::origin_lower).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/extent", imshow::extent).sized(4.0, 3.0, 100.0),
    interp(
        "images/extent_origin_lower_and_reversed",
        imshow::extent_origin_lower_and_reversed,
    ),
    interp(
        "images/aspect_auto_and_numeric",
        imshow::aspect_auto_and_numeric,
    ),
    FigureCase::new("images/xlim_zoom", imshow::xlim_zoom).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/rgb_float_uint8_rgba", imshow::rgb_float_uint8_rgba)
        .sized(6.0, 2.5, 100.0),
    FigureCase::new(
        "images/rgba_uint8_downsampled",
        imshow::rgba_uint8_downsampled,
    )
    .sized(3.0, 3.0, 100.0),
    interp(
        "images/alpha_scalar_and_array",
        imshow::alpha_scalar_and_array,
    ),
    FigureCase::new("images/vmin_vmax", imshow::vmin_vmax).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/lognorm", imshow::lognorm).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/symlognorm", imshow::symlognorm).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/powernorm", imshow::powernorm).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/boundarynorm", imshow::boundarynorm).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/cmap_resampled", imshow::cmap_resampled).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/cmap_extremes", imshow::cmap_extremes).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/masked_default", imshow::masked_default).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/matshow", imshow::matshow).sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "images/spy_image_and_markers",
        imshow::spy_image_and_markers,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("images/figimage", imshow::figimage).sized(4.0, 3.0, 100.0),
    FigureCase::new("images/clip_path_circle", imshow::clip_path_circle).sized(4.0, 4.0, 100.0),
    small("images/pcolor_default", meshes::pcolor_default),
    small("images/pcolor_edges_masked", meshes::pcolor_edges_masked),
    small("images/pcolormesh_flat", meshes::pcolormesh_flat),
    small("images/pcolormesh_nearest", meshes::pcolormesh_nearest),
    small("images/pcolormesh_gouraud", meshes::pcolormesh_gouraud),
    small(
        "images/pcolormesh_auto_same_shape",
        meshes::pcolormesh_auto_same_shape,
    ),
    small(
        "images/pcolormesh_nonuniform",
        meshes::pcolormesh_nonuniform,
    ),
    small(
        "images/pcolormesh_curvilinear_edges",
        meshes::pcolormesh_curvilinear_edges,
    ),
    small(
        "images/pcolormesh_alpha_log_axes",
        meshes::pcolormesh_alpha_log_axes,
    ),
    small("images/pcolorfast_uniform", meshes::pcolorfast_uniform),
    small(
        "images/pcolorfast_nonuniform",
        meshes::pcolorfast_nonuniform,
    ),
    small("images/pcolorfast_quadmesh", meshes::pcolorfast_quadmesh),
    small("images/nonuniform_nearest", nonuniform::nearest),
    small(
        "images/nonuniform_bilinear_relim",
        nonuniform::bilinear_relim,
    ),
    small("images/nonuniform_rgb_log", nonuniform::rgb_log),
    FigureCase::new("images/hexbin_default", meshes::hexbin_default),
    small(
        "images/hexbin_gridsize_tuple",
        meshes::hexbin_gridsize_tuple,
    ),
    small("images/hexbin_log", meshes::hexbin_log),
    small("images/hexbin_mincnt_extent", meshes::hexbin_mincnt_extent),
    small("images/hexbin_reduce_mean", meshes::hexbin_reduce_mean),
    interp("images/log_axes_resampled", placement::log_axes_resampled),
    FigureCase::new("images/affine_transforms", placement::affine_transforms),
    FigureCase::new(
        "images/nan_inf_interpolation_stages",
        values::nan_inf_interpolation_stages,
    )
    .sized(6.0, 2.5, 100.0),
    FigureCase::new("images/resample_factors", values::resample_factors).sized(6.0, 2.2, 100.0),
    FigureCase::new("images/origin_extent_grid", placement::origin_extent_grid)
        .sized(6.0, 4.0, 100.0),
    FigureCase::new(
        "images/clip_off_beyond_axes",
        placement::clip_off_beyond_axes,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "images/clip_path_rotated_rectangle",
        placement::clip_path_rotated_rectangle,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("images/composite_alpha", values::composite_alpha).sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "images/bool_and_integer_arrays",
        values::bool_and_integer_arrays,
    )
    .sized(6.0, 2.5, 100.0),
    small(
        "images/pcolorfast_nonuniform_alpha",
        values::pcolorfast_nonuniform_alpha,
    ),
];
