//! `figures/cases_mpl_3d_b.py`: matplotlib's own image tests from
//! `mplot3d/tests/test_axes3d.py` (from `test_quiver3d` to the end) and
//! `mplot3d/tests/test_legend3d.py` — every `@image_comparison` test and
//! the `fig_test` side of every `@check_figures_equal` test, by topic.
//!
//! Cases keep the test's style: the reference configuration for the
//! `mpl20`/`default` tests, the test suite's default `classic` plus
//! `_classic_test_patch` (8 × 6 in figures saved at 100 dpi) for the
//! `check_figures_equal` ones. `remove_text=True` (the default of
//! `mpl3d_image_comparison`) becomes [`remove_ticks_and_titles`] at the
//! end of the build.

mod axes;
mod collections;
mod legend;
mod plotting;
mod projections;
mod quiver;
mod scales;

use plt::lines::LineProps;
use plt::mplot3d::{Axes3D, Axes3DError, Axis3, Vec3};

use super::common::subplots_3d;
use super::mpl_tests::classic;
pub(super) use super::mpl_tests::remove_ticks_and_titles;
use crate::figures::FigureCase;

/// What the tests switch on with "Remove when image is regenerated".
const AUTOMARGIN: &[(&str, &str)] = &[("axes3d.automargin", "True")];

/// `itertools.product(a, b, c)`: every triple, the last element varying
/// fastest.
fn product3(a: &[f64], b: &[f64], c: &[f64]) -> Vec<Vec3> {
    let mut triples = Vec::with_capacity(a.len() * b.len() * c.len());
    for &x in a {
        for &y in b {
            triples.extend(c.iter().map(|&z| [x, y, z]));
        }
    }
    triples
}

/// Plots every edge of the cube `r³`, one line each, in the order of
/// `itertools.combinations(itertools.product(r, r, r), 2)`.
fn cube_edges(ax: &Axes3D, r: [f64; 2], props: &LineProps) -> Result<(), Axes3DError> {
    // The corners as indices into `r`; edges join corners one step apart.
    let corners: Vec<[usize; 3]> = (0..8).map(|i| [i >> 2, (i >> 1) & 1, i & 1]).collect();
    for (i, s) in corners.iter().enumerate() {
        for e in &corners[i + 1..] {
            if s.iter().zip(e).filter(|(a, b)| a != b).count() == 1 {
                let [x, y, z] = [0, 1, 2].map(|k| [r[s[k]], r[e[k]]]);
                ax.plot_with(x, y, z, Axis3::Z, props.clone())?;
            }
        }
    }
    Ok(())
}

/// The quiver tests.
const QUIVER: &[FigureCase] = &[
    FigureCase::new("mpl_3d_b/quiver3d", quiver::quiver3d),
    classic("mpl_3d_b/quiver3d_empty", quiver::quiver3d_empty),
    FigureCase::new("mpl_3d_b/quiver3d_masked", quiver::quiver3d_masked),
    FigureCase::new("mpl_3d_b/quiver3d_colorcoded", quiver::quiver3d_colorcoded),
    classic("mpl_3d_b/quiver3D_smoke", quiver::quiver3d_smoke),
];

/// The collection tests.
const COLLECTIONS: &[FigureCase] = &[
    FigureCase::new(
        "mpl_3d_b/poly3dcollection_closed",
        collections::poly3dcollection_closed,
    ),
    FigureCase::new(
        "mpl_3d_b/poly3dcollection_alpha",
        collections::poly3dcollection_alpha,
    ),
    FigureCase::new(
        "mpl_3d_b/add_collection3d_zs_array",
        collections::add_collection3d_zs_array,
    )
    .with_rc(AUTOMARGIN),
    FigureCase::new(
        "mpl_3d_b/add_collection3d_zs_scalar",
        collections::add_collection3d_zs_scalar,
    )
    .with_rc(AUTOMARGIN),
    FigureCase::new("mpl_3d_b/computed_zorder", collections::computed_zorder).with_rc(AUTOMARGIN),
];

