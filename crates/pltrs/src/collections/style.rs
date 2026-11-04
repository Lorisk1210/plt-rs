//! Colours (faces, edges and hatches), widths and dash patterns of
//! collection items after defaults and colour mapping.

use pltrs_core::Rgba;
use pltrs_render::Dashes;

use super::{CollectionArtist, CollectionKind, EdgeColors};
use crate::artist::Artist;
use crate::colors::{Alpha, Color, ColorError, to_rgba_array};
use crate::lines::LineStyle;
use crate::rc::{ColorOrEdge, RcParams};

/// Face and edge colours to draw: those of the last 3-D projection, or
/// else [`resolve_base_colors`].
pub(super) fn resolve_colors(c: &CollectionArtist) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
    match c
        .depth_sorted
        .as_ref()
        .and_then(|sorted| sorted.colors.as_ref())
    {
        Some(colors) => Ok(colors.clone()),
        None => resolve_base_colors(c),
    }
}

/// Face and edge colours as matplotlib's getters report them: those of
/// the last 3-D projection, or else [`resolve_base_colors`] with the
/// mapped colours only where the last draw put them
/// ([`CollectionArtist::mapped_shown`]), the colours as set elsewhere.
pub(super) fn resolve_reported_colors(
    c: &CollectionArtist,
) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
    match c
        .depth_sorted
        .as_ref()
        .and_then(|sorted| sorted.colors.as_ref())
    {
        Some(colors) => Ok(colors.clone()),
        None => resolve_mapped_on(c, c.mapped_shown.get()),
    }
}

/// Face and edge colours after defaults, colour mapping and alpha
/// (matplotlib's `_set_facecolor`, `_set_edgecolor` and
/// `update_scalarmappable`).
pub(super) fn resolve_base_colors(c: &CollectionArtist) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
    resolve_mapped_on(c, MappedSides::BOTH)
}

/// The sides of a collection's items (faces, edges) coloured by its
/// colour mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MappedSides {
    /// The faces.
    pub(crate) faces: bool,
    /// The edges.
    pub(crate) edges: bool,
}

pltrs_core::archive_struct!(MappedSides { faces, edges });

impl MappedSides {
    /// Both sides, where the mapping colours them.
    const BOTH: Self = Self {
        faces: true,
        edges: true,
    };

    /// The sides the colour mapping of `c` colours (matplotlib's
    /// `_set_mappable_flags`): the faces unless they are `'none'`, and the
    /// edges when they follow the faces, or when the faces are `'none'`
    /// and the edges keep the default rule; none without a mapping.
    pub(crate) fn of(c: &CollectionArtist) -> Self {
        if c.mapping.is_none() && c.program_colors.is_none() {
            return Self::default();
        }
        let face_is_none = unfilled(&c.face_spec(c.rc()));
        let edges = if face_is_none {
            matches!(c.edgecolors, EdgeColors::Default)
        } else {
            matches!(c.edgecolors, EdgeColors::Face)
        };
        Self {
            faces: !face_is_none,
            edges,
        }
    }
}

/// Face and edge colours after defaults, alpha and the colour mapping on
/// the sides of `on` it colours.
fn resolve_mapped_on(
    c: &CollectionArtist,
    on: MappedSides,
) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
    let rc = c.rc();
    let alpha = collection_alpha(c);
    let resolve = |colors: &[Color]| with_alpha(c, &alpha, colors);
    let face_spec = c.face_spec(rc);
    let face_is_none = unfilled(&face_spec);
    let edge_default = c.kind == CollectionKind::Line;

    let sides = MappedSides::of(c);
    let face_mapped = sides.faces && on.faces;
    let edge_mapped = sides.edges && on.edges;
    // A draw maps the values whether or not a side shows them (and fails
    // when they cannot be mapped), as matplotlib's `update_scalarmappable`.
    let mapping = on == MappedSides::BOTH || face_mapped || edge_mapped;
    let mapped = match (&c.program_colors, &c.mapping) {
        (Some(colors), _) if mapping => Some(colors.clone()),
        (None, Some(mapping_of_values)) if mapping => Some(mapping_of_values.colors(&alpha)?),
        _ => None,
    };

    // The mapped colours move to the faces, or the edges, or both (one
    // copy): meshes map a million of them.
    let (faces, mapped) = match mapped {
        Some(colors) if face_mapped && edge_mapped => (colors.clone(), Some(colors)),
        Some(colors) if face_mapped => (colors, None),
        mapped => (resolve(&face_spec)?, mapped),
    };
    let edges = match (mapped, edge_mapped, &c.edgecolors) {
        (Some(colors), true, _) => colors,
        (_, _, EdgeColors::Face) => faces.clone(),
        (_, _, EdgeColors::Colors(colors)) => resolve(colors)?,
        (_, _, EdgeColors::Default) => {
            if rc.patch_force_edgecolor || edge_default || face_is_none {
                resolve(std::slice::from_ref(default_edge(c)))?
            } else {
                Vec::new()
            }
        }
    };
    Ok((faces, edges))
}

