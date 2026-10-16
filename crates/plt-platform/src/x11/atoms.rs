//! Atoms: predefined ones, and the ones interned once per connection.

use super::connection::Connection;
use super::reply;
use super::request;
use crate::error::Error;

/// Atoms with fixed values in the core protocol.
pub(crate) mod predefined {
    /// `ATOM`.
    pub(crate) const ATOM: u32 = 4;
    /// `CARDINAL`.
    pub(crate) const CARDINAL: u32 = 6;
    /// `RESOURCE_MANAGER`: the resource database on the root window.
    pub(crate) const RESOURCE_MANAGER: u32 = 23;
    /// `STRING` (Latin-1 text).
    pub(crate) const STRING: u32 = 31;
    /// `WM_CLIENT_MACHINE`.
    pub(crate) const WM_CLIENT_MACHINE: u32 = 36;
    /// `WM_NAME`.
    pub(crate) const WM_NAME: u32 = 39;
    /// `WM_NORMAL_HINTS`.
    pub(crate) const WM_NORMAL_HINTS: u32 = 40;
    /// `WM_SIZE_HINTS`.
    pub(crate) const WM_SIZE_HINTS: u32 = 41;
    /// `WM_CLASS`.
    pub(crate) const WM_CLASS: u32 = 67;
}

/// Declares [`Atoms`] with one field per atom name.
macro_rules! atoms {
    ($($field:ident = $name:literal,)*) => {
        /// The non-predefined atoms this crate uses.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct Atoms {
            $(
                #[doc = concat!("`", $name, "`.")]
                pub(crate) $field: u32,
            )*
        }

        impl Atoms {
            /// The atom names, in field order.
            const NAMES: &[&str] = &[$($name),*];

            /// Interns every atom with one round trip's latency.
            pub(crate) fn intern(conn: &mut Connection) -> Result<Self, Error> {
                let cookies = Self::NAMES
                    .iter()
                    .map(|name| conn.query(&request::intern_atom(name)))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut values = cookies.into_iter().map(|cookie| {
                    conn.reply(cookie).and_then(|reply| reply::intern_atom(&reply))
                });
                // Struct fields are initialised in the order written, which
                // is the order of `NAMES`.
                Ok(Self {
                    $($field: values.next().expect("one cookie per atom name")?,)*
                })
            }
        }
    };
}

atoms! {
    wm_protocols = "WM_PROTOCOLS",
    wm_delete_window = "WM_DELETE_WINDOW",
    net_wm_ping = "_NET_WM_PING",
    net_wm_name = "_NET_WM_NAME",
    net_wm_icon = "_NET_WM_ICON",
    net_wm_pid = "_NET_WM_PID",
    utf8_string = "UTF8_STRING",
    wake = "_PLT_PLATFORM_WAKE",
}
