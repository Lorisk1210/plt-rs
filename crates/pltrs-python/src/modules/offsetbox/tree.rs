//! Offset-box trees described by Python (`OffsetBox._native_tree`): the
//! boxes of matplotlib's `offsetbox` with the artists they hold, loose
//! ones (which move into the tree when it joins a figure,
//! `BoxItem::from_loose`) or, for a tree handed again
//! (`pltrs::offsetbox::rebuild_tree`), those its box holds already.
//!
//! A box crosses as a tuple naming its kind and whether it is visible,
//! then its settings: `("text", visible, text, multilinebaseline)`,
//! `("drawing", visible, width, height, xdescent, ydescent, clip,
//! artists)`, `("aux", visible, transform, artists)`, `("hpacker" |
//! "vpacker", visible, pad, sep, width, height, align, mode, children)`,
//! `("padded", visible, pad, child, frame)`, `("image", visible, image,
//! zoom, dpi_cor)`, `("anchored", visible, child, options, frame)` (an
//! `AnchoredOffsetbox` inside another box, its options as `add_anchored`
//! takes them) and `("program", visible, box)` (an `OffsetBox` subclass
//! of the program's, sizing and drawing itself, its own artist); frames
//! are `FancyBboxPatch`es.

use pltrs::artist::RawHandle;
use pltrs::offsetbox::{
    AnchoredNode, AuxTransformBox, BoxItem, DrawingArea, OffsetBox, OffsetImage, Packer, PaddedBox,
    ProgramBox, TextArea,
};
use pltrs::transforms::Transform;

use super::add::PyAnchoredOptions;
use super::{pack_align, pack_mode};
use crate::binding::prelude::*;

/// An offset-box tree described by Python (see the module documentation),
/// its artists as their handles.
pub(super) struct PyOffsetBox(pub(super) OffsetBox<RawHandle>);

impl PyOffsetBox {
    /// The tree of a box joining the figure of `into`, its artists moved
    /// out of their holders, or out of that figure's free artists (see
    /// `BoxItem::joining`).
    pub(super) fn joining(self, into: &RawHandle) -> PyResult<OffsetBox<BoxItem>> {
        Ok(self
            .0
            .try_map(&mut |artist| BoxItem::joining(&artist, into))?)
    }
}

impl FromPy<'_> for PyOffsetBox {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let parts: Vec<Bound<'_>> = object.extract()?;
        let (Some(kind), Some(visible)) = (parts.first(), parts.get(1)) else {
            return Err(PyErr::value_error(
                "an offset box description without its kind",
            ));
        };
        let kind: String = kind.extract()?;
        let mut tree = match (kind.as_str(), &parts[2..]) {
            ("text", [text, multilinebaseline]) => {
                OffsetBox::TextArea(TextArea::new(text.extract()?, multilinebaseline.extract()?))
            }
            ("drawing", [width, height, xdescent, ydescent, clip, artists]) => {
                let mut area = DrawingArea::new(
                    width.extract()?,
                    height.extract()?,
                    xdescent.extract()?,
                    ydescent.extract()?,
                );
                area.clip = clip.extract()?;
                area.artists = artists.extract()?;
                OffsetBox::DrawingArea(area)
            }
            ("aux", [transform, artists]) => {
                OffsetBox::AuxTransformBox(artists.extract::<Vec<RawHandle>>()?.into_iter().fold(
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
            ("padded", [pad, child, frame]) => OffsetBox::PaddedBox(PaddedBox {
                pad: pad.extract()?,
                child: Box::new(child.extract::<PyOffsetBox>()?.0),
                frame: frame.extract()?,
                visible: true,
            }),
            ("anchored", [child, options, frame]) => OffsetBox::Anchored(AnchoredNode::new(
                child.extract::<PyOffsetBox>()?.0,
                &options.extract::<PyAnchoredOptions>()?.0,
                frame.extract()?,
            )),
            ("image", [image, zoom, dpi_cor]) => OffsetBox::Image(OffsetImage {
                image: image.extract()?,
                zoom: zoom.extract()?,
                dpi_cor: dpi_cor.extract()?,
                visible: true,
            }),
            ("program", [program]) => OffsetBox::Program(ProgramBox::new(program.extract()?, true)),
            _ => {
                return Err(PyErr::value_error(format!(
                    "a malformed offset box description of kind {kind:?}"
                )));
            }
        };
        tree.set_visible(visible.extract()?);
        Ok(Self(tree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};
    use pltrs::artist::{ArtistHandle, Custom};
    use pltrs::offsetbox::ProgramBoxArtist;
    use pltrs::text::Text;

    #[test]
    fn descriptions_take_their_loose_artists() {
        with_python(|py| {
            extension(py);
            let text = Text::loose(0.0, 0.0, "a");
            let child = ("text", false, text.raw().clone(), false)
                .into_py(py)
                .unwrap();
            let packed = vec![
                "vpacker".into_py(py).unwrap(),
                true.into_py(py).unwrap(),
                0.0.into_py(py).unwrap(),
                2.0.into_py(py).unwrap(),
                Bound::none(py),
                Bound::none(py),
                "center".into_py(py).unwrap(),
                "fixed".into_py(py).unwrap(),
                vec![child].into_py(py).unwrap(),
            ]
            .into_py(py)
            .unwrap();
            let tree = packed
                .extract::<PyOffsetBox>()
                .unwrap()
                .joining(text.raw())
                .unwrap();
            let OffsetBox::VPacker(packer) = &tree else {
                panic!("a vertical packer, not {tree:?}");
            };
            assert!(!packer.children[0].is_visible(), "a hidden text area");
            assert_eq!(tree.all_artists().len(), 1);
            assert!(!text.raw().is_attached(), "the text left its holder");
            let unknown = eval(py, "", "('frame', True, 1.0)");
            assert!(unknown.extract::<PyOffsetBox>().is_err());
            assert!(eval(py, "", "('text',)").extract::<PyOffsetBox>().is_err());
        });
    }

    #[test]
    fn boxes_of_the_program_are_leaves_of_their_own_artist() {
        with_python(|py| {
            extension(py);
            let plain = Custom::loose(ProgramBoxArtist::default());
            let description = ("program", true, plain.raw().clone()).into_py(py).unwrap();
            let tree = description
                .extract::<PyOffsetBox>()
                .unwrap()
                .joining(plain.raw())
                .unwrap();
            assert!(matches!(tree, OffsetBox::Program(_)), "{tree:?}");
            assert_eq!(tree.artists().len(), 1);
            assert!(!plain.raw().is_attached(), "the artist left its holder");
        });
    }
}
