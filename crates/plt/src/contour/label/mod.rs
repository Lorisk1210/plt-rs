//! Labelling line contours (matplotlib's `ContourLabeler`).
//!
//! Automatic placement walks every labelled level's lines in the order the
//! generator traced them, puts a label on each line long enough for it at
//! its straightest stretch (skipping spots too close to earlier labels),
//! and with `inline` cuts the line around the label. Manual placement
//! labels the nearest line point to each given position.

mod place;
mod split;

use std::rc::Rc;

use plt_core::{Path, Point, Rgba};
use plt_text::{HAlign, VAlign};

use self::split::{LabelGeometry, split_path};
use super::{ContourError, ContourSet};
use crate::artist::{Artist, ArtistHandle, ArtistId, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::text::{Text, TextArtist};
use crate::ticker::pyfmt::PrintfFormat;
use crate::ticker::{Formatter, OffsetMode, ScalarFormatter, TickContext, TickerSettings};
use crate::transforms::Transform;

/// How label texts are made from levels.
#[derive(Debug, Default)]
pub enum LabelFormat {
    /// matplotlib's default: a `ScalarFormatter` without offset applied to
    /// all labelled levels at once.
    #[default]
    Scalar,
    /// A printf-style format such as `"%1.2f m"`.
    Printf(PrintfFormat),
    /// A text per level; other levels get the literal text `%1.3f`, as in
    /// matplotlib.
    Map(Vec<(f64, String)>),
    /// Any tick formatter, applied to all labelled levels at once.
    Formatter(Box<dyn Formatter>),
}

/// Options of [`ContourSet::clabel`] (matplotlib's `clabel` keywords).
#[derive(Debug)]
pub struct ClabelOptions {
    /// The levels to label (a subset of the set's levels; default all).
    pub levels: Option<Vec<f64>>,
    /// Label size (default `font.size`).
    pub fontsize: Option<FontSize>,
    /// Cut the lines under the labels.
    pub inline: bool,
    /// Pixels left free on each side of an inline label.
    pub inline_spacing: f64,
    /// How labels are written.
    pub fmt: LabelFormat,
    /// Label colours cycled over the labelled levels (default: the
    /// colour of each level).
    pub colors: Option<Vec<Color>>,
    /// Data positions to label the nearest line at, instead of automatic
    /// placement.
    pub manual: Option<Vec<(f64, f64)>>,
    /// Keep labels upright (rotations within ±90°).
    pub rightside_up: bool,
    /// Label z-order (default: two above the contours).
    pub zorder: Option<f64>,
}

impl Default for ClabelOptions {
    fn default() -> Self {
        Self {
            levels: None,
            fontsize: None,
            inline: true,
            inline_spacing: 5.0,
            fmt: LabelFormat::Scalar,
            colors: None,
            manual: None,
            rightside_up: true,
            zorder: None,
        }
    }
}

/// One contour label.
#[derive(Clone, Debug, PartialEq)]
pub struct ContourLabel {
    /// The labelled level.
    pub level: f64,
    /// The label text.
    pub text: String,
    /// The label centre in display pixels.
    pub position: Point,
    /// Counter-clockwise rotation in degrees, in `[0, 360)`.
    pub rotation: f64,
    /// Text colour.
    pub color: Rgba,
    /// The text artist.
    pub artist: Text,
}

/// What a contour set remembers of one of its labels: a [`ContourLabel`]
/// with the text artist's id (a handle would keep the figure alive from
/// inside itself).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelRecord {
    pub(crate) level: f64,
    pub(crate) text: String,
    pub(crate) position: Point,
    pub(crate) rotation: f64,
    pub(crate) color: Rgba,
    pub(crate) artist: ArtistId,
    /// Index of the labelled level.
    pub(crate) index: usize,
    /// Whether the label takes the colour of its level, and is recoloured
    /// with it (no explicit label colours).
    pub(crate) follows: bool,
}

impl LabelRecord {
    /// The public view of the label, with `artist` the handle to its text.
    pub(crate) fn label(&self, artist: Text) -> ContourLabel {
        ContourLabel {
            level: self.level,
            text: self.text.clone(),
            position: self.position,
            rotation: self.rotation,
            color: self.color,
            artist,
        }
    }
}

