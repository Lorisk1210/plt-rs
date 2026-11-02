//! Putting labels on the lines of one contour set: automatically along
//! every labelled level, or on the line nearest to a point (matplotlib's
//! `labels` and `add_label_near`), cutting the lines under inline labels.

use std::rc::Rc;

use pltrs_core::{Path, Point};
use pltrs_text::{HAlign, VAlign};

use super::place;
use super::split::{LabelCut, LabelGeometry, LabelSplitter, split_path};
use super::{LabelRecord, LevelLabel};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::colors::Color;
use crate::contour::nearest::{closest_on_lines, components};
use crate::contour::{ContourError, ContourSet};
use crate::fonts::FontProperties;
use crate::text::{Text, TextArtist};
use crate::transforms::{ResolvedTransform, Transform, TransformError};

/// The font and z-order of the label texts.
#[derive(Clone, Debug)]
pub(crate) struct TextStyle {
    pub(super) font: FontProperties,
    pub(super) zorder: f64,
    /// Whether the rotations are kept in data coordinates
    /// (`use_clabeltext`).
    pub(super) rotates_with_data: bool,
    pub(super) rc: Rc<crate::rc::RcParams>,
}

pltrs_core::archive_struct!(TextStyle {
    font,
    zorder,
    rotates_with_data,
    rc
});

/// How the last `clabel` call labels the set, which later labels added
/// near a point follow (matplotlib's `labelIndiceList`, `labelFmt`,
/// `labelMappable` and friends).
#[derive(Clone, Debug)]
pub(crate) struct LabelSetup {
    /// The labelled levels with their texts and colours.
    pub(super) labels: Vec<LevelLabel>,
    pub(super) style: TextStyle,
    /// Keep labels upright.
    pub(super) rightside_up: bool,
}

pltrs_core::archive_struct!(LabelSetup {
    labels,
    style,
    rightside_up
});

impl LabelSetup {
    /// The label widths in pixels, per labelled level.
    pub(super) fn widths(&self) -> Vec<f64> {
        self.labels.iter().map(|l| l.width).collect()
    }
}

/// Places labels on one contour set.
pub(super) struct Placer<'s> {
    set: &'s ContourSet,
    setup: &'s LabelSetup,
    geometry: LabelGeometry,
    /// Display to data coordinates, where the label texts go.
    display_to_data: ResolvedTransform,
    /// Display positions of the labels placed so far, which new automatic
    /// labels keep away from.
    positions: Vec<Point>,
    created: Vec<Text>,
}

impl<'s> Placer<'s> {
    /// A placer labelling `set` as `setup` says, in the axes' current
    /// geometry.
    pub(super) fn new(set: &'s ContourSet, setup: &'s LabelSetup) -> Result<Self, ContourError> {
        // The geometry first: it brings the view limits up to date.
        let geometry = LabelGeometry::of(set, setup.rightside_up)?;
        let data_to_display = {
            let figure = set.axes().raw().figure_state().borrow();
            Transform::Data(set.axes().id()).resolve(&*figure)?
        };
        Ok(Self {
            set,
            setup,
            geometry,
            display_to_data: data_to_display.inverted().ok_or(TransformError::Singular)?,
            positions: Vec::new(),
            created: Vec::new(),
        })
    }

    /// The same placer, keeping new automatic labels away from the labels
    /// the set has already (matplotlib's `labelXYs`).
    pub(super) fn after_labels(mut self) -> Self {
        self.positions = self
            .set
            .read(|state| state.labels.iter().map(|label| label.position).collect());
        self
    }

    /// The display position of `point`, in the coordinates of the lines.
    fn to_display(&self, point: Point) -> Point {
        self.geometry.to_display.transform_point(point)
    }

    /// The texts added so far.
    pub(super) fn into_created(self) -> Vec<Text> {
        self.created
    }

    /// The rotation of the label of `cut` and its line cut around it: by
    /// `splitter`, else by pltrs.
    fn split(&self, cut: &LabelCut<'_>, splitter: Option<&mut LabelSplitter<'_>>) -> (f64, Path) {
        match splitter {
            Some(split) => split(*cut),
            None => split_path(&self.geometry, cut),
        }
    }

