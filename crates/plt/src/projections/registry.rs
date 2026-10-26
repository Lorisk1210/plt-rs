//! The projections registered by name on this thread (matplotlib's
//! `register_projection`, `get_projection_names` and
//! `get_projection_class`).

use std::any::{Any, type_name};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::spec::AnyOptions;
use super::{Projection, ProjectionError, ProjectionOptions};
use crate::axes::ProjectionKind;

/// Makes the projection of one new axes from the options given, `None`
/// when they are not of the type the projection takes.
type Factory = Rc<dyn Fn(Option<&dyn Any>) -> Option<Box<dyn Projection>>>;

/// A projection registered by name.
#[derive(Clone)]
struct Registration {
    /// Makes the projection of a new axes.
    factory: Factory,
    /// Whether options are of the type the projection takes.
    accepts: fn(&dyn Any) -> bool,
    /// The name of that type.
    options: &'static str,
}

/// The name of 3-D axes, which are not drawn through a projection of
/// 2-D axes but take the name all the same.
const AXES_3D: &str = "3d";

thread_local! {
    /// The projections registered at run time, on this thread, by name.
    static REGISTERED: RefCell<BTreeMap<&'static str, Registration>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Registers the projection `factory` makes under its
/// [`name`](Projection::name), so that axes can be made with it by name on
/// this thread (matplotlib's `register_projection`). `factory` is called
/// once for every axes made, and once now for the name; registering a name
/// again replaces the projection, as matplotlib does. Such a projection
/// takes no options; [`register_with`] registers one that does.
///
/// ```
/// use plt::axes::ProjectionKind;
/// use plt::projections::{self, Projection, ProjectionView};
/// use plt::transforms::ResolvedTransform;
///
/// #[derive(Debug, Default)]
/// struct Plain;
///
/// impl Projection for Plain {
///     fn name(&self) -> &'static str {
///         "plain"
///     }
///     fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
///         view.rectilinear()
///     }
/// }
///
/// assert_eq!(projections::register(Plain::default)?, ProjectionKind::Registered("plain"));
/// assert_eq!(ProjectionKind::from_name("plain"), Some(ProjectionKind::Registered("plain")));
/// assert!(projections::projection_names().contains(&"plain"));
/// # Ok::<(), projections::ProjectionError>(())
/// ```
///
/// # Errors
///
/// [`ProjectionError::Builtin`] when the projection has the name of a
/// built-in one.
pub fn register<P: Projection>(
    factory: impl Fn() -> P + 'static,
) -> Result<ProjectionKind, ProjectionError> {
    register_with(move |&(): &()| factory())
}

/// Registers the projection `factory` makes from options of type `O`
/// under its [`name`](Projection::name) (matplotlib's
/// `register_projection` of a class whose `__init__` takes keywords).
/// Axes made with [`ProjectionKind::with_options`] hand their options to
/// `factory`; those made without get `O::default()`, which also gives the
/// name now. Otherwise as [`register`].
///
/// ```
/// use plt::axes::ProjectionKind;
/// use plt::figure::Figure;
/// use plt::projections::{self, Projection, ProjectionError, ProjectionView};
/// use plt::transforms::{Affine2D, ResolvedTransform};
///
/// /// Rectilinear axes leaning `degrees` to the right.
/// #[derive(Clone, Debug)]
/// struct Leaning {
///     degrees: f64,
/// }
///
/// impl Projection for Leaning {
///     fn name(&self) -> &'static str {
///         "leaning"
///     }
///     fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
///         let lean = Affine2D::identity().skew_deg(self.degrees, 0.0);
///         view.rectilinear().then(ResolvedTransform::affine(lean))
///     }
/// }
///
/// /// The keywords `Leaning` is made with.
/// #[derive(Debug, PartialEq)]
/// struct Lean(f64);
///
/// impl Default for Lean {
///     fn default() -> Self {
///         Self(30.0)
///     }
/// }
///
/// let leaning = projections::register_with(|lean: &Lean| Leaning { degrees: lean.0 })?;
/// let fig = Figure::new();
/// let steep = fig.add_subplot_with_projection(1, 2, 1, leaning.with_options(Lean(60.0)))?;
/// let usual = fig.add_subplot_with_projection(1, 2, 2, leaning)?;
/// assert_eq!(steep.with_projection(|p: &Leaning| p.degrees), Some(60.0));
/// assert_eq!(usual.with_projection(|p: &Leaning| p.degrees), Some(30.0));
/// assert!(matches!(
///     fig.subplot_with_projection(leaning.with_options(60.0)),
///     Err(ProjectionError::Options { .. })
/// ));
/// # Ok::<(), ProjectionError>(())
/// ```
///
/// # Errors
///
/// [`ProjectionError::Builtin`] when the projection has the name of a
/// built-in one.
pub fn register_with<O, P>(
    factory: impl Fn(&O) -> P + 'static,
) -> Result<ProjectionKind, ProjectionError>
where
    O: ProjectionOptions + Default,
    P: Projection,
{
    let name = factory(&O::default()).name();
    if is_builtin(name) {
        return Err(ProjectionError::Builtin(name));
    }
    let factory: Factory = Rc::new(move |options: Option<&dyn Any>| {
        let projection = match options {
            None => factory(&O::default()),
            Some(options) => factory(options.downcast_ref::<O>()?),
        };
        Some(Box::new(projection))
    });
    let registration = Registration {
        factory,
        accepts: |options| options.is::<O>(),
        options: type_name::<O>(),
    };
    REGISTERED.with_borrow_mut(|registered| registered.insert(name, registration));
    Ok(ProjectionKind::Registered(name))
}

/// The names of the built-in projections and of those registered on this
/// thread, sorted (matplotlib's `get_projection_names`).
pub fn projection_names() -> Vec<&'static str> {
    let builtin = ProjectionKind::BUILTIN
        .into_iter()
        .map(ProjectionKind::name)
        .chain([AXES_3D]);
    let mut names: Vec<&'static str> =
        REGISTERED.with_borrow(|registered| builtin.chain(registered.keys().copied()).collect());
    names.sort_unstable();
    names
}

