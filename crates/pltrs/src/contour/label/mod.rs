//! Labelling line contours (matplotlib's `ContourLabeler`).
//!
//! Automatic placement walks every labelled level's lines in the order the
//! generator traced them, puts a label on each line long enough for it at
//! its straightest stretch (skipping spots too close to earlier labels),
//! and with `inline` cuts the line around the label. Manual placement
//! labels the nearest line point to each given position, and
//! [`ContourSet::add_label_near`] adds such labels later;
//! [`ContourSet::add_label`] puts one exactly where it is told and
//! [`ContourSet::pop_label`] removes one.

mod added;
mod place;
mod placer;
mod split;

use std::rc::Rc;

use pltrs_core::{Point, Rgba};

use self::placer::{Placer, TextStyle};
use super::{ContourError, ContourSet};
use crate::artist::{ArtistHandle, ArtistId, LayoutContext};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::text::{Text, TextArtist};
use crate::ticker::{
    Formatter, LabelFormat, OffsetMode, ScalarFormatter, TickContext, TickerSettings,
};

pub use place::{locate_label, print_label, too_close};
pub(crate) use placer::LabelSetup;
pub use split::{LabelCut, LabelSplitter};

/// How label texts are made from levels (matplotlib's `fmt` of `clabel`).
#[derive(Clone, Debug, Default)]
pub enum ClabelFormat {
    /// matplotlib's default: a `ScalarFormatter` without offset applied to
    /// all labelled levels at once.
    #[default]
    Scalar,
    /// A printf-style template applied to each level, such as
    /// `"%1.2f m"`, or a function of the level.
    Label(LabelFormat),
    /// Any tick formatter, applied to all labelled levels at once.
    Formatter(Rc<dyn Formatter>),
    /// A text per level; other levels get the literal text `%1.3f`, as in
    /// matplotlib.
    Map(Vec<(f64, String)>),
}

impl From<LabelFormat> for ClabelFormat {
    fn from(format: LabelFormat) -> Self {
        Self::Label(format)
    }
}

/// Options of [`ContourSet::clabel`] (matplotlib's `clabel` keywords).
#[derive(Clone, Debug)]
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
    pub fmt: ClabelFormat,
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
    /// Keep the labels along their lines when the axes change after
    /// labelling: each rotation is kept in data coordinates and turned
    /// with them when drawn (matplotlib's `use_clabeltext`).
    pub use_clabeltext: bool,
}