/// What every label of one level shares.
#[derive(Clone, Debug)]
struct LevelLabel {
    /// Index into the set's levels.
    index: usize,
    level: f64,
    text: String,
    /// Text width in pixels, used for placement.
    width: f64,
    color: Rgba,
    /// Whether the colour is that of the level.
    follows: bool,
}

impl ContourSet {
    /// Labels the line contours (matplotlib's `clabel`) and returns the
    /// new text artists.
    ///
    /// # Errors
    ///
    /// When requested levels are not in the set, a format fails, label
    /// text cannot be laid out, or a manual position has no labelled line
    /// to snap to.
    pub fn clabel(&self, options: &ClabelOptions) -> Result<Vec<Text>, ContourError> {
        let rc = crate::rc::params();
        let indices = self.label_indices(options.levels.as_deref())?;
        let font = match options.fontsize {
            Some(size) => FontProperties::from_rc(&rc).with_size(size, rc.font_size),
            None => FontProperties::from_rc(&rc),
        };
        let all_levels = self.levels();
        let levels: Vec<f64> = indices.iter().map(|&i| all_levels[i]).collect();
        let texts = label_texts(&levels, &options.fmt, &TickerSettings::from(&*rc))?;
        let explicit = options.colors.as_deref().filter(|c| !c.is_empty());
        let colors = self.label_colors(&indices, explicit)?;
        let mut labels = Vec::with_capacity(indices.len());
        for (k, &index) in indices.iter().enumerate() {
            labels.push(LevelLabel {
                index,
                level: levels[k],
                width: self.text_width(&texts[k], &font)?,
                text: texts[k].clone(),
                color: colors[k],
                follows: explicit.is_none(),
            });
        }
        self.update(|state| state.label_widths = labels.iter().map(|l| l.width).collect());
        let geometry = self.label_geometry(options.rightside_up)?;
        let style = TextStyle {
            font,
            zorder: options.zorder.unwrap_or(2.0 + self.zorder()),
            rc,
        };
        let spacing = options.inline_spacing;
        let mut placer = Placer {
            set: self,
            geometry,
            style,
            positions: Vec::new(),
            created: Vec::new(),
        };
        match &options.manual {
            Some(points) => {
                for &(x, y) in points {
                    placer.label_near(Point::new(x, y), &labels, options.inline, spacing)?;
                }
            }
            None => placer.label_all(&labels, options.inline, spacing),
        }
        Ok(placer.created)
    }

    /// The widths in pixels of the labels of the levels last labelled,
    /// as used for placement.
    pub fn label_widths(&self) -> Vec<f64> {
        self.read(|state| state.label_widths.clone())
    }

    /// Indices of the levels to label: all, or those listed.
    fn label_indices(&self, requested: Option<&[f64]>) -> Result<Vec<usize>, ContourError> {
        let levels = self.levels();
        let Some(requested) = requested else {
            return Ok((0..levels.len()).collect());
        };
        let indices: Vec<usize> = (0..levels.len())
            .filter(|&i| requested.contains(&levels[i]))
            .collect();
        if indices.len() < requested.len() {
            return Err(ContourError::UnknownLabelLevels(requested.to_vec()));
        }
        Ok(indices)
    }

    /// The colour of each labelled level: its contour colour, or the
    /// explicit colours cycled.
    fn label_colors(
        &self,
        indices: &[usize],
        colors: Option<&[Color]>,
    ) -> Result<Vec<Rgba>, ContourError> {
        self.read(|state| match colors {
            None => Ok(indices
                .iter()
                .map(|&i| {
                    state
                        .colors
                        .get(i)
                        .map_or(Rgba::TRANSPARENT, |&c| state.with_alpha(c))
                })
                .collect()),
            Some(colors) => colors
                .iter()
                .cycle()
                .take(indices.len())
                .map(|c| Ok(state.with_alpha(c.resolve()?)))
                .collect(),
        })
    }

    /// The width in pixels of `text` laid out like a label (matplotlib's
    /// `_get_nth_label_width`).
    fn text_width(&self, text: &str, font: &FontProperties) -> Result<f64, ContourError> {
        let mut artist = TextArtist::new(Point::ORIGIN, text, crate::rc::params());
        artist.font = font.clone();
        let figure = self.axes().raw().figure_state().borrow();
        Ok(artist
            .layout(LayoutContext::new(&figure))?
            .block
            .bbox
            .width())
    }

