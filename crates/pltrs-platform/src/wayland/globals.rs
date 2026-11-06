//! The compositor's globals this backend binds, and at which versions.

use std::collections::HashMap;

use super::connection::Connection;
use super::protocol::Interface;
use super::protocol::core;
use crate::error::Error;

/// The globals bound once: interface name, the highest version this
/// backend speaks, and the object kind.
const SINGLETONS: [(&str, u32, Interface); 11] = [
    // Version 6 adds `wl_surface.preferred_buffer_scale`.
    ("wl_compositor", 6, Interface::Compositor),
    ("wl_subcompositor", 1, Interface::Subcompositor),
    ("wl_shm", 1, Interface::Shm),
    // Version 5 adds `wm_capabilities`; 6 the `suspended` state.
    ("xdg_wm_base", 6, Interface::WmBase),
    ("wl_data_device_manager", 3, Interface::DataDeviceManager),
    (
        "zxdg_decoration_manager_v1",
        1,
        Interface::DecorationManager,
    ),
    ("wp_viewporter", 1, Interface::Viewporter),
    (
        "wp_fractional_scale_manager_v1",
        1,
        Interface::FractionalScaleManager,
    ),
    (
        "wp_cursor_shape_manager_v1",
        1,
        Interface::CursorShapeManager,
    ),
    ("xdg_activation_v1", 1, Interface::Activation),
    (
        "xdg_toplevel_icon_manager_v1",
        1,
        Interface::ToplevelIconManager,
    ),
];

/// The highest `wl_seat` version: 8 adds `axis_value120`.
pub(crate) const SEAT_VERSION: u32 = 8;
/// The highest `wl_output` version: 2 adds `scale`, 3 `release`.
pub(crate) const OUTPUT_VERSION: u32 = 4;

/// A bound global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bound {
    /// The object id.
    pub(crate) id: u32,
    /// The version bound.
    pub(crate) version: u32,
    /// The global's numeric name.
    pub(crate) name: u32,
}

/// The singleton globals bound so far.
#[derive(Debug, Default)]
pub(crate) struct Globals {
    bound: HashMap<Interface, Bound>,
}

impl Globals {
    /// The object id of a bound singleton.
    pub(crate) fn get(&self, interface: Interface) -> Option<u32> {
        self.bound.get(&interface).map(|bound| bound.id)
    }

    /// The object id of a singleton the backend cannot work without.
    pub(crate) fn require(&self, interface: Interface) -> Result<u32, Error> {
        self.get(interface)
            .ok_or(Error::Unsupported(match interface {
                Interface::Compositor => "the compositor offers no wl_compositor",
                Interface::Shm => "the compositor offers no wl_shm",
                _ => "the compositor offers no xdg_wm_base",
            }))
    }

    /// Binds the global `name` if it is a singleton this backend uses and
    /// is not bound yet; returns its kind and id.
    pub(crate) fn bind(
        &mut self,
        conn: &mut Connection,
        registry: u32,
        (name, interface, version): (u32, &str, u32),
    ) -> Result<Option<(Interface, u32)>, Error> {
        let Some(&(interface_name, ours, kind)) =
            SINGLETONS.iter().find(|(known, _, _)| *known == interface)
        else {
            return Ok(None);
        };
        if self.bound.contains_key(&kind) || version == 0 {
            return Ok(None);
        }
        let version = version.min(ours);
        let id = bind(conn, registry, kind, (name, interface_name, version))?;
        self.bound.insert(kind, Bound { id, version, name });
        Ok(Some((kind, id)))
    }

    /// Forgets a singleton whose global was removed; returns its kind.
    pub(crate) fn remove(&mut self, name: u32) -> Option<Interface> {
        let kind = self
            .bound
            .iter()
            .find(|(_, bound)| bound.name == name)
            .map(|(&kind, _)| kind)?;
        self.bound.remove(&kind);
        Some(kind)
    }
}

/// Binds global `name` of `interface` at `version` as a new object of
/// `kind`.
pub(crate) fn bind(
    conn: &mut Connection,
    registry: u32,
    kind: Interface,
    (name, interface, version): (u32, &str, u32),
) -> Result<u32, Error> {
    let id = conn.new_id(kind)?;
    conn.send(core::bind(registry, name, interface, version, id))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;

    use super::*;

    #[test]
    fn singletons_bind_once_at_the_lower_version() {
        let (ours, _compositor) = UnixStream::pair().unwrap();
        let mut conn = Connection::new(ours).unwrap();
        let mut globals = Globals::default();
        let bound = globals.bind(&mut conn, 2, (7, "wl_compositor", 4)).unwrap();
        let (kind, id) = bound.unwrap();
        assert_eq!(kind, Interface::Compositor);
        assert_eq!(globals.get(Interface::Compositor), Some(id));
        assert_eq!(globals.bound[&Interface::Compositor].version, 4);
        assert_eq!(
            globals.bind(&mut conn, 2, (8, "wl_compositor", 6)).unwrap(),
            None
        );
        globals.bind(&mut conn, 2, (9, "xdg_wm_base", 99)).unwrap();
        assert_eq!(globals.bound[&Interface::WmBase].version, 6);
        assert_eq!(
            globals.bind(&mut conn, 2, (10, "wl_unknown", 1)).unwrap(),
            None
        );
        assert_eq!(globals.bind(&mut conn, 2, (11, "wl_shm", 0)).unwrap(), None);
        assert!(globals.require(Interface::Shm).is_err());
        assert_eq!(globals.remove(9), Some(Interface::WmBase));
        assert_eq!(globals.get(Interface::WmBase), None);
        assert_eq!(globals.remove(9), None);
    }
}
