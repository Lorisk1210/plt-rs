//! The projections registered by name on this thread (matplotlib's
//! `register_projection`, `get_projection_names` and
//! `get_projection_class`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::{Projection, ProjectionError};
use crate::axes::ProjectionKind;

/// Makes the projection of one new axes.
type Factory = Rc<dyn Fn() -> Box<dyn Projection>>;

/// The name of 3-D axes, which are not drawn through a projection of
/// 2-D axes but take the name all the same.
const AXES_3D: &str = "3d";

thread_local! {
    /// The projections registered at run time, on this thread, by name.
    static REGISTERED: RefCell<BTreeMap<&'static str, Factory>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Registers the projection `factory` makes under its
/// [`name`](Projection::name), so that axes can be made with it by name on
/// this thread (matplotlib's `register_projection`). `factory` is called
/// once for every axes made, and once now for the name; registering a name
/// again replaces the projection, as matplotlib does.
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
    let name = factory().name();
    if is_builtin(name) {
        return Err(ProjectionError::Builtin(name));
    }
    let factory: Factory = Rc::new(move || Box::new(factory()));
    REGISTERED.with_borrow_mut(|registered| registered.insert(name, factory));
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

/// A projection for new axes, made by the factory registered as `name`.
pub(crate) fn create(name: &str) -> Result<Box<dyn Projection>, ProjectionError> {
    let factory = REGISTERED
        .with_borrow(|registered| registered.get(name).cloned())
        .ok_or_else(|| ProjectionError::Unknown(name.to_owned()))?;
    Ok(factory())
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
        assert!(matches!(create("tilted"), Err(ProjectionError::Unknown(_))));
        assert_eq!(registered_name("tilted"), None);
        let kind = register(|| Named("tilted")).unwrap();
        assert_eq!(kind.name(), "tilted");
        assert_eq!(create("tilted").unwrap().name(), "tilted");
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
    fn registrations_stay_on_their_thread() {
        register(|| Named("threadbound")).unwrap();
        let elsewhere = std::thread::spawn(|| registered_name("threadbound"))
            .join()
            .unwrap();
        assert_eq!(elsewhere, None);
    }
}
