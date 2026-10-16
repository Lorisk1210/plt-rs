//! The connection handshake: setup request encoding and setup reply
//! parsing (pixmap formats, screens, depths and visuals).

use super::auth::Credentials;
use super::wire::{Reader, WireWrite};
use crate::error::Error;

/// Protocol version spoken by this crate.
const PROTOCOL_MAJOR: u16 = 11;
/// TrueColor visual class.
pub(crate) const VISUAL_CLASS_TRUE_COLOR: u8 = 4;

/// Encodes the setup request (little-endian byte order).
pub(crate) fn encode_request(credentials: Option<&Credentials>) -> Vec<u8> {
    let (name, data) = credentials.map_or((&[][..], &[][..]), |c| (&c.name[..], &c.data[..]));
    let mut buf = Vec::with_capacity(12 + name.len() + data.len() + 6);
    buf.put_u8(b'l');
    buf.put_u8(0);
    buf.put_u16(PROTOCOL_MAJOR);
    buf.put_u16(0);
    buf.put_u16(u16::try_from(name.len()).unwrap_or(0));
    buf.put_u16(u16::try_from(data.len()).unwrap_or(0));
    buf.put_u16(0);
    buf.put_padded(name);
    buf.put_padded(data);
    buf
}

/// Byte order of image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageByteOrder {
    /// Least significant byte first.
    LsbFirst,
    /// Most significant byte first.
    MsbFirst,
}

/// A pixmap format: how pixels of one depth are stored in images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Format {
    pub(crate) depth: u8,
    pub(crate) bits_per_pixel: u8,
    pub(crate) scanline_pad: u8,
}

/// A visual: how pixel values map to colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Visual {
    pub(crate) id: u32,
    pub(crate) class: u8,
    pub(crate) red_mask: u32,
    pub(crate) green_mask: u32,
    pub(crate) blue_mask: u32,
}

/// The visuals available at one depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Depth {
    pub(crate) depth: u8,
    pub(crate) visuals: Vec<Visual>,
}

/// A screen (root window) of the display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Screen {
    pub(crate) root: u32,
    pub(crate) default_colormap: u32,
    pub(crate) width_px: u16,
    pub(crate) height_px: u16,
    pub(crate) root_visual: u32,
    pub(crate) root_depth: u8,
    pub(crate) depths: Vec<Depth>,
}

impl Screen {
    /// Finds a visual by id, with its depth.
    pub(crate) fn visual(&self, id: u32) -> Option<(u8, Visual)> {
        self.depths.iter().find_map(|depth| {
            let visual = depth.visuals.iter().find(|v| v.id == id)?;
            Some((depth.depth, *visual))
        })
    }

    /// The visual to render with: the root visual if it is 24-bit
    /// TrueColor, else the first 24-bit TrueColor visual.
    pub(crate) fn true_color_visual(&self) -> Option<(u8, Visual)> {
        let suitable = |(depth, visual): &(u8, Visual)| {
            *depth == 24 && visual.class == VISUAL_CLASS_TRUE_COLOR
        };
        self.visual(self.root_visual).filter(suitable).or_else(|| {
            self.depths
                .iter()
                .filter(|depth| depth.depth == 24)
                .flat_map(|depth| depth.visuals.iter().map(|v| (depth.depth, *v)))
                .find(suitable)
        })
    }
}

/// The parsed setup reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Setup {
    pub(crate) resource_id_base: u32,
    pub(crate) resource_id_mask: u32,
    /// Maximum request length in 4-byte units (without BIG-REQUESTS).
    pub(crate) maximum_request_length: u16,
    pub(crate) image_byte_order: ImageByteOrder,
    pub(crate) min_keycode: u8,
    pub(crate) max_keycode: u8,
    pub(crate) vendor: String,
    pub(crate) formats: Vec<Format>,
    pub(crate) screens: Vec<Screen>,
}

impl Setup {
    /// The pixmap format used for images of `depth`.
    pub(crate) fn format(&self, depth: u8) -> Option<Format> {
        self.formats.iter().copied().find(|f| f.depth == depth)
    }
}

/// Number of bytes in the fixed part of the setup reply header.
pub(crate) const REPLY_HEADER_LEN: usize = 8;

/// The length of the reply body following the 8-byte header.
pub(crate) fn reply_body_len(header: &[u8; REPLY_HEADER_LEN]) -> usize {
    usize::from(u16::from_le_bytes([header[6], header[7]])) * 4
}

/// Parses a complete setup reply (header plus body).
pub(crate) fn parse_reply(reply: &[u8]) -> Result<Setup, Error> {
    let mut r = Reader::new(reply);
    match r.u8()? {
        1 => parse_success(reply),
        status @ (0 | 2) => {
            let reason_len = usize::from(r.u8()?);
            let reason = if status == 0 {
                Reader::at(reply, REPLY_HEADER_LEN).bytes(reason_len)?
            } else {
                // `Authenticate` replies carry the reason in the whole body.
                Reader::at(reply, REPLY_HEADER_LEN).bytes(reply.len() - REPLY_HEADER_LEN)?
            };
            let reason = String::from_utf8_lossy(reason);
            let reason = reason.trim_end_matches(['\0', '\n']);
            Err(Error::ConnectionRefused(reason.to_owned()))
        }
        _ => Err(Error::Malformed("unknown setup status")),
    }
}

