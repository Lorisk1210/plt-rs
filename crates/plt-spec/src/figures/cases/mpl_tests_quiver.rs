//! `figures/cases_mpl_tests_quiver.py`: matplotlib's non-image tests of
//! `test_quiver.py` that draw a figure and check what it holds, in the
//! test suite's default style: quiver keys of arrows oriented by angle
//! arrays, `xy` angles or `xy` scale units (one arrow each, also on an
//! Aitoff map), and components replaced with `set_UVC`.

use std::f64::consts::PI;

use plt::axes::{Axes, PlotError};
use plt::geo::GeoProjection;
use plt::numeric::linspace;
use plt::prelude::*;
use plt::quiver::{
    Angles, LabelPosition, Quiver, QuiverKeyOptions, QuiverOptions, QuiverUnits, VectorField,
};

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    use KeyKeywords::{AnglesXy, Both, ScaleUnitsXy};
    vec![
        classic("mpl_tests_quiver/quiverkey_angles", quiverkey_angles),
        classic(
            "mpl_tests_quiver/quiverkey_angles_scale_units_cartesian_angles_xy",
            |fig| cartesian(fig, AnglesXy),
        ),
        classic(
            "mpl_tests_quiver/quiverkey_angles_scale_units_cartesian_angles_xy_scale_units_xy",
            |fig| cartesian(fig, Both),
        ),
        classic(
            "mpl_tests_quiver/quiverkey_angles_scale_units_cartesian_scale_units_xy",
            |fig| cartesian(fig, ScaleUnitsXy),
        ),
        classic(
            "mpl_tests_quiver/quiverkey_angles_xy_aitoff_angles_xy",
            |fig| aitoff(fig, AnglesXy),
        ),
        classic(
            "mpl_tests_quiver/quiverkey_angles_xy_aitoff_angles_xy_scale_units_xy",
            |fig| aitoff(fig, Both),
        ),
        classic(
            "mpl_tests_quiver/quiverkey_angles_xy_aitoff_scale_units_xy",
            |fig| aitoff(fig, ScaleUnitsXy),
        ),
        classic("mpl_tests_quiver/quiver_setuvc_numbers", setuvc_numbers),
    ]
}

/// The keyword sets the key tests loop over.
#[derive(Clone, Copy)]
enum KeyKeywords {
    /// `angles="xy"`.
    AnglesXy,
    /// `angles="xy", scale_units="xy"`.
    Both,
    /// `scale_units="xy"`.
    ScaleUnitsXy,
}

impl KeyKeywords {
    /// The quiver options of the keyword set.
    fn options(self) -> QuiverOptions {
        let (angles, scale_units) = match self {
            Self::AnglesXy => (Angles::Xy, None),
            Self::Both => (Angles::Xy, Some(QuiverUnits::Xy)),
            Self::ScaleUnitsXy => (Angles::Uv, Some(QuiverUnits::Xy)),
        };
        QuiverOptions {
            angles,
            scale_units,
            ..QuiverOptions::default()
        }
    }
}

/// The quiver of four unit vectors at the corners of the unit square.
fn unit_square(ax: &Axes, options: QuiverOptions) -> Result<Quiver, PlotError> {
    let field = VectorField::new(
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0; 4],
        [1.0; 4],
    );
    ax.quiver(field, options)
}

/// `test_quiverkey_angles`: unit vectors given an array of angles, and
/// their key.
fn quiverkey_angles(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = QuiverOptions {
        angles: Angles::Degrees(vec![1.0; 4]),
        ..QuiverOptions::default()
    };
    let q = unit_square(&ax, options)?;
    ax.quiverkey(&q, (1.0, 1.0), 2.0, "Label", QuiverKeyOptions::default());
    Ok(())
}

/// `test_quiverkey_angles_scale_units_cartesian`: three vectors and two
/// keys.
fn cartesian(fig: &Figure, keywords: KeyKeywords) -> CaseResult {
    let ax = fig.subplot();
    let field = VectorField::new(
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, -1.0, 1.0],
        [1.0, -1.0, 0.0],
    );
    let q = ax.quiver(field, keywords.options())?;
    let east = QuiverKeyOptions {
        labelpos: LabelPosition::East,
        ..QuiverKeyOptions::default()
    };
    ax.quiverkey(&q, (0.3, 1.1), 1.0, "Quiver key, length = 1", east);
    ax.quiverkey(&q, (0.0, 0.0), 1.0, "1 units", QuiverKeyOptions::default());
    Ok(())
}

/// `test_quiverkey_angles_xy_aitoff`: upward vectors along latitude 30°
/// on an Aitoff map, and a key.
fn aitoff(fig: &Figure, keywords: KeyKeywords) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Aitoff);
    let x = linspace(-PI, PI, 11);
    let n = x.len();
    let field = VectorField::new(x, vec![PI / 6.0; n], vec![0.0; n], vec![1.0; n]);
    let q = ax.quiver(field, keywords.options())?;
    ax.quiverkey(&q, (0.0, 0.0), 1.0, "1 units", QuiverKeyOptions::default());
    Ok(())
}

/// `test_quiver_setuvc_numbers`: unit vectors given the components 0 and
/// 1 for every arrow.
fn setuvc_numbers(fig: &Figure) -> CaseResult {
    let q = unit_square(&fig.subplot(), QuiverOptions::default())?;
    q.set_uvc(&[0.0], &[1.0])?;
    Ok(())
}
