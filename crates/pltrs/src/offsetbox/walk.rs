//! Walking an offset-box tree: its boxes in tree order, the artists its
//! leaves hold, and the same tree referring to its artists another way.
//!
//! *Tree order* lists a box before the boxes inside it, a packer's
//! children in their order (matplotlib's recursion over `get_children`);
//! [`OffsetBox::node`] reaches a box by its place in it.

use std::convert::Infallible;

use super::leaves::{AuxTransformBox, DrawingArea, OffsetImage, PaddedBox, TextArea};
use super::program::ProgramBox;
use super::{OffsetBox, Packer};

impl<A> OffsetBox<A> {
    /// The artists of the visible boxes in drawing order: frames before
    /// what they frame.
    pub fn artists(&self) -> Vec<&A> {
        let mut artists = Vec::new();
        self.visit_artists(false, &mut |artist| artists.push(artist));
        artists
    }

    /// The artists of every box, hidden ones too, in the order of
    /// [`map`](Self::map): frames before what they frame.
    pub fn all_artists(&self) -> Vec<&A> {
        let mut artists = Vec::new();
        self.visit_artists(true, &mut |artist| artists.push(artist));
        artists
    }

    /// Calls `f` on the artists of the tree's boxes, those of hidden
    /// boxes with `hidden`.
    fn visit_artists<'a>(&'a self, hidden: bool, f: &mut impl FnMut(&'a A)) {
        match self {
            Self::DrawingArea(area) => area.artists.iter().for_each(&mut *f),
            Self::TextArea(area) => f(&area.text),
            Self::AuxTransformBox(aux) => aux.artists.iter().for_each(&mut *f),
            Self::Image(image) => f(&image.image),
            Self::PaddedBox(PaddedBox { frame, .. }) => f(frame),
            Self::Anchored(anchored) => f(&anchored.frame),
            Self::Program(program) => f(&program.artist),
            Self::HPacker(_) | Self::VPacker(_) => {}
        }
        for child in self.children() {
            if hidden || child.is_visible() {
                child.visit_artists(hidden, f);
            }
        }
    }

