//! Per-window state, and the window-manager properties (ICCCM and EWMH)
//! that describe a top-level window.

use super::atoms::{Atoms, predefined};
use super::auth;
use super::request::{self, Request};
use super::selection::CHANGE_PROPERTY_OVERHEAD;
use crate::geometry::PhysicalSize;
use crate::icon::{Icon, IconLayout};
use crate::window::WindowOptions;

/// `WM_SIZE_HINTS` flag: the minimum size is set.
const P_MIN_SIZE: u32 = 1 << 4;
/// Number of 32-bit fields of `WM_SIZE_HINTS`.
const SIZE_HINTS_LEN: usize = 18;
/// Index of the minimum width in `WM_SIZE_HINTS`.
const SIZE_HINTS_MIN_WIDTH: usize = 5;
/// The sides of the icon images in `_NET_WM_ICON`, from the title bar's
/// to the task switcher's and docks'.
const ICON_SIDES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// What the backend tracks about each open window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowState {
    /// The graphics context used to present frames.
    pub(crate) gc: u32,
    /// Content size, as last configured.
    pub(crate) size: PhysicalSize,
    /// Whether the pointer is inside (to drop duplicate crossings).
    pub(crate) pointer_inside: bool,
    /// Whether the window has keyboard focus (to drop duplicates).
    pub(crate) focused: bool,
}

impl WindowState {
    /// State of a new window.
    pub(crate) fn new(gc: u32, size: PhysicalSize) -> Self {
        Self {
            gc,
            size,
            pointer_inside: false,
            focused: false,
        }
    }
}

/// Requests setting the title: `_NET_WM_NAME` in UTF-8 for modern window
/// managers, and `WM_NAME` in Latin-1 (unrepresentable characters become
/// `?`) for the rest.
pub(crate) fn title(atoms: &Atoms, window: u32, title: &str) -> [Request; 2] {
    let latin1: Vec<u8> = title
        .chars()
        .map(|c| u8::try_from(c).unwrap_or(b'?'))
        .collect();
    [
        request::set_property_bytes(
            window,
            atoms.net_wm_name,
            atoms.utf8_string,
            title.as_bytes(),
        ),
        request::set_property_bytes(window, predefined::WM_NAME, predefined::STRING, &latin1),
    ]
}

/// The `_NET_ACTIVE_WINDOW` client message asking an EWMH window manager
/// to raise and focus `window`, sent by a normal application (source 1)
/// without a timestamp.
pub(crate) fn activate(atoms: &Atoms, window: u32) -> [u8; 32] {
    request::client_message(window, atoms.net_active_window, [1, 0, 0, 0, 0])
}

/// The `_NET_WM_STATE` client message asking an EWMH window manager to
/// add (or remove) the full-screen state of `window`.
pub(crate) fn fullscreen(atoms: &Atoms, window: u32, on: bool) -> [u8; 32] {
    let action = u32::from(on);
    request::client_message(
        window,
        atoms.net_wm_state,
        [action, atoms.net_wm_state_fullscreen, 0, 1, 0],
    )
}

/// Requests describing a new window: title, class, supported protocols
/// (close button and ping), process and minimum size (the icon is
/// [`icon_property`]).
pub(crate) fn initial_properties(
    atoms: &Atoms,
    window: u32,
    options: &WindowOptions,
    scale_factor: f64,
) -> Vec<Request> {
    let mut requests = Vec::from(title(atoms, window, &options.title));
    let class = format!("{0}\0{0}\0", options.app_id);
    requests.push(request::set_property_bytes(
        window,
        predefined::WM_CLASS,
        predefined::STRING,
        class.as_bytes(),
    ));
    let protocols = [atoms.wm_delete_window, atoms.net_wm_ping];
    requests.push(request::set_property_u32(
        window,
        atoms.wm_protocols,
        predefined::ATOM,
        &protocols,
    ));
    requests.push(request::set_property_u32(
        window,
        atoms.net_wm_pid,
        predefined::CARDINAL,
        &[std::process::id()],
    ));
    if let Some(host) = auth::hostname() {
        requests.push(request::set_property_bytes(
            window,
            predefined::WM_CLIENT_MACHINE,
            predefined::STRING,
            host.as_bytes(),
        ));
    }
    if let Some(min) = options.min_inner_size {
        let min = min.to_physical(scale_factor);
        requests.push(request::set_property_u32(
            window,
            predefined::WM_NORMAL_HINTS,
            predefined::WM_SIZE_HINTS,
            &min_size_hints(min),
        ));
    }
    requests
}