impl CollectionArtist {
    /// The getters report the face colours as set until the next draw.
    pub(super) fn unmap_faces(&self) {
        let shown = self.mapped_shown.get();
        self.mapped_shown.set(MappedSides {
            faces: false,
            ..shown
        });
    }

    /// The getters report the edge colours as set until the next draw.
    pub(super) fn unmap_edges(&self) {
        let shown = self.mapped_shown.get();
        self.mapped_shown.set(MappedSides {
            edges: false,
            ..shown
        });
    }

    /// The face colours as set, else the default of the collection's
    /// kind: none for lines, `patch.facecolor` for the others.
    pub(super) fn face_spec(&self, rc: &RcParams) -> Vec<Color> {
        self.facecolors.clone().unwrap_or_else(|| {
            vec![if self.kind == CollectionKind::Line {
                Color::none()
            } else {
                rc.patch_facecolor.clone()
            }]
        })
    }
}

/// Whether faces of `face_spec` are `'none'`; no colours at all
/// (matplotlib's `'none'` as an empty RGBA array) count as `'none'`,
/// which leaves the colour mapping to the edges.
pub(super) fn unfilled(face_spec: &[Color]) -> bool {
    match face_spec {
        [] => true,
        [only] => only.is_none(),
        _ => false,
    }
}

/// The colours drawn in the gaps of dashed edges, with the collection's
/// alpha (matplotlib's `_set_gapcolor`); `None` when the gaps stay empty.
pub(super) fn resolve_gap_colors(c: &CollectionArtist) -> Result<Option<Vec<Rgba>>, String> {
    c.gapcolors
        .as_deref()
        .map(|colors| with_alpha(c, &collection_alpha(c), colors))
        .transpose()
}

/// The collection's opacity: one per item, or the artist's single alpha.
fn collection_alpha(c: &CollectionArtist) -> Alpha {
    c.alphas
        .clone()
        .map_or_else(|| Alpha::from(c.base().alpha), Alpha::Each)
}

/// `colors` with `alpha` applied (matplotlib's `to_rgba_array(colors,
/// alpha)`); a lone "none" means no colours at all.
fn with_alpha(c: &CollectionArtist, alpha: &Alpha, colors: &[Color]) -> Result<Vec<Rgba>, String> {
    if let [only] = colors
        && only.is_none()
    {
        return Ok(Vec::new());
    }
    match alpha {
        Alpha::Each(_) => to_rgba_array(colors.to_vec(), alpha.clone()),
        _ => colors
            .iter()
            .map(|color| color.resolve_alpha(c.base().alpha))
            .collect(),
    }
    .map_err(|e| e.to_string())
}

/// The hatch colours of a collection (matplotlib's `hatchcolor`).
#[derive(Clone, Debug, PartialEq)]
pub enum HatchColors {
    /// The edge colours, or the default edge colour of a collection
    /// without edges (matplotlib's `"edge"`).
    Edge,
    /// Explicit colours, cycled over the items.
    Colors(Vec<Color>),
}

pltrs_core::archive_enum!(HatchColors {
    Edge,
    Colors(v0),
});

impl HatchColors {
    /// `hatch.color` as a collection takes it when it is made.
    pub(super) fn from_rc(rc: &RcParams) -> Self {
        match &rc.hatch_color {
            ColorOrEdge::Edge => Self::Edge,
            ColorOrEdge::Color(color) => Self::Colors(vec![color.clone()]),
        }
    }
}

