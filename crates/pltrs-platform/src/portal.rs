//! The desktop's settings, from the XDG desktop portal
//! (`org.freedesktop.portal.Settings`, on the session bus): how GNOME
//! lays out and acts on title bars (`org.gnome.desktop.wm.preferences`),
//! which client-side decorations follow, as GTK's do.
//!
//! The portal is asked once per backend, in a thread started with it, so
//! that a slow bus delays no window; the first window with decorations
//! waits for the answer a little, and later answers reach the next
//! title bars drawn. Without a bus, a portal or GNOME's settings (other
//! desktops), title bars keep [`TitleBarSettings::default`].

use std::cell::OnceCell;
use std::sync::LazyLock;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::dbus::message::MethodCall;
use crate::dbus::value::Value;
use crate::dbus::{Bus, BusError};
use crate::decoration::TitleBarSettings;

/// The settings' namespace.
const NAMESPACE: &str = "org.gnome.desktop.wm.preferences";
/// How long the portal is waited for, at most.
const PORTAL_TIMEOUT: Duration = Duration::from_secs(2);

/// `ReadAll` of the namespace, which every version of the interface has
/// (and which, unlike `Read`, wraps values in one variant only).
const READ_ALL: MethodCall<'static> = MethodCall {
    destination: "org.freedesktop.portal.Desktop",
    path: "/org/freedesktop/portal/desktop",
    interface: "org.freedesktop.portal.Settings",
    member: "ReadAll",
    strings: Some(&[NAMESPACE]),
};

/// The title-bar settings in a `ReadAll` reply (`a{sa{sv}}`), if it has
/// the namespace.
fn title_bar_settings(reply: &[Value]) -> Option<TitleBarSettings> {
    let keys = reply.first()?.get(NAMESPACE)?;
    Some(TitleBarSettings::from_gnome(|key| {
        keys.get(key).and_then(Value::as_str).map(str::to_owned)
    }))
}

/// Asks the session bus's portal for the title-bar settings.
fn read_from_portal() -> Result<Option<TitleBarSettings>, BusError> {
    let mut bus = Bus::session(Instant::now() + PORTAL_TIMEOUT)?;
    Ok(title_bar_settings(&bus.call(&READ_ALL)?))
}

/// The title-bar settings, known or on their way from the portal.
#[derive(Debug)]
pub(crate) struct DesktopSettings {
    known: OnceCell<TitleBarSettings>,
    arriving: Option<Receiver<TitleBarSettings>>,
}

impl DesktopSettings {
    /// Settings known from the start.
    pub(crate) fn fixed(settings: TitleBarSettings) -> Self {
        Self {
            known: OnceCell::from(settings),
            arriving: None,
        }
    }

    /// Settings a thread now asks the portal for.
    pub(crate) fn from_portal() -> Self {
        let (sender, receiver) = mpsc::channel();
        let asked = thread::Builder::new()
            .name("pltrs-portal".into())
            .spawn(move || {
                let settings = read_from_portal().ok().flatten().unwrap_or_default();
                // The backend may be gone already.
                let _ = sender.send(settings);
            });
        match asked {
            Ok(_) => Self {
                known: OnceCell::new(),
                arriving: Some(receiver),
            },
            Err(_) => Self::fixed(TitleBarSettings::default()),
        }
    }

    /// The settings, waiting up to `patience` for the portal's answer
    /// if it has not come.
    pub(crate) fn wait(&self, patience: Duration) -> &TitleBarSettings {
        if self.known.get().is_none()
            && let Some(settings) = self
                .arriving
                .as_ref()
                .and_then(|arriving| arriving.recv_timeout(patience).ok())
        {
            let _ = self.known.set(settings);
        }
        self.get()
    }

    /// The settings as known now: the defaults until the portal answers.
    pub(crate) fn get(&self) -> &TitleBarSettings {
        static DEFAULT: LazyLock<TitleBarSettings> = LazyLock::new(TitleBarSettings::default);
        if self.known.get().is_none()
            && let Some(settings) = self
                .arriving
                .as_ref()
                .and_then(|arriving| arriving.try_recv().ok())
        {
            let _ = self.known.set(settings);
        }
        self.known.get().unwrap_or(&DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoration::{ButtonLayout, TitleBarAction};

    fn entry(key: &str, value: Value) -> Value {
        Value::Entry(Box::new(Value::Str(key.into())), Box::new(value))
    }

    fn string(text: &str) -> Value {
        Value::Variant(Box::new(Value::Str(text.into())))
    }

    #[test]
    fn replies_give_gnomes_title_bar_settings() {
        // As GNOME's portal answers (Fedora 45, GNOME 51).
        let reply = [Value::Array(vec![entry(
            NAMESPACE,
            Value::Array(vec![
                entry("theme", string("Adwaita")),
                entry("num-workspaces", Value::Variant(Box::new(Value::Int(4)))),
                entry("action-right-click-titlebar", string("menu")),
                entry("action-double-click-titlebar", string("toggle-maximize")),
                entry("action-middle-click-titlebar", string("none")),
                entry("button-layout", string("appmenu:close")),
            ]),
        )])];
        let settings = title_bar_settings(&reply).expect("the namespace");
        assert_eq!(settings.layout, ButtonLayout::parse(":close"));
        assert_eq!(settings.double_click, TitleBarAction::ToggleMaximize);
        assert_eq!(settings.middle_click, TitleBarAction::Nothing);
        assert_eq!(settings.right_click, TitleBarAction::Menu);
        assert_eq!(title_bar_settings(&[Value::Array(Vec::new())]), None);
        assert_eq!(title_bar_settings(&[]), None);
        let numbers = [Value::Array(vec![entry(
            NAMESPACE,
            Value::Array(vec![entry("button-layout", Value::Int(3))]),
        )])];
        assert_eq!(
            title_bar_settings(&numbers),
            Some(TitleBarSettings::default()),
            "values of the wrong type are skipped"
        );
    }

    #[test]
    fn settings_arrive_or_stay_the_defaults() {
        let fixed = DesktopSettings::fixed(TitleBarSettings {
            layout: ButtonLayout::parse("close:"),
            ..TitleBarSettings::default()
        });
        assert_eq!(fixed.get().layout.left(), [crate::TitleBarButton::Close]);
        let (sender, receiver) = mpsc::channel();
        let arriving = DesktopSettings {
            known: OnceCell::new(),
            arriving: Some(receiver),
        };
        assert_eq!(*arriving.get(), TitleBarSettings::default(), "not yet");
        assert_eq!(
            *arriving.wait(Duration::from_millis(1)),
            TitleBarSettings::default()
        );
        let close_only = TitleBarSettings {
            layout: ButtonLayout::parse(":close"),
            ..TitleBarSettings::default()
        };
        sender.send(close_only.clone()).unwrap();
        assert_eq!(*arriving.wait(Duration::from_secs(1)), close_only);
        assert_eq!(*arriving.get(), close_only);
    }

    #[test]
    fn the_portal_is_asked_without_failing() {
        // Whatever this session has (a GNOME portal, another, none), the
        // answer comes within the timeout.
        let settings = DesktopSettings::from_portal();
        let started = Instant::now();
        let _ = settings.wait(PORTAL_TIMEOUT + Duration::from_secs(1));
        assert!(started.elapsed() <= PORTAL_TIMEOUT + Duration::from_secs(1));
    }
}
