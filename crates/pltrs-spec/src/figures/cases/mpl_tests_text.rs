//! `figures/cases_mpl_tests_text.py`: matplotlib's non-image tests of
//! `test_text.py` and `test_mathtext.py` that only check that a figure
//! draws, in the test suite's default style.

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_mathtext_cmr10_minus_sign`: mathtext tick labels in cmr10,
        // which has no minus sign of its own.
        classic("mpl_tests_text/mathtext_cmr10_minus_sign", |fig| {
            fig.subplot().plot([-1.0, 0.0], [-1.0, 0.0])?;
            Ok(())
        })
        .with_rc_setup(|rc| {
            rc.set("font.family", "cmr10")?;
            Ok(rc.set("axes.formatter.use_mathtext", "True")?)
        }),
    ]
}
