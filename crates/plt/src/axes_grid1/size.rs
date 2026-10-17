//! The sizes a divider splits its rectangle by (matplotlib's
//! `axes_grid1.axes_size`): each has a relative part, scaled to fill the
//! rectangle, and an absolute part in inches.

use crate::artist::ArtistId;
use crate::axes::AxesState;
use crate::figure::FigureState;
use crate::ticker::AxisName;

/// What the data extent of an axes is multiplied by in [`Size::AxesX`]
/// and [`Size::AxesY`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeAspect {
    /// A constant factor (matplotlib's numeric `aspect`).
    Factor(f64),
    /// The aspect of a reference axes relative to the measured axes'
    /// (matplotlib's `aspect="axes"` with `ref_ax`).
    RelativeTo(ArtistId),
}

/// A width or height in a divider (matplotlib's `axes_size` classes).
#[derive(Clone, Debug, PartialEq)]
pub enum Size {
    /// An absolute size in inches (`Size.Fixed`).
    Fixed(f64),
    /// A relative size (`Size.Scaled`).
    Scaled(f64),
    /// The width of an axes' view limits times an aspect, as a relative
    /// size (`Size.AxesX`).
    AxesX(ArtistId, SizeAspect),
    /// The height of an axes' view limits times an aspect, as a relative
    /// size (`Size.AxesY`).
    AxesY(ArtistId, SizeAspect),
    /// A fraction of another size (`Size.Fraction`).
    Fraction(f64, Box<Size>),
}

impl Size {
    /// The width of `axes`' view limits (`Size.AxesX(axes)`).
    pub fn axes_x(axes: ArtistId) -> Self {
        Self::AxesX(axes, SizeAspect::Factor(1.0))
    }

    /// The height of `axes`' view limits (`Size.AxesY(axes)`).
    pub fn axes_y(axes: ArtistId) -> Self {
        Self::AxesY(axes, SizeAspect::Factor(1.0))
    }

    /// `fraction` of `self` (matplotlib's `fraction * size`).
    #[must_use]
    pub fn times(self, fraction: f64) -> Self {
        Self::Fraction(fraction, Box::new(self))
    }

    /// `(relative, absolute)` parts (matplotlib's `get_size`); view limits
    /// are brought up to date first.
    pub(crate) fn evaluate(&self, fig: &mut FigureState) -> (f64, f64) {
        match self {
            Self::Fixed(inches) => (0.0, *inches),
            Self::Scaled(scale) => (*scale, 0.0),
            Self::AxesX(axes, aspect) => (data_extent(fig, *axes, AxisName::X, *aspect), 0.0),
            Self::AxesY(axes, aspect) => (data_extent(fig, *axes, AxisName::Y, *aspect), 0.0),
            Self::Fraction(fraction, size) => {
                let (relative, absolute) = size.evaluate(fig);
                (relative * fraction, absolute * fraction)
            }
        }
    }
}

/// How long a size given as a number or a percentage is (matplotlib's
/// `Size.from_any`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeSpec {
    /// Inches.
    Inches(f64),
    /// A percentage of a reference size.
    Percent(f64),
}

impl SizeSpec {
    /// The size, with percentages taken of `reference`.
    pub(crate) fn resolve(self, reference: &Size) -> Size {
        match self {
            Self::Inches(inches) => Size::Fixed(inches),
            Self::Percent(percent) => reference.clone().times(percent / 100.0),
        }
    }
}

impl From<f64> for SizeSpec {
    fn from(inches: f64) -> Self {
        Self::Inches(inches)
    }
}

/// The extent of `axes`' view limits along `name` times `aspect`, zero for
/// a removed axes (matplotlib's `AxesX.get_size` and `AxesY.get_size`).
fn data_extent(fig: &mut FigureState, axes: ArtistId, name: AxisName, aspect: SizeAspect) -> f64 {
    fig.unstale_axes(axes);
    let axes_aspect = |fig: &FigureState, id: ArtistId| {
        fig.store
            .get_as::<AxesState>(id)
            .and_then(|state| state.aspect.ratio())
            .unwrap_or(1.0)
    };
    let factor = match (aspect, name) {
        (SizeAspect::Factor(factor), _) => factor,
        (SizeAspect::RelativeTo(reference), AxisName::X) => {
            axes_aspect(fig, reference) / axes_aspect(fig, axes)
        }
        // matplotlib's `AxesY` measures the reference but then uses the
        // measured axes' own aspect.
        (SizeAspect::RelativeTo(_), AxisName::Y) => axes_aspect(fig, axes),
    };
    fig.store.get_as::<AxesState>(axes).map_or(0.0, |state| {
        let (low, high) = state.view_interval(name);
        (high - low).abs() * factor
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::rc::Aspect;

    #[test]
    fn sizes_combine_relative_and_absolute_parts() {
        let fig = Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        ax.set_xlim(0.0, 20.0).set_ylim(5.0, -5.0);
        let mut state = fig.raw().figure_state().borrow_mut();
        assert_eq!(Size::Fixed(0.5).evaluate(&mut state), (0.0, 0.5));
        assert_eq!(Size::axes_x(ax.id()).evaluate(&mut state), (20.0, 0.0));
        assert_eq!(Size::axes_y(ax.id()).evaluate(&mut state), (10.0, 0.0));
        let five_percent = SizeSpec::Percent(5.0).resolve(&Size::axes_x(ax.id()));
        assert_eq!(five_percent.evaluate(&mut state), (1.0, 0.0));
        assert_eq!(
            SizeSpec::from(0.1).resolve(&Size::Scaled(1.0)),
            Size::Fixed(0.1)
        );
    }

    #[test]
    fn relative_aspects_follow_matplotlib() {
        let fig = Figure::new();
        let reference = fig.add_axes([0.1, 0.1, 0.4, 0.8]);
        let measured = fig.add_axes([0.5, 0.1, 0.4, 0.8]);
        measured.set_xlim(0.0, 4.0).set_ylim(0.0, 4.0);
        reference.set_aspect(Aspect::Ratio(2.0)).unwrap();
        measured.set_aspect(Aspect::Ratio(4.0)).unwrap();
        let mut state = fig.raw().figure_state().borrow_mut();
        let relative = SizeAspect::RelativeTo(reference.id());
        assert_eq!(
            Size::AxesX(measured.id(), relative).evaluate(&mut state).0,
            2.0
        );
        assert_eq!(
            Size::AxesY(measured.id(), relative).evaluate(&mut state).0,
            16.0
        );
    }
}
