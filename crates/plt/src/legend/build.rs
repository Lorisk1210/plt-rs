//! Making a legend: its frame, title, labels and handles, and the box
//! tree that lays them out (matplotlib's `Legend.__init__` and
//! `_init_legend_box`).

use std::rc::Rc;

use plt_core::{Bbox, Point};

use super::artist::{HandleRef, LegendArtist, LegendParent};
use super::handle::Source;
use super::handlers::{self, HandleArtists, HandleBox, HandlerContext};
use super::labelcolor::{HandleView, label_colors};
use super::settings::{Location, Placement, Settings};
use super::{LegendAlignment, LegendError, LegendMode, LegendOptions};
use crate::artist::{Artist, ArtistBase, ArtistId, ArtistStore};
use crate::colors::Color;
use crate::figure::FigureState;
use crate::fonts::{FontProperties, FontSize};
use crate::offsetbox::{AnchorLoc, DrawingArea, OffsetBox, PackAlign, PackMode, Packer, TextArea};
use crate::patches::{BoxShape, BoxStyle, FancyBboxPatch, PatchArtist, Shadow};
use crate::rc::{ColorOrInherit, RcParams};
use crate::text::TextArtist;
use crate::transforms::Transform;

/// The z-order of legends (matplotlib's `Legend.zorder`).
const LEGEND_ZORDER: f64 = 5.0;

/// One entry before its artists join the figure.
struct Entry {
    handle: HandleArtists,
    text: TextArtist,
}

/// The parts of a legend as they join the figure.
struct Parts<'a> {
    store: &'a mut ArtistStore,
    ids: Vec<ArtistId>,
}

/// The handle box and text area of an entry.
type EntryBoxes = (OffsetBox<HandleRef>, OffsetBox<HandleRef>);

impl Parts<'_> {
    fn insert(&mut self, artist: Box<dyn Artist>) -> ArtistId {
        let id = self.store.insert(artist);
        self.ids.push(id);
        id
    }

    /// Adds the handle artists and the label of every entry, the label
    /// coloured by `colors`, and returns the labels and the boxes of each
    /// entry.
    fn insert_entries(
        &mut self,
        made: Vec<Entry>,
        colors: Vec<Option<Color>>,
        area: &HandleBox,
    ) -> (Vec<ArtistId>, Vec<EntryBoxes>) {
        made.into_iter()
            .zip(colors)
            .map(|(entry, color)| {
                let mut text = entry.text;
                if let Some(color) = color {
                    text.color = color;
                }
                let mut drawing =
                    DrawingArea::new(area.width, area.height, area.xdescent, area.ydescent);
                drawing.artists = entry
                    .handle
                    .artists
                    .into_iter()
                    .map(|handle| HandleRef {
                        id: self.insert(handle.artist),
                        attach: handle.attach,
                    })
                    .collect();
                let text = self.insert(Box::new(text));
                let boxes = (
                    OffsetBox::DrawingArea(drawing),
                    OffsetBox::TextArea(TextArea::new(HandleRef::text(text), true)),
                );
                (text, boxes)
            })
            .unzip()
    }
}

