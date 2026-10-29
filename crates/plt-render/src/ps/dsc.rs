//! Document structuring comments: the bounding box a document declares
//! (`_get_bbox_header`) and matplotlib's `pstoeps`, which turns the
//! PostScript a distiller wrote back into Encapsulated PostScript.

use plt_core::Bbox;

/// `_get_bbox_header`: the `%%BoundingBox` (whole points, outwards) and
/// `%%HiResBoundingBox` comments of `bbox`, without a final newline.
pub(super) fn bbox_header(bbox: &Bbox) -> String {
    let (l, b, r, t) = (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1());
    format!(
        "%%BoundingBox: {} {} {} {}\n%%HiResBoundingBox: {l:.6} {b:.6} {r:.6} {t:.6}",
        l.trunc() as i64,
        b.trunc() as i64,
        r.ceil() as i64,
        t.ceil() as i64
    )
}

/// `_get_rotate_command`: turns the page of `bbox` on its side.
fn rotate_command(bbox: &Bbox) -> String {
    format!(
        "{:.2} {:.2} translate\n90 rotate",
        bbox.x0() + bbox.x1(),
        0.0
    )
}

/// The comments `pstoeps` drops from the header when it writes a bounding
/// box of its own.
const REPLACED_COMMENTS: [&[u8]; 4] = [b"%%Bound", b"%%HiResBound", b"%%DocumentMedia", b"%%Pages"];

/// What `pstoeps` puts after the header comments: a prolog saving the
/// interpreter's state and disabling `showpage` and `setpagedevice`, and
/// the page (matplotlib's `%%Page` comment lacks its colon).
const EPS_PROLOG: &[u8] = b"%%BeginProlog\nsave\ncountdictstack\nmark\nnewpath\n\
    /showpage {} def\n/setpagedevice {pop} def\n%%EndProlog\n%%Page 1 1\n";

/// What replaces the `%%EOF` comment: the state saved in the prolog
/// restored, then the page shown.
const EPS_TRAILER: &[u8] =
    b"cleartomark\ncountdictstack\nexch sub { end } repeat\nrestore\nshowpage\n%%EOF\n";

/// matplotlib's `pstoeps`: `document`, a PostScript file a distiller
/// wrote, as Encapsulated PostScript. The header declares EPS and, given a
/// `bbox`, that bounding box instead of the page's; after the header
/// comments come a prolog saving the interpreter's state and disabling
/// `showpage` and `setpagedevice` and, when `rotated` (`dvips` turned a
/// usetex figure into a landscape page), the rotation of `bbox`. The
/// `%%EOF` comment becomes the state restored and the page shown, and
/// `%%PageBoundingBox` comments go.
///
/// ```
/// use plt_render::pstoeps;
/// let eps = pstoeps(b"%!PS-Adobe-3.0\n%%Pages: 1\n%%EndComments\nshowpage\n%%EOF\n", None, false);
/// assert!(eps.starts_with(b"%!PS-Adobe-3.0 EPSF-3.0\n%%Pages: 1\n%%EndComments\n%%BeginProlog\n"));
/// assert!(eps.ends_with(b"showpage\ncleartomark\ncountdictstack\nexch sub { end } repeat\nrestore\nshowpage\n%%EOF\n"));
/// ```
pub fn pstoeps(document: &[u8], bbox: Option<&Bbox>, rotated: bool) -> Vec<u8> {
    let mut eps = Vec::with_capacity(document.len() + EPS_PROLOG.len() + EPS_TRAILER.len());
    let mut lines = document.split_inclusive(|&byte| byte == b'\n');
    for line in lines.by_ref() {
        if line.starts_with(b"%!PS") {
            eps.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");
            if let Some(bbox) = bbox {
                eps.extend_from_slice(bbox_header(bbox).as_bytes());
                eps.push(b'\n');
            }
        } else if line.starts_with(b"%%EndComments") {
            eps.extend_from_slice(line);
            eps.extend_from_slice(EPS_PROLOG);
            if let (true, Some(bbox)) = (rotated, bbox) {
                eps.extend_from_slice(rotate_command(bbox).as_bytes());
                eps.push(b'\n');
            }
            break;
        } else if bbox.is_none()
            || !REPLACED_COMMENTS
                .iter()
                .any(|comment| line.starts_with(comment))
        {
            eps.extend_from_slice(line);
        }
    }
    // The rest of the file, where an embedded EPS file's header is left
    // alone.
    for line in lines {
        if line.starts_with(b"%%EOF") {
            eps.extend_from_slice(EPS_TRAILER);
        } else if !line.starts_with(b"%%PageBoundingBox") {
            eps.extend_from_slice(line);
        }
    }
    eps
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISTILLED: &[u8] = b"%!PS-Adobe-3.0\n%%Invocation: gs\n%%BoundingBox: 0 0 612 792\n\
        %%HiResBoundingBox: 0 0 612.00 792.00\n%%Pages: 1\n%%EndComments\n%%BeginProlog\n\
        %%EndProlog\n%%Page: 1 1\n%%PageBoundingBox: 0 0 612 792\n0 0 m\n%%BoundingBox: 1 2 3 4\n\
        %%Trailer\n%%EOF\n";

    #[test]
    fn bounding_boxes_round_outwards() {
        let bbox = Bbox::from_extents(-0.5, 0.25, 10.5, 20.0);
        assert_eq!(
            bbox_header(&bbox),
            "%%BoundingBox: 0 0 11 20\n%%HiResBoundingBox: -0.500000 0.250000 10.500000 20.000000"
        );
        assert_eq!(
            rotate_command(&Bbox::from_extents(0.0, 0.0, 144.0, 230.4)),
            "144.00 0.00 translate\n90 rotate"
        );
    }

    #[test]
    fn distilled_documents_become_eps() {
        let bbox = Bbox::from_extents(0.0, 0.0, 144.0, 230.4);
        let eps = String::from_utf8(pstoeps(DISTILLED, Some(&bbox), true)).unwrap();
        assert_eq!(
            eps,
            "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 144 231\n\
             %%HiResBoundingBox: 0.000000 0.000000 144.000000 230.400000\n%%Invocation: gs\n\
             %%EndComments\n%%BeginProlog\nsave\ncountdictstack\nmark\nnewpath\n\
             /showpage {} def\n/setpagedevice {pop} def\n%%EndProlog\n%%Page 1 1\n\
             144.00 0.00 translate\n90 rotate\n%%BeginProlog\n%%EndProlog\n%%Page: 1 1\n0 0 m\n\
             %%BoundingBox: 1 2 3 4\n%%Trailer\ncleartomark\ncountdictstack\n\
             exch sub { end } repeat\nrestore\nshowpage\n%%EOF\n"
        );
    }

    #[test]
    fn without_a_bounding_box_the_header_stays() {
        let eps = String::from_utf8(pstoeps(DISTILLED, None, true)).unwrap();
        assert!(eps.starts_with(
            "%!PS-Adobe-3.0 EPSF-3.0\n%%Invocation: gs\n%%BoundingBox: 0 0 612 792\n\
             %%HiResBoundingBox: 0 0 612.00 792.00\n%%Pages: 1\n%%EndComments\n%%BeginProlog\n"
        ));
        assert!(
            eps.contains("%%Page 1 1\n%%BeginProlog\n"),
            "no rotation without a box"
        );
        assert_eq!(pstoeps(b"", None, false), b"");
        assert_eq!(pstoeps(b"no newline", None, false), b"no newline");
    }
}