/// The tests of the axes.
const AXES: &[FigureCase] = &[
    FigureCase::new("mpl_3d_b/axes3d_labelpad", axes::axes3d_labelpad),
    FigureCase::new("mpl_3d_b/axes3d_cla", axes::axes3d_cla),
    FigureCase::new("mpl_3d_b/axes3d_rotated", axes::axes3d_rotated),
    classic("mpl_3d_b/culling", axes::culling),
    FigureCase::new("mpl_3d_b/axes3d_focal_length", axes::axes3d_focal_length),
    FigureCase::new("mpl_3d_b/axes3d_ortho", axes::axes3d_ortho),
    FigureCase::new("mpl_3d_b/axes3d_isometric", axes::axes3d_isometric),
    classic("mpl_3d_b/inverted", axes::inverted),
    classic("mpl_3d_b/inverted_ref", axes::inverted_ref),
    classic("mpl_3d_b/ticklabel_format", axes::ticklabel_format),
    FigureCase::new("mpl_3d_b/minor_ticks", axes::minor_ticks),
    FigureCase::new("mpl_3d_b/equal_box_aspect", axes::equal_box_aspect),
    classic("mpl_3d_b/shared_view", axes::shared_view),
    FigureCase::new("mpl_3d_b/panecolor_rcparams", axes::panecolor_rcparams)
        .sized(1.0, 1.0, 100.0)
        .with_rc(&[
            ("axes3d.xaxis.panecolor", "r"),
            ("axes3d.yaxis.panecolor", "g"),
            ("axes3d.zaxis.panecolor", "b"),
        ]),
];

/// The projection matrix tests.
const PROJECTIONS: &[FigureCase] = &[
    FigureCase::new("mpl_3d_b/proj_axes_cube", projections::proj_axes_cube).with_rc(AUTOMARGIN),
    FigureCase::new(
        "mpl_3d_b/proj_axes_cube_ortho",
        projections::proj_axes_cube_ortho,
    )
    .with_rc(AUTOMARGIN),
];

/// The plotting tests.
const PLOTTING: &[FigureCase] = &[
    classic("mpl_3d_b/axlim_clip", plotting::axlim_clip),
    FigureCase::new(
        "mpl_3d_b/errorbar3d_errorevery",
        plotting::errorbar3d_errorevery,
    ),
    FigureCase::new("mpl_3d_b/errorbar3d", plotting::errorbar3d),
    FigureCase::new("mpl_3d_b/stem3d", plotting::stem3d).sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_3d_b/scalarmap_update", plotting::scalarmap_update),
    classic("mpl_3d_b/text_3d", plotting::text_3d),
    classic("mpl_3d_b/pathpatch_3d", plotting::pathpatch_3d),
    FigureCase::new("mpl_3d_b/arc_pathpatch", plotting::arc_pathpatch),
    FigureCase::new("mpl_3d_b/scatter_spiral", plotting::scatter_spiral).with_rc(AUTOMARGIN),
    classic(
        "mpl_3d_b/mutating_input_arrays_y_and_z",
        plotting::mutating_input_arrays_y_and_z,
    ),
    FigureCase::new(
        "mpl_3d_b/surface3d_zsort_inf",
        plotting::surface3d_zsort_inf,
    )
    .with_rc(AUTOMARGIN),
];

/// The scale tests.
const SCALES: &[FigureCase] = &[
    FigureCase::new("mpl_3d_b/scale3d_artists_log", scales::scale3d_artists_log)
        .sized(16.0, 12.0, 100.0),
    FigureCase::new("mpl_3d_b/scale3d_all_scales", scales::scale3d_all_scales)
        .sized(10.0, 6.0, 100.0),
    classic("mpl_3d_b/scale3d_all_clipped", scales::scale3d_all_clipped),
    FigureCase::new("mpl_3d_b/scale3d_log_bases", scales::scale3d_log_bases)
        .sized(10.0, 8.0, 100.0),
    FigureCase::new(
        "mpl_3d_b/scale3d_symlog_params",
        scales::scale3d_symlog_params,
    ),
];

/// The tests of `test_legend3d.py`.
const LEGEND: &[FigureCase] = &[
    FigureCase::new("mpl_3d_b/legend_plot", legend::legend_plot),
    FigureCase::new("mpl_3d_b/legend_bar", legend::legend_bar),
    FigureCase::new("mpl_3d_b/fancy", legend::fancy),
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        QUIVER,
        COLLECTIONS,
        AXES,
        PROJECTIONS,
        PLOTTING,
        SCALES,
        LEGEND,
    ]
    .concat()
}
