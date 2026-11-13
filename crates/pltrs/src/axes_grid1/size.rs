//! The sizes a divider splits its rectangle by (matplotlib's
//! `axes_grid1.axes_size`): each has a relative part, scaled to fill the
//! rectangle, and an absolute part in inches.

use std::fmt;
use std::ops::Add;
use std::rc::Rc;

use pltrs_core::Bbox;
use pltrs_core::archive::{External, ProgramFn};

use crate::artist::{ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::figure::FigureState;
use crate::spines::SpineSide;
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

pltrs_core::archive_enum!(SizeAspect {
    Factor(v0),
    RelativeTo(v0),
});

/// A size the program computes (matplotlib's `axes_size` classes of the
/// program's, asked `get_size`): a function returning the `(relative,
/// absolute)` parts, called at every layout. It runs inside the layout
/// pass, which lends it the figure as laid out so far, so that it may
/// read it through the figure's handles as matplotlib's do. Clones share
/// the function; a figure with one is archived only if the function has
/// an origin ([`SizeFn::with_origin`]).
#[derive(Clone)]
pub struct SizeFn(ProgramFn<dyn Fn() -> (f64, f64)>);

pltrs_core::archive_struct!(SizeFn(v0));

impl SizeFn {
    /// A size computed by `size`.
    pub fn new(size: impl Fn() -> (f64, f64) + Send + 'static) -> Self {
        Self(ProgramFn::new(Rc::new(size)))
    }

    /// A size computed by `size`, archived as `origin`, what the program
    /// made it from (see [`ProgramFn`]).
    pub fn with_origin(size: impl Fn() -> (f64, f64) + Send + 'static, origin: External) -> Self {
        Self(ProgramFn::with_origin(Rc::new(size), origin))
    }

    /// The `(relative, absolute)` parts now.
    pub fn parts(&self) -> (f64, f64) {
        (self.0.get())()
    }
}

impl fmt::Debug for SizeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SizeFn").finish_non_exhaustive()
    }
}

impl PartialEq for SizeFn {
    /// Sizes are equal when they share the function.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self.0.get(), other.0.get())
    }
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
    /// The sum of sizes (`Size.Add` and `Size.AddList`, also made by
    /// `+`).
    Sum(Vec<Size>),
    /// The widest window extent of artists, as an absolute size
    /// (`Size.MaxWidth`).
    MaxWidth(Vec<ArtistId>),
    /// The tallest window extent of artists, as an absolute size
    /// (`Size.MaxHeight`).
    MaxHeight(Vec<ArtistId>),
    /// How far the decorations of axes (tick labels, axis labels, titles)
    /// reach past one side of them, the most of any of the axes, as an
    /// absolute size. Each axes is measured at its original position, so
    /// that its own divider can use the size (matplotlib's
    /// `_AxesDecorationsSize`, made by `add_auto_adjustable_area`).
    AxesDecorations(Vec<ArtistId>, SpineSide),
    /// A size the program computes (an `axes_size` class of the
    /// program's).
    Program(SizeFn),
}

pltrs_core::archive_enum!(Size {
    Fixed(v0),
    Scaled(v0),
    AxesX(v0, v1),
    AxesY(v0, v1),
    Fraction(v0, v1),
    Sum(v0),
    MaxWidth(v0),
    MaxHeight(v0),
    AxesDecorations(v0, v1),
    Program(v0),
});

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
            Self::Sum(sizes) => sizes
                .iter()
                .map(|size| size.evaluate(fig))
                .fold((0.0, 0.0), |(r, a), (sr, sa)| (r + sr, a + sa)),
            Self::MaxWidth(artists) => (0.0, max_extent(fig, artists, Bbox::width)),
            Self::MaxHeight(artists) => (0.0, max_extent(fig, artists, Bbox::height)),
            Self::AxesDecorations(axes, side) => (0.0, decorations_extent(fig, axes, *side)),
            // Lent the figure, which matplotlib's sizes read (mplhep's
            // `RemainderFixed` asks its divider the axes' position).
            Self::Program(size) => fig.lend(|_| size.parts()).unwrap_or_else(|| size.parts()),
        }
    }
}

impl Add for Size {
    type Output = Self;

    /// Both sizes side by side (matplotlib's `Size.Add`).
    fn add(self, other: Self) -> Self {
        Self::Sum(vec![self, other])
    }
}

impl Add<f64> for Size {
    type Output = Self;

