//! Putting labels on the lines of one contour set: automatically along
//! every labelled level, or on the line nearest to a point (matplotlib's
//! `labels` and `add_label_near`), cutting the lines under inline labels.

use std::rc::Rc;

use plt_core::{Path, Point};
use plt_text::{HAlign, VAlign};

use super::place;
use super::split::{LabelGeometry, split_path};
use super::{LabelRecord, LevelLabel};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::colors::Color;
use crate::contour::{ContourError, ContourSet};
use crate::fonts::FontProperties;
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;

/// The font and z-order of the label texts.
#[derive(Clone, Debug)]
pub(crate) struct TextStyle {
    pub(super) font: FontProperties,
    pub(super) zorder: f64,
    pub(super) rc: Rc<crate::rc::RcParams>,
}

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
    /// Display positions of the labels placed so far, which new automatic
    /// labels keep away from.
    positions: Vec<Point>,
    created: Vec<Text>,
}

impl<'s> Placer<'s> {
    /// A placer labelling `set` as `setup` says, in the axes' current
    /// geometry.
    pub(super) fn new(set: &'s ContourSet, setup: &'s LabelSetup) -> Result<Self, ContourError> {
        let axes = set.axes();
        axes.unstale_view();
        let figure = axes.raw().figure_state().borrow();
        let to_display = Transform::Data(axes.id()).resolve(&*figure)?;
        let to_data = to_display
            .inverted()
            .ok_or(crate::transforms::TransformError::Singular)?;
        Ok(Self {
            set,
            setup,
            geometry: LabelGeometry {
                to_display,
                to_data,
                rightside_up: setup.rightside_up,
            },
            positions: Vec::new(),
            created: Vec::new(),
        })
    }

    /// The display position of data point `data`.
    pub(super) fn to_display(&self, data: Point) -> Point {
        self.geometry.to_display.transform_point(data)
    }

    /// The texts added so far.
    pub(super) fn into_created(self) -> Vec<Text> {
        self.created
    }

    /// Automatic placement (matplotlib's `labels`).
    pub(super) fn label_all(&mut self, inline: bool, spacing: f64) {
        let mut paths = self.set.paths();
        for label in &self.setup.labels {
            let mut additions = Vec::new();
            for subpath in components(&paths[label.index]) {
                let screen: Vec<Point> = subpath
                    .vertices()
                    .iter()
                    .map(|&p| self.to_display(p))
                    .collect();
                if !place::long_enough(&screen, label.width) {
                    additions.push(subpath);
                    continue;
                }
                let (at, index) = place::locate(&screen, label.width, &self.positions);
                let (rotation, cut) =
                    split_path(&self.geometry, &subpath, index, at, (label.width, spacing));
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

    /// Labels the labelled line nearest to display point `target`
    /// (matplotlib's `add_label_near`).
    pub(super) fn label_near(
        &mut self,
        target: Point,
        inline: bool,
        spacing: f64,
    ) -> Result<(), ContourError> {
        let mut paths = self.set.paths();
        let mut best: Option<(f64, usize, usize, Point)> = None;
        for (k, label) in self.setup.labels.iter().enumerate() {
            let mut start = 0;
            for subpath in components(&paths[label.index]) {
                let screen: Vec<Point> = subpath
                    .vertices()
                    .iter()
                    .map(|&p| self.to_display(p))
                    .collect();
                if let Some((d2, proj, leg)) = place::closest_point(&screen, target)
                    && best.is_none_or(|(min, ..)| d2 < min)
                {
                    best = Some((d2, k, start + leg, proj));
                }
                start += screen.len();
            }
        }
        let (_, k, vertex, proj) = best.ok_or(ContourError::NoContourToLabel)?;
        let label = &self.setup.labels[k];
        let (rotation, cut) = split_path(
            &self.geometry,
            &paths[label.index],
            vertex,
            proj,
            (label.width, spacing),
        );
        self.add(label, proj, rotation);
        if inline {
            paths[label.index] = cut;
            self.set.set_paths(paths);
        }
        Ok(())
    }

    /// Adds the text artist of a label centred at display point `at`.
    fn add(&mut self, label: &LevelLabel, at: Point, rotation: f64) {
        let style = &self.setup.style;
        let axes = self.set.axes().clone();
        let data = self.geometry.to_data.transform_point(at);
        let mut artist = TextArtist::new(data, label.text.clone(), Rc::clone(&style.rc));
        artist.font = style.font.clone();
        artist.rotation = rotation;
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
                rotation: crate::ticker::pymath::py_mod(rotation, 360.0),
                color: label.color,
                artist: id,
                index: label.index,
                follows: label.follows,
            });
        });
        self.created
            .push(Text::from_raw(RawHandle::new(figure, id)));
    }
}

/// The connected components of a path (matplotlib's
/// `_iter_connected_components`): a path without codes is one component.
fn components(path: &Path) -> Vec<Path> {
    if path.codes().is_none() {
        vec![path.clone()]
    } else {
        path.subpaths()
    }
}
