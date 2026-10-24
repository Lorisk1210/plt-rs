//! `figures/cases_mpl_tests_image.py`: matplotlib's own tests of images
//! (`test_image.py`) that draw and then check pixels or the saved document,
//! one case per figure they check, in the test suite's default style.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, ImshowOptions, Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::image::{ColorizerOptions, Extent, ImageData, Interpolation, InterpolationStage, Origin};
use plt::numeric::Grid;
use plt::rc::Aspect;

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_image/image_edges", image_edges)
            .sized(1.0, 1.0, 100.0)
            // `savefig(facecolor=(0, 1, 0))`.
            .with_rc_setup(|rc| Ok(rc.set("savefig.facecolor", "#00ff00")?)),
        classic("mpl_tests_image/unclipped", unclipped),
        classic("mpl_tests_image/respects_bbox_before", |fig| {
            respects_bbox(fig, false)
        }),
        classic("mpl_tests_image/respects_bbox_after", |fig| {
            respects_bbox(fig, true)
        }),
        classic("mpl_tests_image/image_preserve_size2", image_preserve_size2).sized(7.0, 7.0, 1.0),
        classic("mpl_tests_image/minimized_rasterized", minimized_rasterized),
        classic(
            "mpl_tests_image/rgba_stage_demultiplies_resampled_alpha",
            rgba_stage_demultiplies_resampled_alpha,
        ),
    ]
}

/// `test_image_edges`: integers repeating every 12 values in grey, zoomed
/// so that the 1 in figure shows only their middle.
fn image_edges(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_frame_on(false);
    let values = Grid::from_fn(20, 9, |r, c| ((r * 9 + c) % 12) as i64);
    ax.imshow(
        plt::image::ScalarField::integers(&values),
        ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            origin: Some(Origin::Upper),
            extent: Some(Extent::new(-10.0, 10.0, -10.0, 10.0)),
            interpolation: Some(Interpolation::None),
            ..ImshowOptions::default()
        },
    )?;
    ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    Ok(())
}

/// `ax.imshow(data, aspect="auto", extent=extent)` with the given colormap.
fn stretched(
    ax: &Axes,
    data: Grid<f64>,
    extent: Extent,
    cmap: Option<&str>,
) -> Result<plt::image::AxesImage, Box<dyn std::error::Error>> {
    let colors = match cmap {
        Some(name) => ColorizerOptions::default().cmap(colormaps::get(name)?),
        None => ColorizerOptions::default(),
    };
    Ok(ax.imshow(
        data,
        ImshowOptions {
            colors,
            aspect: Some(Aspect::Auto),
            extent: Some(extent),
            ..ImshowOptions::default()
        },
    )?)
}

/// `test_unclipped`: an unclipped black image reaching far beyond the
/// axes.
fn unclipped(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let image = stretched(
        &ax,
        Grid::filled(2, 2, 0.0),
        Extent::new(-10.0, 10.0, -10.0, 10.0),
        Some("gray"),
    )?;
    image.set_clip_on(false);
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

/// `test_respects_bbox`: an image of the lower axes clipped to the box of
/// the upper one only, before or after the lower axes is panned.
fn respects_bbox(fig: &Figure, panned: bool) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    top.set_axis_off();
    bottom.set_axis_off();
    let data = Grid::from_vec(2, 2, vec![0.0, 1.0, 2.0, 3.0])?;
    let image = stretched(bottom, data, Extent::new(0.0, 1.0, 0.0, 1.0), None)?;
    ArtistHandle::set_clip_path(&image, None);
    image.set_clip_box(Some(top.id()));
    if panned {
        bottom.set_ylim(-1.0, 0.0);
    }
    Ok(())
}

/// `test_image_preserve_size2`: the 7 × 7 identity, nearest-sampled over a
/// 7 in figure saved at 1 dpi.
fn image_preserve_size2(fig: &Figure) -> CaseResult {
    fig.set_frameon(false);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_axis_off();
    let identity = Grid::from_fn(7, 7, |r, c| if r == c { 1.0 } else { 0.0 });
    ax.imshow(
        identity,
        ImshowOptions {
            interpolation: Some(Interpolation::Nearest),
            origin: Some(Origin::Lower),
            aspect: Some(Aspect::Auto),
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}

/// `test_minimized_rasterized`: a colorbar beside each of two meshes.
fn minimized_rasterized(fig: &Figure) -> CaseResult {
    let values = SpecRng::new(0).uniform(100, 0.0, 1.0);
    let data = Grid::from_vec(10, 10, values)?;
    for ax in fig.subplots(1, 2)?.as_slice() {
        let mesh = ax.pcolormesh(Mesh::new(data.clone()), PcolorOptions::default())?;
        fig.colorbar(
            &mesh,
            ColorbarOptions {
                axes: ColorbarAxes::Parent(ax.clone()),
                ..ColorbarOptions::default()
            },
        )?;
    }
    Ok(())
}

/// `test_rgba_stage_demultiplies_resampled_alpha`: one flat RGBA image per
/// alpha, resampled in the `rgba` stage.
fn rgba_stage_demultiplies_resampled_alpha(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 5)?;
    for (ax, alpha) in axs.as_slice().iter().zip([0.0, 0.25, 0.5, 0.75, 1.0]) {
        let data: ImageData = Grid::filled(4, 4, [0.2, 0.4, 0.6, alpha]).into();
        ax.imshow(
            data,
            ImshowOptions {
                interpolation: Some(Interpolation::Nearest),
                interpolation_stage: Some(InterpolationStage::Rgba),
                ..ImshowOptions::default()
            },
        )?;
    }
    Ok(())
}