    /// Automatic placement (matplotlib's `labels`), the lines cut by
    /// `splitter` when given.
    pub(super) fn label_all(
        &mut self,
        inline: bool,
        spacing: f64,
        mut splitter: Option<&mut LabelSplitter<'_>>,
    ) {
        let mut paths = self.set.paths();
        for label in &self.setup.labels {
            let mut additions = Vec::new();
            for subpath in components(&paths[label.index]) {
                let screen: Vec<Point> = subpath
                    .vertices()
                    .iter()
                    .map(|&p| self.to_display(p))
                    .collect();
                if !place::print_label(&screen, label.width) {
                    additions.push(subpath);
                    continue;
                }
                let (at, index) = place::locate_label(&screen, label.width, &self.positions);
                let request = LabelCut {
                    path: &subpath,
                    index,
                    position: at,
                    width: label.width,
                    spacing,
                };
                let (rotation, cut) = self.split(&request, splitter.as_deref_mut());
                self.add(label, at, rotation);
                additions.push(if inline { cut } else { subpath });
            }
            if inline {
                paths[label.index] = Path::compound(&additions);
            }
        }
        if inline {
            self.set.set_paths(paths);
        }
    }

    /// Labels the labelled line nearest to display point `target` and
    /// returns the new text (matplotlib's `add_label_near`), the line cut
    /// by `splitter` when given.
    pub(super) fn label_near(
        &mut self,
        target: Point,
        inline: bool,
        spacing: f64,
        splitter: Option<&mut LabelSplitter<'_>>,
    ) -> Result<Text, ContourError> {
        let mut paths = self.set.paths();
        let setup = self.setup;
        let labelled = setup
            .labels
            .iter()
            .enumerate()
            .map(|(k, label)| (k, &paths[label.index]));
        let closest = closest_on_lines(labelled, |p| self.to_display(p), target)
            .ok_or(ContourError::NoContourToLabel)?;
        let label = &setup.labels[closest.key];
        let request = LabelCut {
            path: &paths[label.index],
            index: closest.vertex,
            position: closest.point,
            width: label.width,
            spacing,
        };
        let (rotation, cut) = self.split(&request, splitter);
        let text = self.add(label, closest.point, rotation);
        if inline {
            paths[label.index] = cut;
            self.set.set_paths(paths);
        }
        Ok(text)
    }

    /// Adds the text artist of a label centred at display point `at`,
    /// turned by `rotation` degrees in the display, and returns it
    /// (matplotlib's `add_label`).
    pub(super) fn add(&mut self, label: &LevelLabel, at: Point, rotation: f64) -> Text {
        let style = &self.setup.style;
        let axes = self.set.axes().clone();
        let data = self.display_to_data.transform_point(at);
        let mut artist = TextArtist::new(data, label.text.clone(), Rc::clone(&style.rc));
        artist.font = style.font.clone();
        if style.rotates_with_data {
            artist.rotation = self.display_to_data.transform_angle(rotation, at);
            artist.transform_rotates_text = true;
        } else {
            artist.rotation = rotation;
        }
        artist.halign = HAlign::Center;
        artist.valign = VAlign::Center;
        artist.color = Color::from(label.color);
        let base = artist.base_mut();
        base.zorder = style.zorder;
        base.transform = Some(axes.trans_data());
        base.clip_box = Some(axes.id());
        let figure = Rc::clone(axes.raw().figure_state());
        let id = figure.borrow_mut().adopt(axes.id(), Box::new(artist));
        self.positions.push(at);
        self.set.update(|state| {
            state.labels.push(LabelRecord {
                level: label.level,
                text: label.text.clone(),
                position: at,
                rotation: pltrs_core::numeric::remainder(rotation, 360.0),
                color: label.color,
                artist: id,
                index: label.index,
                follows: label.follows,
            });
        });
        let text = Text::from_raw(RawHandle::new(figure, id));
        self.created.push(text.clone());
        text
    }
}