/// The hatch colour of each item (matplotlib's
/// `Collection.get_hatchcolor`): the resolved `edges` for
/// [`HatchColors::Edge`], or the default edge colour when there are none;
/// both the default and explicit colours take the collection's alpha.
pub(super) fn resolve_hatch_colors(
    c: &CollectionArtist,
    edges: &[Rgba],
) -> Result<Vec<Rgba>, ColorError> {
    let alpha = c.base().alpha;
    match &c.hatchcolors {
        HatchColors::Edge if edges.is_empty() => Ok(vec![default_edge(c).resolve_alpha(alpha)?]),
        HatchColors::Edge => Ok(edges.to_vec()),
        HatchColors::Colors(colors) => colors
            .iter()
            .map(|color| color.resolve_alpha(alpha))
            .collect(),
    }
}

/// The edge colour a collection of its kind gets by default.
fn default_edge(c: &CollectionArtist) -> &Color {
    if c.kind == CollectionKind::Line {
        &c.rc().lines_color
    } else {
        &c.rc().patch_edgecolor
    }
}

/// Pairs line widths with dash patterns (cycling both to their least
/// common multiple) and scales each pattern by its width (matplotlib's
/// `Collection._bcast_lwls`).
pub(super) fn broadcast_widths_and_dashes(
    widths: &[f64],
    styles: &[LineStyle],
    rc: &RcParams,
) -> (Vec<f64>, Vec<Dashes>) {
    let styles = if styles.is_empty() {
        &[LineStyle::Solid][..]
    } else {
        styles
    };
    let widths = if widths.is_empty() {
        &[0.0][..]
    } else {
        widths
    };
    let n = lcm(widths.len(), styles.len());
    let paired_widths: Vec<f64> = (0..n).map(|i| widths[i % widths.len()]).collect();
    let dashes = (0..n)
        .map(|i| styles[i % styles.len()].scaled_dashes(paired_widths[i], rc))
        .collect();
    (paired_widths, dashes)
}

fn lcm(a: usize, b: usize) -> usize {
    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    a / gcd(a, b) * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_and_dashes_cycle_to_common_length() {
        let rc = RcParams::matplotlib();
        let (widths, dashes) = broadcast_widths_and_dashes(
            &[1.0, 2.0],
            &[LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted],
            &rc,
        );
        assert_eq!(widths.len(), 6);
        assert_eq!(widths, [1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
        assert_eq!(dashes[1].lengths(), [3.7 * 2.0, 1.6 * 2.0]);
    }

    #[test]
    fn mapped_lines_without_faces_colour_their_edges() {
        use crate::artist::ArtistHandle;
        use crate::collections::CollectionSpec;
        use crate::figure::Figure;
        use pltrs_core::Point;

        let ax = Figure::new().subplot();
        let segment = vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        let lines = ax.add_collection(CollectionSpec::lines(vec![segment.clone(), segment]));
        lines.set_facecolors(Vec::new());
        lines.set_array(vec![Some(0.0), Some(1.0)]).unwrap();
        let (faces, edges) = lines
            .raw()
            .with::<CollectionArtist, _>(CollectionArtist::resolved_colors)
            .unwrap();
        assert_eq!(faces.len(), 0, "no faces");
        assert_eq!(edges.len(), 2, "one mapped colour per line");
        assert_ne!(edges[0], edges[1]);
    }

    #[test]
    fn least_common_multiple() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(1, 1), 1);
    }

    #[test]
    fn per_item_alphas_repeat_a_single_colour() {
        use crate::artist::ArtistHandle;
        use crate::axes::{ScatterColors, ScatterOptions};
        use crate::colors::Alpha;
        use crate::figure::Figure;

        let ax = Figure::new().subplot();
        let options = ScatterOptions {
            colors: ScatterColors::Single(Color::from("red")),
            alpha: Alpha::Each(vec![0.25, 0.5, 1.0]),
            ..ScatterOptions::default()
        };
        let points = ax
            .scatter_with([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], options)
            .unwrap();
        let (faces, edges) = points
            .raw()
            .with::<CollectionArtist, _>(CollectionArtist::resolved_colors)
            .unwrap();
        let alphas: Vec<f64> = faces.iter().map(|c| c.a).collect();
        assert_eq!(alphas, [0.25, 0.5, 1.0]);
        assert_eq!(edges.len(), 3, "face-coloured edges follow");
        let wrong = ScatterOptions {
            alpha: Alpha::Each(vec![0.5; 2]),
            ..ScatterOptions::default()
        };
        assert!(ax.scatter_with([0.0, 1.0, 2.0], [0.0; 3], wrong).is_err());
    }
}