    /// The boxes directly inside this one: a packer's children, the child
    /// of a padded or an anchored box.
    pub fn children(&self) -> Vec<&Self> {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.children.iter().collect(),
            Self::PaddedBox(padded) => vec![&*padded.child],
            Self::Anchored(anchored) => vec![&*anchored.child],
            Self::DrawingArea(_)
            | Self::TextArea(_)
            | Self::AuxTransformBox(_)
            | Self::Image(_)
            | Self::Program(_) => Vec::new(),
        }
    }

    /// The boxes directly inside this one, mutably.
    fn children_mut(&mut self) -> Vec<&mut Self> {
        match self {
            Self::HPacker(packer) | Self::VPacker(packer) => packer.children.iter_mut().collect(),
            Self::PaddedBox(padded) => vec![&mut *padded.child],
            Self::Anchored(anchored) => vec![&mut *anchored.child],
            Self::DrawingArea(_)
            | Self::TextArea(_)
            | Self::AuxTransformBox(_)
            | Self::Image(_)
            | Self::Program(_) => Vec::new(),
        }
    }

    /// How many boxes the tree has, this one included.
    pub fn box_count(&self) -> usize {
        1 + self
            .children()
            .iter()
            .map(|child| child.box_count())
            .sum::<usize>()
    }

    /// The box at `index` in tree order (the box itself at 0).
    ///
    /// ```
    /// use pltrs::offsetbox::{BoxItem, DrawingArea, OffsetBox, PackAlign, Packer, TextArea};
    /// use pltrs::text::TextProps;
    /// let tree = OffsetBox::VPacker(Packer::new(0.0, 0.0, PackAlign::Left, vec![
    ///     TextArea::from_text("a", TextProps::default()).into(),
    ///     DrawingArea::<BoxItem>::new(10.0, 10.0, 0.0, 0.0).into(),
    /// ]));
    /// assert!(matches!(tree.node(2), Some(OffsetBox::DrawingArea(_))));
    /// assert_eq!(tree.box_count(), 3);
    /// assert!(tree.node(3).is_none());
    /// ```
    pub fn node(&self, index: usize) -> Option<&Self> {
        if index == 0 {
            return Some(self);
        }
        let mut start = 1;
        for child in self.children() {
            let count = child.box_count();
            if index < start + count {
                return child.node(index - start);
            }
            start += count;
        }
        None
    }

    /// The same tree with every artist reference replaced by `f` of it,
    /// in the order of [`all_artists`](Self::all_artists) (how a box's
    /// artists join a figure).
    pub fn map<B>(self, f: &mut impl FnMut(A) -> B) -> OffsetBox<B> {
        match self.try_map(&mut |artist| Ok::<_, Infallible>(f(artist))) {
            Ok(tree) => tree,
            Err(never) => match never {},
        }
    }

    /// [`map`](Self::map) with a function that may fail, which stops at
    /// the first failure.
    ///
    /// # Errors
    ///
    /// The first error `f` returns.
    pub fn try_map<B, E>(self, f: &mut impl FnMut(A) -> Result<B, E>) -> Result<OffsetBox<B>, E> {
        Ok(match self {
            Self::HPacker(packer) => OffsetBox::HPacker(packer.try_map(f)?),
            Self::VPacker(packer) => OffsetBox::VPacker(packer.try_map(f)?),
            Self::DrawingArea(area) => OffsetBox::DrawingArea(DrawingArea {
                width: area.width,
                height: area.height,
                xdescent: area.xdescent,
                ydescent: area.ydescent,
                clip: area.clip,
                artists: area
                    .artists
                    .into_iter()
                    .map(&mut *f)
                    .collect::<Result<_, _>>()?,
                visible: area.visible,
            }),
            Self::TextArea(area) => OffsetBox::TextArea(TextArea {
                text: f(area.text)?,
                multilinebaseline: area.multilinebaseline,
                visible: area.visible,
            }),
            Self::AuxTransformBox(aux) => OffsetBox::AuxTransformBox(AuxTransformBox {
                aux_transform: aux.aux_transform,
                artists: aux
                    .artists
                    .into_iter()
                    .map(&mut *f)
                    .collect::<Result<_, _>>()?,
                visible: aux.visible,
                extent: aux.extent,
            }),
            Self::Image(image) => OffsetBox::Image(OffsetImage {
                image: f(image.image)?,
                zoom: image.zoom,
                dpi_cor: image.dpi_cor,
                visible: image.visible,
            }),
            Self::PaddedBox(padded) => {
                let frame = f(padded.frame)?;
                OffsetBox::PaddedBox(PaddedBox {
                    pad: padded.pad,
                    child: Box::new(padded.child.try_map(f)?),
                    frame,
                    visible: padded.visible,
                })
            }
            Self::Anchored(anchored) => OffsetBox::Anchored(anchored.try_map(f)?),
            Self::Program(program) => OffsetBox::Program(program.try_map(f)?),
        })
    }

    /// Anchors the anchored boxes of the tree in `owner`, the axes or
    /// figure the tree joined (matplotlib's `OffsetBox.axes` setter,
    /// which gives the box's axes to its children).
    pub(crate) fn join_owner(&mut self, owner: crate::artist::ArtistId) {
        self.for_each_box_mut(&mut |tree| {
            if let Self::Anchored(anchored) = tree {
                anchored.owner = Some(owner);
            }
        });
    }

    /// The drawing area at `index` among those of the tree, visible or not,
    /// in tree order.
    pub fn drawing_area_mut(&mut self, index: usize) -> Option<&mut DrawingArea<A>> {
        let mut areas = Vec::new();
        self.collect_leaves(
            &|tree| match tree {
                Self::DrawingArea(area) => Some(area),
                _ => None,
            },
            &mut areas,
        );
        areas.into_iter().nth(index)
    }

    /// The offset image at `index` among those of the tree, visible or
    /// not, in tree order (as [`drawing_area_mut`](Self::drawing_area_mut)
    /// counts drawing areas).
    pub fn image_mut(&mut self, index: usize) -> Option<&mut OffsetImage<A>> {
        let mut images = Vec::new();
        self.collect_leaves(
            &|tree| match tree {
                Self::Image(image) => Some(image),
                _ => None,
            },
            &mut images,
        );
        images.into_iter().nth(index)
    }

    /// The leaves of the tree `leaf` picks, in tree order.
    fn collect_leaves<'a, T>(
        &'a mut self,
        leaf: &impl Fn(&'a mut Self) -> Option<&'a mut T>,
        found: &mut Vec<&'a mut T>,
    ) {
        if matches!(
            self,
            Self::HPacker(_) | Self::VPacker(_) | Self::PaddedBox(_) | Self::Anchored(_)
        ) {
            for child in self.children_mut() {
                child.collect_leaves(leaf, found);
            }
        } else {
            found.extend(leaf(self));
        }
    }

    /// Calls `f` on every aux-transform box of the tree.
    pub(crate) fn for_each_aux_mut(&mut self, f: &mut impl FnMut(&mut AuxTransformBox<A>)) {
        self.for_each_box_mut(&mut |tree| {
            if let Self::AuxTransformBox(aux) = tree {
                f(aux);
            }
        });
    }

    /// Calls `f` on every box of the program's of the tree.
    pub(crate) fn for_each_program_mut(&mut self, f: &mut impl FnMut(&mut ProgramBox<A>)) {
        self.for_each_box_mut(&mut |tree| {
            if let Self::Program(program) = tree {
                f(program);
            }
        });
    }

    /// Calls `f` on every box of the tree, in tree order.
    fn for_each_box_mut(&mut self, f: &mut impl FnMut(&mut Self)) {
        f(self);
        for child in self.children_mut() {
            child.for_each_box_mut(f);
        }
    }
}