/// The name a projection is registered under on this thread, if one is
/// registered as `name`.
pub(crate) fn registered_name(name: &str) -> Option<&'static str> {
    REGISTERED.with_borrow(|registered| registered.get_key_value(name).map(|(&name, _)| name))
}

/// The projection registered as `name`, if it takes `options`.
fn registration(
    name: &'static str,
    options: Option<&dyn AnyOptions>,
) -> Result<Registration, ProjectionError> {
    let registration = REGISTERED
        .with_borrow(|registered| registered.get(name).cloned())
        .ok_or_else(|| ProjectionError::Unknown(name.to_owned()))?;
    match options {
        Some(options) if !(registration.accepts)(as_any(options)) => {
            Err(ProjectionError::Options {
                projection: name,
                expected: registration.options,
                given: options.type_name(),
            })
        }
        _ => Ok(registration),
    }
}

/// Checks that a projection is registered as `name` and takes `options`.
pub(crate) fn check(
    name: &'static str,
    options: Option<&dyn AnyOptions>,
) -> Result<(), ProjectionError> {
    registration(name, options).map(drop)
}

/// A projection for new axes, made from `options` by the factory
/// registered as `name`.
pub(crate) fn create(
    name: &'static str,
    options: Option<&dyn AnyOptions>,
) -> Result<Box<dyn Projection>, ProjectionError> {
    let registration = registration(name, options)?;
    Ok((registration.factory)(options.map(as_any))
        .expect("the registration accepts the options it was checked against"))
}

/// `options` as a value of any type.
fn as_any(options: &dyn AnyOptions) -> &dyn Any {
    options
}

/// Whether `name` is the name of a built-in projection.
fn is_builtin(name: &str) -> bool {
    name == AXES_3D
        || ProjectionKind::BUILTIN
            .into_iter()
            .any(|kind| kind.name() == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::ProjectionView;
    use crate::transforms::ResolvedTransform;

    #[derive(Debug)]
    struct Named(&'static str);

    impl Projection for Named {
        fn name(&self) -> &'static str {
            self.0
        }
        fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
            view.rectilinear()
        }
    }

    #[test]
    fn builtin_names_are_refused() {
        for name in ["polar", "hammer", "3d", "rectilinear"] {
            assert_eq!(
                register(move || Named(name)),
                Err(ProjectionError::Builtin(name))
            );
        }
        assert!(projection_names().contains(&"3d"));
    }

    #[test]
    fn registered_projections_are_found_by_name() {
        assert!(matches!(
            create("tilted", None),
            Err(ProjectionError::Unknown(_))
        ));
        assert_eq!(registered_name("tilted"), None);
        let kind = register(|| Named("tilted")).unwrap();
        assert_eq!(kind.name(), "tilted");
        assert_eq!(create("tilted", None).unwrap().name(), "tilted");
        assert_eq!(registered_name("tilted"), Some("tilted"));
        let names = projection_names();
        assert!(names.is_sorted());
        assert!(names.contains(&"tilted") && names.contains(&"mollweide"));
        // Registering again replaces the factory.
        assert!(register(|| Named("tilted")).is_ok());
        assert_eq!(
            projection_names()
                .iter()
                .filter(|&&n| n == "tilted")
                .count(),
            1
        );
    }

    #[test]
    fn factories_receive_options_of_their_type() {
        /// A projection keeping the weight it is made with.
        #[derive(Debug)]
        struct Weighted(u8);

        impl Projection for Weighted {
            fn name(&self) -> &'static str {
                "weighted"
            }
            fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
                view.rectilinear()
            }
        }

        let weight = |projection: Box<dyn Projection>| {
            let projection: Box<dyn Any> = projection;
            projection.downcast::<Weighted>().map(|weighted| weighted.0)
        };
        register_with(|&weight: &u8| Weighted(weight)).unwrap();
        assert_eq!(weight(create("weighted", None).unwrap()).ok(), Some(0));
        assert_eq!(
            weight(create("weighted", Some(&7_u8)).unwrap()).ok(),
            Some(7)
        );
        assert_eq!(
            check("weighted", Some(&2.0)),
            Err(ProjectionError::Options {
                projection: "weighted",
                expected: "u8",
                given: "f64",
            })
        );
        register(|| Named("plainly")).unwrap();
        assert_eq!(check("plainly", Some(&())), Ok(()));
        assert!(matches!(
            check("plainly", Some(&7_u8)),
            Err(ProjectionError::Options { expected: "()", .. })
        ));
    }

    #[test]
    fn registrations_stay_on_their_thread() {
        register(|| Named("threadbound")).unwrap();
        let elsewhere = std::thread::spawn(|| registered_name("threadbound"))
            .join()
            .unwrap();
        assert_eq!(elsewhere, None);
    }
}
