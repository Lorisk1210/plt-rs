//! `figures/cases_mpl_tests_patches.py`: matplotlib's non-image tests from
//! `test_patches.py` that only draw: cut stars edged in the eight
//! spellings of the named line styles, in the test suite's default style,
//! and an unfilled dashed rectangle without width, in the `mpl20` style.

use plt::patches::{PathShape, Rectangle};
use plt::prelude::*;
use plt_core::{Affine2D, Rgba};

use super::mpl_collections::cut_stars;
use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic(
            "mpl_tests_patches/patch_linestyle_accents",
            patch_linestyle_accents,
        ),
        FigureCase::new(
            "mpl_tests_patches/patch_zero_linewidth_dashed",
            patch_zero_linewidth_dashed,
        ),
    ]
}

/// `test_patch_linestyle_accents`: the line styles given by mnemonic and
/// by name, one cut star each, moved along the diagonal.
fn patch_linestyle_accents(fig: &Figure) -> CaseResult {
    let (star, _) = cut_stars()?;
    let ax = fig.subplot();
    let styles = ["-", "--", "-.", ":", "solid", "dashed", "dashdot", "dotted"];
    for (i, style) in (0_u32..).zip(styles) {
        let offset = f64::from(i);
        let moved = star.transformed(&Affine2D::identity().translate(offset, offset));
        ax.add_patch(PathShape(moved)).update_props(
            PatchProps::default()
                .linewidth(3.0)
                .linestyle(style.parse::<LineStyle>()?)
                .facecolor(Rgba::opaque(1.0, 0.0, 0.0))
                .edgecolor(Rgba::opaque(0.0, 0.0, 1.0)),
        );
    }
    let last = f64::from(u32::try_from(styles.len())?);
    ax.set_xlim(-1.0, last)?.set_ylim(-1.0, last)?;
    Ok(())
}

/// `test_patch_zero_linewidth_dashed_uses_solid_gc_dashes`: a dashed
/// outline of zero width, which draws nothing.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case builder returns a CaseResult"
)]
fn patch_zero_linewidth_dashed(fig: &Figure) -> CaseResult {
    fig.subplot()
        .add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0))
        .set_fill(false)
        .set_linewidth(0.0)
        .set_linestyle(LineStyle::Dashed);
    Ok(())
}