/// Makes a legend listing `entries` in `parent`, adds it and its parts to
/// the figure and returns its id.
pub(crate) fn build(
    state: &mut FigureState,
    parent: LegendParent,
    mut entries: Vec<(Source, String)>,
    options: &LegendOptions,
) -> Result<ArtistId, LegendError> {
    let rc = crate::rc::params();
    let settings = Settings::resolve(options, &rc)?;
    let placement = placement(options, parent, &rc)?;
    if options.reverse {
        entries.reverse();
    }
    let ncols = match (entries.len(), options.ncols) {
        (0 | 1, _) => 1,
        (_, 0) => return Err(LegendError::NoColumns),
        (_, ncols) => ncols,
    };
    let frame = frame(options, &settings, &rc)?;
    let shadow = options
        .shadow
        .unwrap_or(rc.legend_shadow)
        .then(|| frame_shadow(&frame))
        .transpose()?;
    let title = title_text(options, &rc)?;
    let has_title = title.base().visible;
    let area = handle_box(&settings);
    let (made, handles) = make_entries(state, &settings, &rc, &entries, &area)?;
    let colors = entry_colors(options, &made, &handles, &rc)?;

    let mut parts = Parts {
        store: &mut state.store,
        ids: Vec::new(),
    };
    let frame = parts.insert(Box::new(frame));
    let shadow = shadow.map(|shadow| parts.insert(Box::new(shadow)));
    let title = parts.insert(Box::new(title));
    let (texts, items) = parts.insert_entries(made, colors, &area);
    let parts = parts.ids;
    let mut title_box = OffsetBox::TextArea(TextArea::new(HandleRef::text(title), false));
    title_box.set_visible(has_title);
    let legend = LegendArtist {
        base: ArtistBase::with_zorder(LEGEND_ZORDER),
        parent,
        location: placement.location,
        outside: placement.outside,
        mode: options.mode,
        anchor: anchor(options, parent),
        legend_box: legend_box(items, title_box, ncols, options, &settings),
        settings,
        frame,
        shadow,
        title,
        texts,
        parts: parts.clone(),
        kept: false,
        dragger: None,
    };
    let id = state.adopt(parent.id(), Box::new(legend));
    for part in parts {
        if let Some(artist) = state.store.get_mut(part) {
            artist.base_mut().parent = Some(id);
        }
    }
    Ok(id)
}

/// The label colour of every made entry (see [`label_colors`]).
fn entry_colors(
    options: &LegendOptions,
    made: &[Entry],
    handles: &[Option<usize>],
    rc: &RcParams,
) -> Result<Vec<Option<Color>>, LegendError> {
    let views: Vec<HandleView<'_>> = handles
        .iter()
        .map(|index| HandleView {
            artist: index.map(|i| made[i].handle.artists[0].artist.as_ref()),
            mapped: index.is_some_and(|i| made[i].handle.mapped),
        })
        .collect();
    label_colors(options.labelcolor.as_ref(), &views, made.len(), rc)
}

/// `bbox_to_anchor` with its coordinate system: the option's, else the
/// parent's fractions.
fn anchor(options: &LegendOptions, parent: LegendParent) -> Option<(Bbox, Transform)> {
    options.bbox_to_anchor.map(|anchor| {
        let transform = options
            .bbox_transform
            .clone()
            .unwrap_or_else(|| parent.fractions());
        (anchor.bbox(), transform)
    })
}

/// The title text, hidden when empty (matplotlib's `set_title`).
fn title_text(options: &LegendOptions, rc: &Rc<RcParams>) -> Result<TextArtist, LegendError> {
    let mut title = TextArtist::new(
        Point::ORIGIN,
        options.title.clone().unwrap_or_default(),
        Rc::clone(rc),
    );
    title.font = title_font(options, rc)?;
    title.base_mut().visible = !title.text.is_empty();
    Ok(title)
}

/// The validated placement: the option, else `legend.loc`, which figure
/// legends take as `"upper right"` when it asks for `"best"`.
fn placement(
    options: &LegendOptions,
    parent: LegendParent,
    rc: &RcParams,
) -> Result<Placement, LegendError> {
    let figure = matches!(parent, LegendParent::Figure(_));
    if let Some(loc) = &options.loc {
        return Placement::parse(loc, figure);
    }
    let location = match Location::parse(&rc.legend_loc)? {
        Location::Best if figure => Location::Anchored(AnchorLoc::UpperRight),
        location => location,
    };
    Ok(Placement {
        location,
        outside: None,
    })
}