impl<A> Packer<A> {
    /// The children taking part in the packer's layout.
    pub(super) fn visible_children(&self) -> impl Iterator<Item = &OffsetBox<A>> {
        self.children.iter().filter(|c| c.is_visible())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsetbox::{BoxItem, PackAlign};
    use crate::patches::PatchProps;
    use crate::text::TextProps;

    /// A padded packer of a text and a hidden area holding two artists.
    fn tree() -> OffsetBox<BoxItem> {
        let mut area = OffsetBox::from(
            DrawingArea::new(1.0, 1.0, 0.0, 0.0)
                .with_artist(BoxItem::patch(
                    crate::patches::Rectangle::new((0.0, 0.0), 1.0, 1.0),
                    PatchProps::default(),
                ))
                .with_artist(BoxItem::patch(
                    crate::patches::Circle::new((0.0, 0.0), 1.0),
                    PatchProps::default(),
                )),
        );
        area.set_visible(false);
        let packer = OffsetBox::HPacker(Packer::new(
            0.0,
            0.0,
            PackAlign::Center,
            vec![TextArea::from_text("t", TextProps::default()).into(), area],
        ));
        PaddedBox::new(packer, 1.0, true, PatchProps::default()).into()
    }

    #[test]
    fn hidden_boxes_keep_their_artists_and_places() {
        let tree = tree().map(&mut |_| ());
        assert_eq!(tree.artists().len(), 2, "the frame and the text");
        assert_eq!(tree.all_artists().len(), 4, "and the hidden area's");
        assert_eq!(tree.box_count(), 4);
        assert!(matches!(tree.node(1), Some(OffsetBox::HPacker(_))));
        assert!(matches!(tree.node(3), Some(OffsetBox::DrawingArea(_))));
    }

    #[test]
    fn mapping_visits_frames_first_and_stops_at_failures() {
        let mut seen = 0;
        let numbered = tree().map(&mut |_| {
            seen += 1;
            seen
        });
        assert_eq!(numbered.all_artists(), [&1, &2, &3, &4]);
        let failed = numbered.try_map(&mut |n| if n == 3 { Err(n) } else { Ok(n) });
        assert_eq!(failed.err(), Some(3));
    }
}
