//! Labelling line contours (matplotlib's `ContourLabeler`).
//!
//! Automatic placement walks every labelled level's lines in the order the
//! generator traced them, puts a label on each line long enough for it at
//! its straightest stretch (skipping spots too close to earlier labels),
//! and with `inline` cuts the line around the label. Manual placement
//! labels the nearest line point to each given position, and
//! [`ContourSet::add_label_near`] adds such labels later.

mod place;
mod placer;
mod split;

use plt_core::{Point, Rgba};

use self::placer::{Placer, TextStyle};
use super::{ContourError, ContourSet};
use crate::artist::{ArtistHandle, ArtistId, LayoutContext};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::text::{Text, TextArtist};
use crate::ticker::pyfmt::PrintfFormat;
use crate::ticker::{Formatter, OffsetMode, ScalarFormatter, TickContext, TickerSettings};
use crate::transforms::Transform;

pub(crate) use placer::LabelSetup;

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
        let setup = LabelSetup {
            labels,
            style: TextStyle {
                font,
                zorder: options.zorder.unwrap_or(2.0 + self.zorder()),
                rc,
            },
            rightside_up: options.rightside_up,
        };
        self.update(|state| state.label_setup = Some(setup.clone()));
        let spacing = options.inline_spacing;
        let mut placer = Placer::new(self, &setup)?;
        match &options.manual {
            Some(points) => {
                for &(x, y) in points {
                    let target = placer.to_display(Point::new(x, y));
                    placer.label_near(target, options.inline, spacing)?;
                }
            }
            None => placer.label_all(options.inline, spacing),
        }
        Ok(placer.into_created())
    }

    /// Labels the labelled line nearest to `point`, given in the
    /// coordinates of `transform` (data: the axes' `trans_data`; display
    /// pixels: [`Transform::Display`]), the way the last
    /// [`ContourSet::clabel`] call labelled the set, and returns the new
    /// text (matplotlib's `add_label_near`). With `inline` the line is cut
    /// under the label, leaving `inline_spacing` pixels on each side.
    ///
    /// # Errors
    ///
    /// When the set has not been labelled yet, `transform` cannot be
    /// resolved, or there is no labelled line to snap to.
    pub fn add_label_near(
        &self,
        point: Point,
        transform: &Transform,
        inline: bool,
        inline_spacing: f64,
    ) -> Result<Text, ContourError> {
        let setup = self
            .read(|state| state.label_setup.clone())
            .ok_or(ContourError::Unlabelled)?;
        let mut placer = Placer::new(self, &setup)?;
        let target = {
            let figure = self.axes().raw().figure_state().borrow();
            transform.resolve(&*figure)?.transform_point(point)
        };
        placer.label_near(target, inline, inline_spacing)?;
        placer
            .into_created()
            .pop()
            .ok_or(ContourError::NoContourToLabel)
    }

    /// The widths in pixels of the labels of the levels last labelled,
    /// as used for placement.
    pub fn label_widths(&self) -> Vec<f64> {
        self.read(|state| {
            state
                .label_setup
                .as_ref()
                .map(LabelSetup::widths)
                .unwrap_or_default()
        })
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

    #[test]
    fn labels_added_near_a_point_follow_the_last_clabel() {
        let cs = bowl(&[0.5, 1.0]);
        let data = cs.axes().trans_data();
        assert_eq!(
            cs.add_label_near(Point::new(0.7, 0.0), &data, true, 5.0)
                .unwrap_err(),
            ContourError::Unlabelled
        );
        let options = ClabelOptions {
            fmt: LabelFormat::Map(vec![(0.5, "half".to_owned()), (1.0, "one".to_owned())]),
            ..ClabelOptions::default()
        };
        let automatic = cs.clabel(&options).unwrap().len();
        let near = cs
            .add_label_near(Point::new(0.0, 0.98), &data, true, 5.0)
            .unwrap();
        let labels = cs.labels();
        assert_eq!(labels.len(), automatic + 1);
        let last = &labels[labels.len() - 1];
        assert_eq!((last.level, last.text.as_str()), (1.0, "one"));
        assert_eq!(last.artist, near);
        // A display point labels the line nearest to it on screen.
        let display = data
            .resolve(&*cs.axes().raw().figure_state().borrow())
            .unwrap()
            .transform_point(Point::new(0.7, 0.0));
        cs.add_label_near(display, &Transform::Display, false, 5.0)
            .unwrap();
        assert_eq!(cs.labels().last().unwrap().level, 0.5);
    }
}