    /// The data ↔ display transforms of the axes as they are now.
    fn label_geometry(&self, rightside_up: bool) -> Result<LabelGeometry, ContourError> {
        let axes = self.axes();
        axes.unstale_view();
        let figure = axes.raw().figure_state().borrow();
        let to_display = Transform::Data(axes.id()).resolve(&*figure)?;
        let to_data = to_display
            .inverted()
            .ok_or(crate::transforms::TransformError::Singular)?;
        Ok(LabelGeometry {
            to_display,
            to_data,
            rightside_up,
        })
    }

    fn zorder(&self) -> f64 {
        self.collections()
            .first()
            .map_or(1.0, |c| c.raw().with_base(|b| b.zorder))
    }
}

/// The texts of the labelled `levels`.
fn label_texts(
    levels: &[f64],
    fmt: &LabelFormat,
    settings: &TickerSettings,
) -> Result<Vec<String>, ContourError> {
    let ctx = TickContext::detached(settings);
    // A formatter sees all labelled levels plus the one being labelled.
    let with_formatter = |formatter: &dyn Formatter| {
        levels
            .iter()
            .map(|&level| {
                let mut locs = levels.to_vec();
                locs.push(level);
                let labels = formatter.format_ticks(&locs, &ctx)?.labels;
                Ok(labels.last().cloned().unwrap_or_default())
            })
            .collect::<Result<Vec<_>, ContourError>>()
    };
    match fmt {
        LabelFormat::Scalar => {
            let scalar = ScalarFormatter::from_settings(settings).with_use_offset(OffsetMode::Off);
            with_formatter(&scalar)
        }
        LabelFormat::Formatter(formatter) => with_formatter(formatter.as_ref()),
        LabelFormat::Printf(format) => levels.iter().map(|&l| Ok(format.format(l)?)).collect(),
        #[expect(clippy::float_cmp, reason = "matplotlib looks levels up in a dict")]
        LabelFormat::Map(map) => Ok(levels
            .iter()
            .map(|&level| {
                map.iter()
                    .find(|(key, _)| *key == level)
                    .map_or_else(|| "%1.3f".to_owned(), |(_, text)| text.clone())
            })
            .collect()),
    }
}

/// The font and z-order of the label texts.
#[derive(Debug)]
struct TextStyle {
    font: FontProperties,
    zorder: f64,
    rc: Rc<crate::rc::RcParams>,
}

/// Places labels on one contour set.
struct Placer<'s> {
    set: &'s ContourSet,
    geometry: LabelGeometry,
    style: TextStyle,
    /// Display positions of the labels of this call, which new labels
    /// keep away from.
    positions: Vec<Point>,
    created: Vec<Text>,
}