/// The request setting `_NET_WM_ICON` to the images of `icon` that fit a
/// request of `max_request_bytes`, the smallest first; `None` when not
/// even the smallest fits (or the icon draws none).
pub(crate) fn icon_property(
    atoms: &Atoms,
    window: u32,
    icon: &Icon,
    max_request_bytes: usize,
) -> Option<Request> {
    let room = max_request_bytes.saturating_sub(CHANGE_PROPERTY_OVERHEAD) / 4;
    let mut images = icon.images(&ICON_SIDES, IconLayout::FullBleed);
    images.sort_by_key(|image| image.size().width);
    let mut words = Vec::new();
    for image in &images {
        let size = image.size();
        let len = 2 + size.width as usize * size.height as usize;
        if words.len() + len > room {
            break;
        }
        words.extend([size.width, size.height]);
        words.extend(image.rgba().as_chunks::<4>().0.iter().map(|p| {
            u32::from(p[3]) << 24 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
        }));
    }
    (!words.is_empty())
        .then(|| request::set_property_u32(window, atoms.net_wm_icon, predefined::CARDINAL, &words))
}

/// `WM_SIZE_HINTS` carrying only a minimum size.
fn min_size_hints(min: PhysicalSize) -> [u32; SIZE_HINTS_LEN] {
    let mut hints = [0; SIZE_HINTS_LEN];
    hints[0] = P_MIN_SIZE;
    hints[SIZE_HINTS_MIN_WIDTH] = min.width;
    hints[SIZE_HINTS_MIN_WIDTH + 1] = min.height;
    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::LogicalSize;
    use crate::icon::IconImage;

    const ATOMS: Atoms = Atoms {
        wm_protocols: 300,
        wm_delete_window: 301,
        net_wm_ping: 302,
        net_wm_name: 303,
        net_wm_icon: 304,
        net_wm_pid: 305,
        utf8_string: 306,
        wake: 307,
        net_active_window: 308,
        net_wm_state: 309,
        net_wm_state_fullscreen: 310,
        clipboard: 311,
        targets: 312,
        image_png: 313,
        text: 314,
        text_plain_utf8: 315,
        incr: 316,
        paste: 317,
    };

    /// `(property, type, value)` of a `ChangeProperty` request.
    fn property(request: &Request) -> (u32, u32, Vec<u8>) {
        let body = request.encoded().body();
        let word = |at: usize| u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        let units = word(16) as usize;
        let len = units * usize::from(body[12] / 8);
        (word(4), word(8), body[20..20 + len].to_vec())
    }

    fn words(bytes: &[u8]) -> Vec<u32> {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect()
    }

    #[test]
    fn title_in_utf8_and_latin1() {
        let [utf8, latin1] = title(&ATOMS, 1, "Figure 1 – α é");
        assert_eq!(
            property(&utf8),
            (303, 306, "Figure 1 – α é".as_bytes().to_vec())
        );
        let (atom, kind, value) = property(&latin1);
        assert_eq!((atom, kind), (predefined::WM_NAME, predefined::STRING));
        assert_eq!(value, b"Figure 1 ? ? \xe9");
    }

    #[test]
    fn initial_properties_describe_the_window() {
        let options = WindowOptions::new("t")
            .with_app_id("pltrs")
            .with_min_inner_size(LogicalSize::new(100.0, 50.0));
        let properties: Vec<_> = initial_properties(&ATOMS, 1, &options, 2.0)
            .iter()
            .map(property)
            .collect();
        let find = |atom| properties.iter().find(|p| p.0 == atom).unwrap().2.clone();
        assert_eq!(find(predefined::WM_CLASS), b"pltrs\0pltrs\0");
        assert_eq!(words(&find(ATOMS.wm_protocols)), [301, 302]);
        assert_eq!(words(&find(ATOMS.net_wm_pid)), [std::process::id()]);
        let hints = words(&find(predefined::WM_NORMAL_HINTS));
        assert_eq!(hints.len(), SIZE_HINTS_LEN);
        assert_eq!((hints[0], hints[5], hints[6]), (P_MIN_SIZE, 200, 100));
    }

    #[test]
    fn bitmap_icons_are_argb() {
        let icon = Icon::from_rgba(1, 1, vec![0x11, 0x22, 0x33, 0x80]).unwrap();
        let request = icon_property(&ATOMS, 1, &icon, 1 << 20).unwrap();
        let (atom, kind, value) = property(&request);
        assert_eq!((atom, kind), (ATOMS.net_wm_icon, predefined::CARDINAL));
        assert_eq!(words(&value), [1, 1, 0x8011_2233]);
    }

    /// An icon drawing each side full bleed in a grey of its side.
    fn rendered() -> Icon {
        Icon::from_renderer(|side, layout| {
            assert_eq!(layout, IconLayout::FullBleed);
            let grey = (side % 256) as u8;
            let pixels = vec![grey; (side * side * 4) as usize];
            IconImage::from_rgba(side, side, pixels).ok()
        })
    }

    /// The `(width, height, first pixel)` of each image of `_NET_WM_ICON`.
    fn images(value: &[u32]) -> Vec<(u32, u32, u32)> {
        let mut images = Vec::new();
        let mut rest = value;
        while let [width, height, pixels @ ..] = rest {
            images.push((*width, *height, pixels[0]));
            rest = &pixels[(width * height) as usize..];
        }
        images
    }

    #[test]
    fn rendered_icons_come_in_every_size() {
        let request = icon_property(&ATOMS, 1, &rendered(), 1 << 22).unwrap();
        let value = words(&property(&request).2);
        let sides: Vec<u32> = images(&value).iter().map(|image| image.0).collect();
        assert_eq!(sides, ICON_SIDES);
        assert!(images(&value).iter().all(|&(w, h, _)| w == h));
        assert_eq!(images(&value)[1].2, 0x1818_1818, "24 pixels, grey 24");
    }

    #[test]
    fn icons_shrink_to_the_request_limit() {
        // Without BIG-REQUESTS: 256 KiB, too little for 256 × 256.
        let request = icon_property(&ATOMS, 1, &rendered(), 0xffff * 4).unwrap();
        let value = words(&property(&request).2);
        let sides: Vec<u32> = images(&value).iter().map(|image| image.0).collect();
        assert_eq!(sides, [16, 24, 32, 48, 64, 128]);
        assert!(request.encoded().wire_len() <= 0xffff * 4);
        assert!(icon_property(&ATOMS, 1, &rendered(), 64).is_none());
    }

    #[test]
    fn activation_is_a_client_message() {
        let event = activate(&ATOMS, 9);
        assert_eq!(event[0], 33, "a ClientMessage");
        assert_eq!(words(&event[4..]), [9, 308, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn fullscreen_is_a_state_message() {
        let event = fullscreen(&ATOMS, 9, true);
        assert_eq!(words(&event[4..]), [9, 309, 1, 310, 0, 1, 0]);
        assert_eq!(words(&fullscreen(&ATOMS, 9, false)[12..16]), [0]);
    }

    #[test]
    fn optional_properties_are_omitted() {
        let requests = initial_properties(&ATOMS, 1, &WindowOptions::new("t"), 1.0);
        let atoms: Vec<u32> = requests.iter().map(|r| property(r).0).collect();
        assert!(!atoms.contains(&predefined::WM_NORMAL_HINTS));
    }
}