impl Default for ClabelOptions {
    fn default() -> Self {
        Self {
            levels: None,
            fontsize: None,
            inline: true,
            inline_spacing: 5.0,
            fmt: ClabelFormat::Scalar,
            colors: None,
            manual: None,
            rightside_up: true,
            zorder: None,
            use_clabeltext: false,
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

pltrs_core::archive_struct!(LabelRecord {
    level,
    text,
    position,
    rotation,
    color,
    artist,
    index,
    follows
});

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

pltrs_core::archive_struct!(LevelLabel {
    index,
    level,
    text,
    width,
    color,
    follows
});

/// The most levels labelled at once, matplotlib's
/// [`MAXTICKS`](crate::ticker::MAXTICKS): like matplotlib, which formats
/// each label with all labelled levels, the work grows with the square of
/// their count.
pub const MAX_LABELLED_LEVELS: usize = crate::ticker::MAXTICKS;

impl ContourSet {
    /// Labels the line contours (matplotlib's `clabel`) and returns the
    /// new text artists.
    ///
    /// # Errors
    ///
    /// When requested levels are not in the set or more than
    /// [`MAX_LABELLED_LEVELS`], a format fails, label text cannot be laid
    /// out, or a manual position has no labelled line to snap to.
    pub fn clabel(&self, options: ClabelOptions) -> Result<Vec<Text>, ContourError> {
        self.clabel_split_by(options, None)
    }

    /// [`ContourSet::clabel`], with the rotations of the labels and the
    /// lines cut under them by `splitter` when given (matplotlib's
    /// `clabel` with an overridden `_split_path_and_get_label_rotation`).
    ///
    /// # Errors
    ///
    /// As [`ContourSet::clabel`].
    pub fn clabel_split_by(
        &self,
        options: ClabelOptions,
        mut splitter: Option<&mut LabelSplitter<'_>>,
    ) -> Result<Vec<Text>, ContourError> {
        let rc = crate::rc::params();
        let indices = self.label_indices(options.levels.as_deref())?;
        // A filled set has a path per band, one fewer than its levels:
        // matplotlib raises an `IndexError` for its last level.
        if indices.len() > MAX_LABELLED_LEVELS {
            return Err(ContourError::TooManyLabelledLevels(indices.len()));
        }
        let paths = self.paths().len();
        if let Some(&index) = indices.iter().find(|&&index| index >= paths) {
            return Err(ContourError::LevelIndex {
                index,
                levels: paths,
            });
        }
        let font = match options.fontsize {
            Some(size) => FontProperties::from_rc(&rc).with_size(size, rc.font_size),
            None => FontProperties::from_rc(&rc),
        };
        let all_levels = self.levels();
        let levels: Vec<f64> = indices.iter().map(|&i| all_levels[i]).collect();
        let relabel = match &options.fmt {
            ClabelFormat::Label(format @ LabelFormat::Func(_)) => Some(format.clone()),
            _ => None,
        };
        // A function of the program's writes each level's label as its
        // labels are placed, as matplotlib measures each in turn.
        let deferred = relabel.is_some() && options.manual.is_none();
        let texts = if deferred {
            vec![String::new(); levels.len()]
        } else {
            label_texts(&levels, &options.fmt, &TickerSettings::from(&*rc))?
        };
        let explicit = options.colors.as_deref().filter(|c| !c.is_empty());
        let colors = self.label_colors(&indices, explicit)?;
        let mut labels = Vec::with_capacity(indices.len());
        for (k, &index) in indices.iter().enumerate() {
            labels.push(LevelLabel {
                index,
                level: levels[k],
                width: if deferred {
                    0.0
                } else {
                    self.text_width(&texts[k], &font)?
                },
                text: texts[k].clone(),
                color: colors[k],
                follows: explicit.is_none(),
            });
        }
        let setup = LabelSetup {
            relabel,
            labels,
            style: TextStyle {
                font,
                zorder: options.zorder.unwrap_or(2.0 + self.zorder()),
                rotates_with_data: options.use_clabeltext,
                rc,
            },
            rightside_up: options.rightside_up,
        };
        self.update(|state| state.label_setup = Some(setup.clone()));
        let spacing = options.inline_spacing;
        let mut placer = Placer::new(self, &setup)?;
        if let Some(points) = &options.manual {
            let data = self.axes().trans_data();
            for &(x, y) in points {
                let target = self.to_display(Point::new(x, y), &data)?;
                placer.label_near(target, options.inline, spacing, splitter.as_deref_mut())?;
            }
        } else {
            let written = placer.label_all(options.inline, spacing, splitter)?;
            self.update(|state| {
                if let Some(setup) = &mut state.label_setup {
                    setup.labels = written;
                }
            });
        }
        Ok(placer.into_created())
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

    /// The levels last labelled (matplotlib's `labelLevelList`), none
    /// before the set is labelled.
    pub fn labelled_levels(&self) -> Vec<f64> {
        self.read(|state| {
            state
                .label_setup
                .as_ref()
                .map(|setup| setup.labels.iter().map(|label| label.level).collect())
                .unwrap_or_default()
        })
    }

    /// Indices of the levels to label: all, or those listed.
    fn label_indices(&self, requested: Option<&[f64]>) -> Result<Vec<usize>, ContourError> {
        let levels = self.levels();
        let Some(requested) = requested else {
            // matplotlib zips the levels with the indices of the coloured
            // values: one per band of a filled set, one fewer than its
            // levels when not extended.
            let values = self.read(|state| state.cvalues.len());
            return Ok((0..levels.len().min(values)).collect());
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
    pub(super) fn text_width(
        &self,
        text: &str,
        font: &FontProperties,
    ) -> Result<f64, ContourError> {
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
    fmt: &ClabelFormat,
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
        ClabelFormat::Scalar => {
            let scalar = ScalarFormatter::from_settings(settings).with_use_offset(OffsetMode::Off);
            with_formatter(&scalar)
        }
        ClabelFormat::Formatter(formatter) => with_formatter(formatter.as_ref()),
        ClabelFormat::Label(format) => levels.iter().map(|&l| Ok(format.printf(l)?)).collect(),
        #[expect(clippy::float_cmp, reason = "matplotlib looks levels up in a dict")]
        ClabelFormat::Map(map) => Ok(levels
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
    pub(super) fn bowl(levels: &[f64]) -> ContourSet {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        let x = linspace(-1.0, 1.0, 9);
        let z = Grid::from_fn(9, 9, |i, j| x[i] * x[i] + x[j] * x[j]);
        let field = QuadField::new(z).with_coords(QuadCoords::Axes { x: x.clone(), y: x });
        let options = ContourOptions {
            levels: Levels::Values(levels.to_vec()),
            ..ContourOptions::default()
        };
        let cs = ax.contour(&field, options).unwrap();
        ax.set_xlim(-1.0, 1.0).expect("finite limits");
        ax.set_ylim(-1.0, 1.0).expect("finite limits");
        cs
    }

    fn subpath_count(cs: &ContourSet) -> usize {
        cs.paths().iter().map(|p| p.subpaths().len()).sum()
    }

    #[test]
    fn a_splitter_places_every_label_as_pltrs_would() {
        let plain = bowl(&[0.5, 1.0]);
        plain.clabel(ClabelOptions::default()).unwrap();
        let split = bowl(&[0.5, 1.0]);
        let mut cuts = Vec::new();
        let mut splitter = |cut: LabelCut<'_>| {
            let placed = split.split_path_at_label(cut).unwrap();
            cuts.push(cut.index);
            placed
        };
        let texts = split
            .clabel_split_by(ClabelOptions::default(), Some(&mut splitter))
            .unwrap();
        assert_eq!(cuts.len(), texts.len());
        let placed = |cs: &ContourSet| {
            cs.labels()
                .into_iter()
                .map(|l| (l.text, l.position, l.rotation))
                .collect::<Vec<_>>()
        };
        assert_eq!(placed(&split), placed(&plain));
        assert_eq!(split.paths(), plain.paths());
    }

    #[test]
    fn labels_cut_out_of_a_vertex_the_line_lacks_fail() {
        let cs = bowl(&[0.5]);
        let line = pltrs_core::Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)]);
        let cut = LabelCut {
            path: &line,
            index: 0,
            position: Point::new(0.0, 0.0),
            width: 1.0,
            spacing: 0.0,
        };
        assert!(
            matches!(cs.split_path_at_label(cut), Err(ContourError::Unlabelled)),
            "before clabel"
        );
        cs.clabel(ClabelOptions::default()).unwrap();
        let beyond = LabelCut { index: 5, ..cut };
        assert!(matches!(
            cs.split_path_at_label(beyond),
            Err(ContourError::VertexIndex {
                index: 5,
                vertices: 2
            })
        ));
    }

    #[test]
    fn automatic_labels_are_upright_and_cut_the_lines() {
        let cs = bowl(&[0.5, 1.0]);
        let before = subpath_count(&cs);
        let texts = cs.clabel(ClabelOptions::default()).unwrap();
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
                .all(|&c| c != pltrs_core::PathCode::ClosePoly)
        );
    }

    #[test]
    fn the_last_level_of_a_filled_set_has_no_line_to_label() {
        // Found by fuzzing: labelling a filled set indexed past its bands.
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let ax = fig.subplot();
        let z = Grid::from_fn(4, 4, |i, j| (i + j) as f64);
        let options = ContourOptions {
            levels: Levels::Values(vec![1.0, 4.0]),
            ..ContourOptions::default()
        };
        let cs = ax.contourf(&QuadField::new(z), options).unwrap();
        // By default, as in matplotlib, the bands are labelled with their
        // lower levels.
        assert!(cs.clabel(ClabelOptions::default()).is_ok());
        let highest = ClabelOptions {
            levels: Some(vec![4.0]),
            ..ClabelOptions::default()
        };
        assert_eq!(
            cs.clabel(highest).unwrap_err(),
            ContourError::LevelIndex {
                index: 1,
                levels: 1
            }
        );
    }

    #[test]
    fn label_counts_are_bounded() {
        // Found by fuzzing: labelling a million levels, each formatted
        // with all of them, never ended.
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let ax = fig.subplot();
        let z = Grid::from_fn(4, 4, |i, j| (i * 4 + j) as f64);
        let options = ContourOptions {
            levels: Levels::Values(linspace(0.0, 15.0, 2000)),
            ..ContourOptions::default()
        };
        let cs = ax.contour(&QuadField::new(z), options).unwrap();
        assert_eq!(
            cs.clabel(ClabelOptions::default()).unwrap_err(),
            ContourError::TooManyLabelledLevels(2000)
        );
    }

    #[test]
    fn manual_labels_snap_to_the_nearest_line() {
        let cs = bowl(&[0.5, 1.0]);
        let before = subpath_count(&cs);
        let options = ClabelOptions {
            manual: Some(vec![(0.7, 0.0)]),
            inline: false,
            fmt: ClabelFormat::Map(vec![(0.5, "half".to_owned())]),
            ..ClabelOptions::default()
        };
        cs.clabel(options).unwrap();
        let labels = cs.labels();
        let [label] = labels.as_slice() else {
            panic!("one label expected");
        };
        assert_eq!((label.level, label.text.as_str()), (0.5, "half"));
        assert_eq!(subpath_count(&cs), before, "not inline: lines are kept");
    }

    #[test]
    fn clabeltext_rotations_follow_the_data() {
        let cs = bowl(&[0.5]);
        let options = ClabelOptions {
            manual: Some(vec![(0.5, 0.5)]),
            use_clabeltext: true,
            ..ClabelOptions::default()
        };
        let [text] = cs.clabel(options).unwrap().try_into().unwrap();
        let displayed = cs.labels()[0].rotation;
        assert!(text.transform_rotates_text());
        // Stretching the view vertically turns the data angle further from
        // the horizontal on screen.
        cs.axes().set_ylim(-0.5, 0.5).expect("finite limits");
        let figure = cs.axes().figure();
        let data = figure.resolve_transform(&cs.axes().trans_data()).unwrap();
        let turned = data.transform_angle(text.rotation(), text.position());
        assert!((turned - displayed).abs() > 1.0, "{turned} vs {displayed}");
    }

    #[test]
    fn label_formats() {
        let settings = TickerSettings::DEFAULT;
        let printf = ClabelFormat::Label("%1.2f m".into());
        assert_eq!(
            label_texts(&[0.5, -1.0], &printf, &settings).unwrap(),
            ["0.50 m", "-1.00 m"]
        );
        let map = ClabelFormat::Map(vec![(1.0, "one".to_owned())]);
        assert_eq!(
            label_texts(&[1.0, 2.0], &map, &settings).unwrap(),
            ["one", "%1.3f"]
        );
        let scalar = label_texts(&[-0.5, 0.0, 0.5], &ClabelFormat::Scalar, &settings).unwrap();
        assert_eq!(scalar, ["\u{2212}0.5", "0.0", "0.5"]);
        let func = ClabelFormat::Label(LabelFormat::func(|level| format!("z={level}")));
        assert_eq!(label_texts(&[2.0], &func, &settings).unwrap(), ["z=2"]);
    }

    #[test]
    fn unknown_levels_and_missing_lines_are_errors() {
        let cs = bowl(&[0.5, 1.0]);
        let unknown = ClabelOptions {
            levels: Some(vec![0.7]),
            ..ClabelOptions::default()
        };
        assert_eq!(
            cs.clabel(unknown).unwrap_err(),
            ContourError::UnknownLabelLevels(vec![0.7])
        );
        let empty = bowl(&[5.0]);
        let manual = ClabelOptions {
            manual: Some(vec![(0.0, 0.0)]),
            ..ClabelOptions::default()
        };
        assert_eq!(
            empty.clabel(manual).unwrap_err(),
            ContourError::NoContourToLabel
        );
    }
}