impl Placer<'_> {
    /// Automatic placement (matplotlib's `labels`).
    fn label_all(&mut self, labels: &[LevelLabel], inline: bool, spacing: f64) {
        let mut paths = self.set.paths();
        for label in labels {
            let mut additions = Vec::new();
            for subpath in components(&paths[label.index]) {
                let screen: Vec<Point> = subpath
                    .vertices()
                    .iter()
                    .map(|&p| self.geometry.to_display.transform_point(p))
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

    /// Labels the labelled line nearest to `data` (matplotlib's
    /// `add_label_near`).
    fn label_near(
        &mut self,
        data: Point,
        labels: &[LevelLabel],
        inline: bool,
        spacing: f64,
    ) -> Result<(), ContourError> {
        let target = self.geometry.to_display.transform_point(data);
        let mut paths = self.set.paths();
        let mut best: Option<(f64, usize, usize, Point)> = None;
        for (k, label) in labels.iter().enumerate() {
            let mut start = 0;
            for subpath in components(&paths[label.index]) {
                let screen: Vec<Point> = subpath
                    .vertices()
                    .iter()
                    .map(|&p| self.geometry.to_display.transform_point(p))
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
        let label = &labels[k];
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
        let axes = self.set.axes().clone();
        let data = self.geometry.to_data.transform_point(at);
        let mut artist = TextArtist::new(data, label.text.clone(), Rc::clone(&self.style.rc));
        artist.font = self.style.font.clone();
        artist.rotation = rotation;
        artist.halign = HAlign::Center;
        artist.valign = VAlign::Center;
        artist.color = Color::from(label.color);
        let base = artist.base_mut();
        base.zorder = self.style.zorder;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::{ContourOptions, Levels, QuadCoords, QuadField};
    use crate::figure::Figure;
    use crate::numeric::{Grid, linspace};

    /// Circles `x² + y² = level` on a 4 × 4 in figure.
    fn bowl(levels: &[f64]) -> ContourSet {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        let x = linspace(-1.0, 1.0, 9);
        let z = Grid::from_fn(9, 9, |i, j| x[i] * x[i] + x[j] * x[j]);
        let field = QuadField::new(z).with_coords(QuadCoords::Axes { x: x.clone(), y: x });
        let options = ContourOptions {
            levels: Levels::Values(levels.to_vec()),
            ..ContourOptions::default()
        };
        let cs = ax.contour(&field, options).unwrap();
        ax.set_xlim(-1.0, 1.0);
        ax.set_ylim(-1.0, 1.0);
        cs
    }

    fn subpath_count(cs: &ContourSet) -> usize {
        cs.paths().iter().map(|p| p.subpaths().len()).sum()
    }

    #[test]
    fn automatic_labels_are_upright_and_cut_the_lines() {
        let cs = bowl(&[0.5, 1.0]);
        let before = subpath_count(&cs);
        let texts = cs.clabel(&ClabelOptions::default()).unwrap();
        assert_eq!(texts.len(), cs.labels().len());
        assert_ne!(texts.len(), 0);
        assert!(
            cs.labels()
                .iter()
                .all(|l| l.rotation <= 90.0 || l.rotation >= 270.0)
        );
        assert_eq!(cs.labels()[0].text, "0.5");
        assert_eq!(cs.label_widths().len(), 2);
        assert!(
            subpath_count(&cs) >= before,
            "loops are opened, open lines split"
        );
        assert!(
            cs.paths()[0]
                .codes()
                .unwrap()
                .iter()
                .all(|&c| c != plt_core::PathCode::ClosePoly)
        );
    }

    #[test]
    fn manual_labels_snap_to_the_nearest_line() {
        let cs = bowl(&[0.5, 1.0]);
        let before = subpath_count(&cs);
        let options = ClabelOptions {
            manual: Some(vec![(0.7, 0.0)]),
            inline: false,
            fmt: LabelFormat::Map(vec![(0.5, "half".to_owned())]),
            ..ClabelOptions::default()
        };
        cs.clabel(&options).unwrap();
        let labels = cs.labels();
        let [label] = labels.as_slice() else {
            panic!("one label expected");
        };
        assert_eq!((label.level, label.text.as_str()), (0.5, "half"));
        assert_eq!(subpath_count(&cs), before, "not inline: lines are kept");
    }

    #[test]
    fn label_formats() {
        let settings = TickerSettings::DEFAULT;
        let printf = LabelFormat::Printf(PrintfFormat::parse("%1.2f m").unwrap());
        assert_eq!(
            label_texts(&[0.5, -1.0], &printf, &settings).unwrap(),
            ["0.50 m", "-1.00 m"]
        );
        let map = LabelFormat::Map(vec![(1.0, "one".to_owned())]);
        assert_eq!(
            label_texts(&[1.0, 2.0], &map, &settings).unwrap(),
            ["one", "%1.3f"]
        );
        let scalar = label_texts(&[-0.5, 0.0, 0.5], &LabelFormat::Scalar, &settings).unwrap();
        assert_eq!(scalar, ["\u{2212}0.5", "0.0", "0.5"]);
    }

    #[test]
    fn unknown_levels_and_missing_lines_are_errors() {
        let cs = bowl(&[0.5, 1.0]);
        let unknown = ClabelOptions {
            levels: Some(vec![0.7]),
            ..ClabelOptions::default()
        };
        assert_eq!(
            cs.clabel(&unknown).unwrap_err(),
            ContourError::UnknownLabelLevels(vec![0.7])
        );
        let empty = bowl(&[5.0]);
        let manual = ClabelOptions {
            manual: Some(vec![(0.0, 0.0)]),
            ..ClabelOptions::default()
        };
        assert_eq!(
            empty.clabel(&manual).unwrap_err(),
            ContourError::NoContourToLabel
        );
    }
}