fn parse_success(reply: &[u8]) -> Result<Setup, Error> {
    let mut r = Reader::at(reply, 8);
    r.skip(4)?; // release number
    let resource_id_base = r.u32()?;
    let resource_id_mask = r.u32()?;
    r.skip(4)?; // motion buffer size
    let vendor_len = usize::from(r.u16()?);
    let maximum_request_length = r.u16()?;
    let screen_count = r.u8()?;
    let format_count = r.u8()?;
    let image_byte_order = match r.u8()? {
        0 => ImageByteOrder::LsbFirst,
        1 => ImageByteOrder::MsbFirst,
        _ => return Err(Error::Malformed("invalid image byte order")),
    };
    r.skip(3)?; // bitmap bit order, scanline unit, scanline pad
    let min_keycode = r.u8()?;
    let max_keycode = r.u8()?;
    r.skip(4)?;
    let vendor = String::from_utf8_lossy(r.bytes(vendor_len)?).into_owned();
    r.skip(super::wire::pad4(vendor_len))?;
    let formats = (0..format_count)
        .map(|_| {
            let format = Format {
                depth: r.u8()?,
                bits_per_pixel: r.u8()?,
                scanline_pad: r.u8()?,
            };
            r.skip(5)?;
            Ok(format)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let screens = (0..screen_count)
        .map(|_| parse_screen(&mut r))
        .collect::<Result<Vec<_>, Error>>()?;
    if resource_id_mask == 0 {
        return Err(Error::Malformed("empty resource id mask"));
    }
    Ok(Setup {
        resource_id_base,
        resource_id_mask,
        maximum_request_length,
        image_byte_order,
        min_keycode,
        max_keycode,
        vendor,
        formats,
        screens,
    })
}

fn parse_screen(r: &mut Reader<'_>) -> Result<Screen, Error> {
    let root = r.u32()?;
    let default_colormap = r.u32()?;
    r.skip(12)?; // white pixel, black pixel, current input masks
    let width_px = r.u16()?;
    let height_px = r.u16()?;
    r.skip(8)?; // millimetres, installed colormaps
    let root_visual = r.u32()?;
    r.skip(2)?; // backing stores, save unders
    let root_depth = r.u8()?;
    let depth_count = r.u8()?;
    let depths = (0..depth_count)
        .map(|_| parse_depth(r))
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Screen {
        root,
        default_colormap,
        width_px,
        height_px,
        root_visual,
        root_depth,
        depths,
    })
}

fn parse_depth(r: &mut Reader<'_>) -> Result<Depth, Error> {
    let depth = r.u8()?;
    r.skip(1)?;
    let visual_count = r.u16()?;
    r.skip(4)?;
    let visuals = (0..visual_count)
        .map(|_| {
            let id = r.u32()?;
            let class = r.u8()?;
            r.skip(3)?; // bits per rgb, colormap entries
            let visual = Visual {
                id,
                class,
                red_mask: r.u32()?,
                green_mask: r.u32()?,
                blue_mask: r.u32()?,
            };
            r.skip(4)?;
            Ok(visual)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Depth { depth, visuals })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A setup reply captured from XWayland (mutter, GNOME 49).
    const XWAYLAND: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/x11/setup_xwayland.bin"
    ));

    #[test]
    fn request_without_credentials() {
        assert_eq!(encode_request(None), [b'l', 0, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn request_with_cookie_is_padded() {
        let creds = Credentials {
            name: b"MIT-MAGIC-COOKIE-1".to_vec(),
            data: vec![0xaa; 16],
        };
        let request = encode_request(Some(&creds));
        assert_eq!(&request[..12], [b'l', 0, 11, 0, 0, 0, 18, 0, 16, 0, 0, 0]);
        assert_eq!(&request[12..30], b"MIT-MAGIC-COOKIE-1");
        assert_eq!(&request[30..32], [0, 0], "name padded to 20 bytes");
        assert_eq!(&request[32..48], [0xaa; 16]);
        assert_eq!(request.len(), 48);
    }

    #[test]
    fn parses_captured_xwayland_reply() {
        let header: [u8; 8] = XWAYLAND[..8].try_into().unwrap();
        assert_eq!(reply_body_len(&header) + 8, XWAYLAND.len());
        let setup = parse_reply(XWAYLAND).unwrap();
        assert_eq!(setup.vendor, "The X.Org Foundation");
        assert_eq!(setup.maximum_request_length, 0xffff);
        assert_eq!(setup.image_byte_order, ImageByteOrder::LsbFirst);
        assert_eq!((setup.min_keycode, setup.max_keycode), (8, 255));
        assert_eq!(setup.formats.len(), 7);
        let format = setup.format(24).unwrap();
        assert_eq!((format.bits_per_pixel, format.scanline_pad), (32, 32));
        assert_eq!(setup.screens.len(), 1);
        let screen = &setup.screens[0];
        assert_eq!(screen.root_depth, 24);
        let (depth, visual) = screen.true_color_visual().unwrap();
        assert_eq!(depth, 24);
        assert_eq!(visual.id, screen.root_visual);
        assert_eq!(
            (visual.red_mask, visual.green_mask, visual.blue_mask),
            (0xff_0000, 0xff00, 0xff)
        );
        assert!(screen.width_px > 0 && screen.height_px > 0);
    }

    #[test]
    fn truncated_reply_is_an_error() {
        for len in [1, 8, 40, XWAYLAND.len() - 1] {
            assert!(parse_reply(&XWAYLAND[..len]).is_err(), "length {len}");
        }
    }

    #[test]
    fn refusal_reason_is_reported() {
        let reason = b"No protocol specified\n";
        let mut reply = vec![0, reason.len() as u8, 11, 0, 0, 0, 6, 0];
        reply.extend_from_slice(reason);
        reply.resize(8 + 24, 0);
        assert_eq!(
            parse_reply(&reply),
            Err(Error::ConnectionRefused("No protocol specified".into()))
        );
        let mut reply = vec![2, 0, 0, 0, 0, 0, 2, 0];
        reply.extend_from_slice(b"auth\0\0\0\0");
        assert_eq!(parse_reply(&reply), Err(Error::ConnectionRefused("auth".into())));
    }
}