/// The title font: `title_fontproperties`, else the rc font at
/// `title_fontsize`, `legend.title_fontsize` or `font.size`.
fn title_font(options: &LegendOptions, rc: &RcParams) -> Result<FontProperties, LegendError> {
    match (&options.title_fontproperties, options.title_fontsize) {
        (Some(_), Some(_)) => Err(LegendError::TitleFontConflict),
        (Some(font), None) => Ok(font.clone()),
        (None, size) => {
            let size = size
                .or(rc.legend_title_fontsize)
                .unwrap_or(FontSize::Points(rc.font_size));
            Ok(FontProperties::from_rc(rc).with_size(size, rc.font_size))
        }
    }
}

/// A frame colour option or rc value, `"inherit"` taking `inherited`.
fn frame_color(option: Option<&Color>, rc_value: &ColorOrInherit, inherited: &Color) -> Color {
    match option {
        Some(Color::Spec(spec)) if spec == "inherit" => inherited.clone(),
        Some(color) => color.clone(),
        None => match rc_value {
            ColorOrInherit::Inherit => inherited.clone(),
            ColorOrInherit::Color(color) => color.clone(),
        },
    }
}

/// The frame (matplotlib's `legendPatch`): a box placed and scaled at
/// layout time, round-cornered unless `fancybox` is off.
fn frame(
    options: &LegendOptions,
    settings: &Settings,
    rc: &Rc<RcParams>,
) -> Result<PatchArtist<FancyBboxPatch>, LegendError> {
    let shape = if options.fancybox.unwrap_or(rc.legend_fancybox) {
        BoxShape::Round {
            rounding_size: Some(0.2),
        }
    } else {
        BoxShape::Square
    };
    let boxed = FancyBboxPatch::new((0.0, 0.0), 1.0, 1.0, BoxStyle::new(shape).with_pad(0.0))
        .with_mutation_scale(settings.fontsize);
    let mut frame = PatchArtist::new(boxed, Rc::clone(rc));
    let face = frame_color(
        options.facecolor.as_ref(),
        &rc.legend_facecolor,
        &rc.axes_facecolor,
    );
    let edge = frame_color(
        options.edgecolor.as_ref(),
        &rc.legend_edgecolor,
        &rc.axes_edgecolor,
    );
    face.resolve()?;
    edge.resolve()?;
    frame.style.facecolor = Some(face);
    frame.style.edgecolor = Some(edge);
    frame.style.linewidth = options.linewidth.or(rc.legend_linewidth);
    let base = frame.base_mut();
    // With a shadow the frame is opaque unless told otherwise.
    base.alpha = options.framealpha.or(if options.shadow == Some(true) {
        Some(1.0)
    } else {
        rc.legend_framealpha
    });
    base.snap = Some(true);
    base.visible = options.frameon.unwrap_or(rc.legend_frameon);
    base.transform = Some(Transform::Display);
    Ok(frame)
}

/// The frame's shadow, placed with the frame at layout time.
fn frame_shadow(
    frame: &PatchArtist<FancyBboxPatch>,
) -> Result<PatchArtist<Shadow<FancyBboxPatch>>, LegendError> {
    let shadow = super::layout::frame_shadow(frame.shape.clone(), &frame.style)?;
    let mut patch = PatchArtist::new(shadow, crate::rc::params());
    patch.base_mut().visible = frame.base().visible;
    Ok(patch)
}

/// The box each handle is drawn in (matplotlib's handle box: a heuristic
/// descent so that handles sit around the text's middle).
fn handle_box(settings: &Settings) -> HandleBox {
    let fontsize = settings.fontsize;
    let descent = 0.35 * fontsize * (settings.handleheight - 0.7);
    HandleBox {
        xdescent: 0.0,
        ydescent: descent,
        width: settings.handlelength * fontsize,
        height: fontsize * settings.handleheight - descent,
    }
}