    /// This size and `inches` more (matplotlib's `size + inches`).
    fn add(self, inches: f64) -> Self {
        self + Self::Fixed(inches)
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

pltrs_core::archive_enum!(SizeSpec {
    Inches(v0),
    Percent(v0),
});

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

/// The largest `measure` of the window extents of `artists`, in inches
/// (matplotlib's `MaxExtent.get_size`).
fn max_extent(fig: &FigureState, artists: &[ArtistId], measure: fn(&Bbox) -> f64) -> f64 {
    let layout = LayoutContext::new(fig);
    let largest = artists
        .iter()
        .filter_map(|&id| layout.store().get(id)?.window_extent(&layout))
        .map(|extent| measure(&extent))
        .fold(0.0, f64::max);
    largest / fig.dpi()
}

/// How far the tight box of any of `axes`, measured at its original
/// position, reaches past its `side`, in inches; invisible axes do not
/// count (matplotlib's `_AxesDecorationsSize.get_size`).
fn decorations_extent(fig: &mut FigureState, axes: &[ArtistId], side: SpineSide) -> f64 {
    let largest = axes
        .iter()
        .filter_map(|&id| {
            let (extent, tight) = fig.unlocated_extents(id)?;
            Some(match side {
                SpineSide::Left => extent.xmin() - tight.xmin(),
                SpineSide::Right => tight.xmax() - extent.xmax(),
                SpineSide::Bottom => extent.ymin() - tight.ymin(),
                SpineSide::Top => tight.ymax() - extent.ymax(),
            })
        })
        .fold(0.0, f64::max);
    largest / fig.dpi()
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
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 20.0)
            .and_then(|ax| ax.set_ylim(5.0, -5.0))
            .expect("finite limits");
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
        let program = Size::Program(SizeFn::new(|| (0.5, 0.25)));
        assert_eq!(program.clone().times(2.0).evaluate(&mut state), (1.0, 0.5));
        assert_eq!(program, program.clone(), "clones share the function");
        assert_ne!(program, Size::Program(SizeFn::new(|| (0.5, 0.25))));
    }

    #[test]
    fn program_sizes_read_the_figure_a_layout_works_on() {
        thread_local! {
            static AXES: std::cell::RefCell<Option<crate::axes::Axes>> =
                const { std::cell::RefCell::new(None) };
        }
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.2, 0.5, 0.4])
            .expect("a valid rectangle");
        AXES.set(Some(ax.clone()));
        // The axes' width in figure fractions, read through its handle.
        let program = Size::Program(SizeFn::new(|| {
            AXES.with_borrow(|ax| {
                let ax = ax.as_ref().expect("the axes is set");
                (ax.original_position().width(), 0.0)
            })
        }));
        let cell = std::rc::Rc::clone(fig.state_cell());
        let (relative, _) = cell.work_on(|state| program.evaluate(state));
        assert!((relative - 0.5).abs() < 1e-12, "{relative}");
        AXES.set(None);
    }

    #[test]
    fn sums_and_extents_are_absolute_sizes() {
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let short = fig.text(0.1, 0.1, "short");
        let long = fig.text(0.1, 0.5, "a much longer text\nover two lines");
        let (short_box, long_box) = (
            short.window_extent().unwrap(),
            long.window_extent().unwrap(),
        );
        let ids = vec![short.id(), long.id()];
        let mut state = fig.raw().figure_state().borrow_mut();
        assert_eq!(
            Size::MaxWidth(ids.clone()).evaluate(&mut state),
            (0.0, long_box.width() / 50.0)
        );
        assert_eq!(
            Size::MaxHeight(ids).evaluate(&mut state),
            (0.0, long_box.height().max(short_box.height()) / 50.0)
        );
        assert_eq!(Size::MaxWidth(Vec::new()).evaluate(&mut state), (0.0, 0.0));
        let sum = Size::Scaled(2.0) + Size::Fixed(0.5) + 0.25;
        assert_eq!(sum.evaluate(&mut state), (2.0, 0.75));
    }

    #[test]
    fn relative_aspects_follow_matplotlib() {
        let fig = Figure::new();
        let reference = fig
            .add_axes([0.1, 0.1, 0.4, 0.8])
            .expect("a valid rectangle");
        let measured = fig
            .add_axes([0.5, 0.1, 0.4, 0.8])
            .expect("a valid rectangle");
        measured
            .set_xlim(0.0, 4.0)
            .and_then(|ax| ax.set_ylim(0.0, 4.0))
            .expect("finite limits");
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
