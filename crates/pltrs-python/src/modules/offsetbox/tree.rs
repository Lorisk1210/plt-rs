//! Offset-box trees described by Python (`OffsetBox._native_tree`): the
//! boxes of matplotlib's `offsetbox` with the loose artists they hold,
//! which move into the tree (`BoxItem::from_loose`).
//!
//! A box crosses as a tuple, the first item naming its kind:
//! `("text", text, multilinebaseline)`, `("drawing", width, height,
//! xdescent, ydescent, clip, artists)`, `("aux", transform, artists)`,
//! `("hpacker" | "vpacker", pad, sep, width, height, align, mode,
//! children)`, `("padded", pad, child, draw_frame, patch_kwargs)`,
//! `("image", image, zoom, dpi_cor)` and `("anchored", child, options)`
//! (an `AnchoredOffsetbox` inside another box, its options as
//! `add_anchored` takes them).

use pltrs::artist::RawHandle;
use pltrs::offsetbox::{
    AnchoredBox, AuxTransformBox, BoxItem, DrawingArea, OffsetBox, OffsetImage, Packer, PaddedBox,
    TextArea,
};
use pltrs::transforms::Transform;

use super::super::patches::PatchKwargs;
use super::add::PyAnchoredOptions;
use super::{pack_align, pack_mode};
use crate::binding::prelude::*;

/// The items of the loose artists `artists`, moved out of their holders.
fn items(artists: &Bound<'_>) -> PyResult<Vec<BoxItem>> {
    artists
        .extract::<Vec<RawHandle>>()?
        .iter()
        .map(|artist| Ok(BoxItem::from_loose(artist)?))
        .collect()
}

/// An offset-box tree described by Python (see the module documentation).
pub(super) struct PyOffsetBox(pub(super) OffsetBox<BoxItem>);

impl FromPy<'_> for PyOffsetBox {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let parts: Vec<Bound<'_>> = object.extract()?;
        let kind: String = parts
            .first()
            .ok_or_else(|| PyErr::value_error("an empty offset box description"))?
            .extract()?;
        let tree = match (kind.as_str(), &parts[1..]) {
            ("text", [text, multilinebaseline]) => OffsetBox::TextArea(TextArea::new(
                BoxItem::from_loose(&text.extract::<RawHandle>()?)?,
                multilinebaseline.extract()?,
            )),
            ("drawing", [width, height, xdescent, ydescent, clip, artists]) => {
                let mut area = DrawingArea::new(
                    width.extract()?,
                    height.extract()?,
                    xdescent.extract()?,
                    ydescent.extract()?,
                );
                area.clip = clip.extract()?;
                area.artists = items(artists)?;
                OffsetBox::DrawingArea(area)
            }
            ("aux", [transform, artists]) => {
                OffsetBox::AuxTransformBox(items(artists)?.into_iter().fold(
                    AuxTransformBox::new(transform.extract::<Transform>()?),
                    AuxTransformBox::with_artist,
                ))
            }
            (
                packer @ ("hpacker" | "vpacker"),
                [pad, sep, width, height, align, mode, children],
            ) => {
                let children: Vec<PyOffsetBox> = children.extract()?;
                let mut packed = Packer::new(
                    pad.extract()?,
                    sep.extract()?,
                    pack_align(&align.extract::<String>()?)?,
                    children.into_iter().map(|child| child.0).collect(),
                );
                packed.width = width.extract()?;
                packed.height = height.extract()?;
                packed.mode = pack_mode(&mode.extract::<String>()?)?;
                if packer == "hpacker" {
                    OffsetBox::HPacker(packed)
                } else {
                    OffsetBox::VPacker(packed)
                }
            }
            ("padded", [pad, child, draw_frame, patch_kwargs]) => {
                OffsetBox::PaddedBox(PaddedBox::new(
                    child.extract::<PyOffsetBox>()?.0,
                    pad.extract()?,
                    draw_frame.extract()?,
                    patch_kwargs.extract::<PatchKwargs>()?.0,
                ))
            }
            ("anchored", [child, options]) => OffsetBox::from(AnchoredBox::new(
                child.extract::<PyOffsetBox>()?.0,
                options.extract::<PyAnchoredOptions>()?.0,
            )),
            ("image", [image, zoom, dpi_cor]) => OffsetBox::Image(OffsetImage {
                image: BoxItem::from_loose(&image.extract::<RawHandle>()?)?,
                zoom: zoom.extract()?,
                dpi_cor: dpi_cor.extract()?,
                visible: true,
            }),
            _ => {
                return Err(PyErr::value_error(format!(
                    "a malformed offset box description of kind {kind:?}"
                )));
            }
        };
        Ok(Self(tree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};
    use pltrs::artist::ArtistHandle;
    use pltrs::text::Text;

    #[test]
    fn descriptions_take_their_loose_artists() {
        with_python(|py| {
            extension(py);
            let text = Text::loose(0.0, 0.0, "a");
            let description = (
                "vpacker",
                (0.0, 2.0),
                ("center", "fixed"),
                vec![("text", text.raw().clone(), false)],
            );
            let (kind, (pad, sep), (align, mode), children) = description;
            let packed = vec![
                kind.into_py(py).unwrap(),
                pad.into_py(py).unwrap(),
                sep.into_py(py).unwrap(),
                Bound::none(py),
                Bound::none(py),
                align.into_py(py).unwrap(),
                mode.into_py(py).unwrap(),
                children.into_py(py).unwrap(),
            ]
            .into_py(py)
            .unwrap();
            let PyOffsetBox(tree) = packed.extract().unwrap();
            let OffsetBox::VPacker(packer) = &tree else {
                panic!("a vertical packer, not {tree:?}");
            };
            assert_eq!(packer.children.len(), 1);
            assert_eq!(tree.artists().len(), 1);
            assert!(!text.raw().is_attached(), "the text left its holder");
            let unknown = eval(py, "", "('frame', 1.0)");
            assert!(unknown.extract::<PyOffsetBox>().is_err());
        });
    }
}