/// The handle artists and label of every supported entry, and for every
/// entry the index of its made entry (`None` when unsupported).
fn make_entries(
    state: &FigureState,
    settings: &Settings,
    rc: &Rc<RcParams>,
    entries: &[(Source, String)],
    area: &HandleBox,
) -> Result<(Vec<Entry>, Vec<Option<usize>>), LegendError> {
    let ctx = HandlerContext {
        store: &state.store,
        settings,
        rc,
    };
    let mut made = Vec::new();
    let mut indices = Vec::with_capacity(entries.len());
    for (source, label) in entries {
        match handlers::create(&ctx, source, area)? {
            Some(handle) if !handle.artists.is_empty() => {
                let mut text = TextArtist::new(Point::ORIGIN, label.clone(), Rc::clone(rc));
                text.font = settings.prop.clone();
                indices.push(Some(made.len()));
                made.push(Entry { handle, text });
            }
            _ => indices.push(None),
        }
    }
    Ok((made, indices))
}

/// The box tree (matplotlib's `_legend_box`): the title above columns of
/// entries, each a handle box beside its label.
fn legend_box(
    items: Vec<(OffsetBox<HandleRef>, OffsetBox<HandleRef>)>,
    title_box: OffsetBox<HandleRef>,
    ncols: usize,
    options: &LegendOptions,
    settings: &Settings,
) -> OffsetBox<HandleRef> {
    let fontsize = settings.fontsize;
    let rows: Vec<OffsetBox<HandleRef>> = items
        .into_iter()
        .map(|(handle, text)| {
            let children = if options.markerfirst {
                vec![handle, text]
            } else {
                vec![text, handle]
            };
            OffsetBox::HPacker(Packer::new(
                0.0,
                settings.handletextpad * fontsize,
                PackAlign::Baseline,
                children,
            ))
        })
        .collect();
    let column_align = if options.markerfirst {
        PackAlign::Baseline
    } else {
        PackAlign::Right
    };
    let columns = split_columns(rows, ncols)
        .into_iter()
        .map(|column| {
            OffsetBox::VPacker(Packer::new(
                0.0,
                settings.labelspacing * fontsize,
                column_align,
                column,
            ))
        })
        .collect();
    let mut handle_box = Packer::new(
        0.0,
        settings.columnspacing * fontsize,
        PackAlign::Baseline,
        columns,
    );
    if options.mode == LegendMode::Expand {
        handle_box.mode = PackMode::Expand;
    }
    let align = match options.alignment {
        LegendAlignment::Center => PackAlign::Center,
        LegendAlignment::Left => PackAlign::Left,
        LegendAlignment::Right => PackAlign::Right,
    };
    OffsetBox::VPacker(Packer::new(
        settings.borderpad * fontsize,
        settings.labelspacing * fontsize,
        align,
        vec![title_box, OffsetBox::HPacker(handle_box)],
    ))
}

/// numpy's `array_split` into `ncols` (at least one) columns without the
/// empty ones: the first `len % ncols` columns get one entry more.
fn split_columns<T>(items: Vec<T>, ncols: usize) -> Vec<Vec<T>> {
    let (base, extra) = (items.len() / ncols, items.len() % ncols);
    let mut items = items.into_iter();
    (0..ncols)
        .map(|column| {
            let size = base + usize::from(column < extra);
            items.by_ref().take(size).collect::<Vec<_>>()
        })
        .filter(|column| !column.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_split_like_numpy() {
        assert_eq!(
            split_columns((0..5).collect(), 3),
            [vec![0, 1], vec![2, 3], vec![4]]
        );
        assert_eq!(split_columns((0..2).collect(), 3), [vec![0], vec![1]]);
        assert_eq!(split_columns(Vec::<u8>::new(), 2), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn handle_boxes_follow_the_heuristic_descent() {
        let rc = RcParams::matplotlib();
        let settings = Settings::resolve(&LegendOptions::default(), &rc).unwrap();
        let area = handle_box(&settings);
        assert_eq!(area.width, 20.0);
        assert!(area.ydescent.abs() < 1e-12);
        assert!((area.height - 7.0).abs() < 1e-12);
    }
}
