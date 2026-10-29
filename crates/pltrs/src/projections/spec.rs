//! A projection to make axes with, and the options a registered one is
//! made with (matplotlib's `projection=` keyword and the keywords that
//! reach the projection's `__init__`).

use std::any::{Any, type_name};
use std::fmt;
use std::rc::Rc;

use super::{ProjectionError, registry};
use crate::axes::ProjectionKind;

/// What options a registered projection is made with can be: any value
/// that can be printed and compared, such as a struct of settings
/// (matplotlib's keywords of `add_subplot(projection=..., **kwargs)`).
/// Every such type implements it.
pub trait ProjectionOptions: Any + fmt::Debug + PartialEq {}

impl<T: Any + fmt::Debug + PartialEq> ProjectionOptions for T {}

/// [`ProjectionOptions`] behind a pointer: compared by value and named
/// by type.
pub(crate) trait AnyOptions: Any + fmt::Debug {
    /// Whether `other` holds equal options of the same type.
    fn same_as(&self, other: &dyn AnyOptions) -> bool;

    /// The name of the type of the options.
    fn type_name(&self) -> &'static str;
}

impl<T: ProjectionOptions> AnyOptions for T {
    fn same_as(&self, other: &dyn AnyOptions) -> bool {
        let other: &dyn Any = other;
        other.downcast_ref::<T>() == Some(self)
    }

    fn type_name(&self) -> &'static str {
        type_name::<T>()
    }
}

/// The options a projection is made with.
pub(crate) type SharedOptions = Rc<dyn AnyOptions>;

/// A projection to make axes with: its kind and, for a registered
/// projection, the options its factory receives (matplotlib's
/// `projection=name` with the keywords that reach the projection's
/// `__init__`).
///
/// The constructors taking a projection take `impl Into<ProjectionSpec>`,
/// so a [`ProjectionKind`] alone will do; [`ProjectionKind::with_options`]
/// adds options. Two specifications are equal when their kinds are and
/// their options are equal values of the same type.
///
/// ```
/// use pltrs::axes::ProjectionKind;
/// use pltrs::projections::ProjectionSpec;
/// let plain = ProjectionSpec::from(ProjectionKind::Registered("leaning"));
/// let steep = ProjectionKind::Registered("leaning").with_options(45.0);
/// assert_eq!(steep.kind(), plain.kind());
/// assert_ne!(steep, plain);
/// assert_eq!(steep, ProjectionKind::Registered("leaning").with_options(45.0));
/// assert_eq!(ProjectionSpec::default(), ProjectionKind::Rectilinear.into());
/// ```
#[derive(Clone, Debug, Default)]
pub struct ProjectionSpec {
    kind: ProjectionKind,
    options: Option<SharedOptions>,
}

impl ProjectionSpec {
    /// The kind of projection.
    pub fn kind(&self) -> ProjectionKind {
        self.kind
    }

    /// A specification of `kind` with `options`, if any.
    pub(crate) fn new(kind: ProjectionKind, options: Option<SharedOptions>) -> Self {
        Self { kind, options }
    }

    /// The options the projection is made with, if any were given.
    pub(crate) fn options(&self) -> Option<&SharedOptions> {
        self.options.as_ref()
    }

    /// Checks that axes can be made with this projection: a registered
    /// one is registered on this thread and takes options of the type
    /// given, a built-in one takes none.
    pub(crate) fn check(&self) -> Result<(), ProjectionError> {
        match (self.kind, &self.options) {
            (ProjectionKind::Registered(name), options) => {
                registry::check(name, options.as_deref())
            }
            (_, None) => Ok(()),
            (kind, Some(options)) => Err(ProjectionError::Options {
                projection: kind.name(),
                expected: type_name::<()>(),
                given: options.type_name(),
            }),
        }
    }
}

impl From<ProjectionKind> for ProjectionSpec {
    fn from(kind: ProjectionKind) -> Self {
        Self {
            kind,
            options: None,
        }
    }
}

impl PartialEq for ProjectionSpec {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && match (&self.options, &other.options) {
                (None, None) => true,
                (Some(mine), Some(theirs)) => mine.same_as(&**theirs),
                _ => false,
            }
    }
}

impl ProjectionKind {
    /// This projection made with `options`, which the factory of a
    /// projection registered with [`register_with`](super::register_with)
    /// receives (matplotlib's `add_subplot(projection=name, **options)`).
    /// Making axes fails when the projection takes options of another
    /// type, or none.
    pub fn with_options<O: ProjectionOptions>(self, options: O) -> ProjectionSpec {
        ProjectionSpec {
            kind: self,
            options: Some(Rc::new(options)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_compare_by_value_and_type() {
        let kind = ProjectionKind::Registered("compared");
        assert_eq!(kind.with_options(1.5), kind.with_options(1.5));
        assert_ne!(kind.with_options(1.5), kind.with_options(2.5));
        assert_ne!(kind.with_options(1_i32), kind.with_options(1_i64));
        assert_ne!(kind.with_options(()), ProjectionSpec::from(kind));
        assert_ne!(
            ProjectionKind::Polar.with_options(1.5),
            kind.with_options(1.5)
        );
        assert!(format!("{:?}", kind.with_options(1.5)).contains("1.5"));
    }

    #[test]
    fn built_in_projections_take_no_options() {
        assert_eq!(ProjectionSpec::from(ProjectionKind::Hammer).check(), Ok(()));
        assert_eq!(
            ProjectionKind::Polar.with_options(0.5_f64).check(),
            Err(ProjectionError::Options {
                projection: "polar",
                expected: "()",
                given: "f64",
            })
        );
    }
}
