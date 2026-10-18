//! Properties every artist has.

use plt_core::Path;
use plt_render::Sketch;

use super::{AggFilter, ArtistId, Picker};
use crate::patheffects::PathEffect;
use crate::rc::RcParams;
use crate::transforms::Transform;

/// A clip region attached to an artist.
#[derive(Clone, Debug, PartialEq)]
pub enum ClipPathSpec {
    /// Clip to an explicit path given in the coordinates of `transform`.
    Path {
        /// The clip outline.
        path: Path,
        /// Coordinates of `path`.
        transform: Transform,
    },
    /// Clip to the current outline of another artist (a patch).
    Artist(ArtistId),
    /// Clip to the display bounding box of `path` placed by `transform`,
    /// in place of the artist's clip box: matplotlib turns a `Rectangle`
    /// clip path into the clip box `TransformedBbox(Bbox.unit(),
    /// rect.get_transform())`, so a rotated rectangle clips to its
    /// bounding box.
    BoundingBox {
        /// The outline whose bounding box clips.
        path: Path,
        /// Coordinates of `path`.
        transform: Transform,
    },
}

/// The state matplotlib's `Artist` base class carries.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors matplotlib's independent boolean artist properties"
)]
pub struct ArtistBase {
    /// Whether the artist is drawn.
    pub visible: bool,
    /// Opacity override in `[0, 1]`; `None` keeps the colours' own alpha.
    pub alpha: Option<f64>,
    /// Drawing order: higher values are drawn on top.
    pub zorder: f64,
    /// Legend label; labels starting with `_` are excluded from legends.
    pub label: String,
    /// Coordinate system of the artist's data; `None` uses its default.
    pub transform: Option<Transform>,
    /// Whether clipping is applied at all.
    pub clip_on: bool,
    /// Rectangular clip in the coordinates of `transform`-free display
    /// space, given as the extent of an axes or figure frame.
    pub clip_box: Option<ArtistId>,
    /// Arbitrary clip region.
    pub clip_path: Option<ClipPathSpec>,
    /// Rasterize this artist even in vector output.
    pub rasterized: bool,
    /// Exclude from the normal draw (used for blitting animations).
    pub animated: bool,
    /// Whether layout engines account for this artist's extent.
    pub in_layout: bool,
    /// Pixel snapping preference; `None` lets the renderer decide.
    pub snap: Option<bool>,
    /// Hand-drawn wobble.
    pub sketch: Option<Sketch>,
    /// Effects applied when the artist's paths are drawn (matplotlib's
    /// `path_effects`; empty draws the paths as they are).
    pub path_effects: Vec<PathEffect>,
    /// Group id for vector output.
    pub gid: Option<String>,
    /// Hyperlink for vector output.
    pub url: Option<String>,
    /// How mouse presses pick the artist; `None` leaves it unpickable.
    pub picker: Option<Picker>,
    /// Whether a status bar shows the data under the pointer when it is
    /// over the artist (matplotlib's `mouseover`).
    pub mouseover: bool,
    /// An image filter for what the artist draws (matplotlib's
    /// `agg_filter`).
    pub agg_filter: Option<AggFilter>,
    /// Values autoscaling must not expand past (`x`, `y`).
    pub sticky_edges: (Vec<f64>, Vec<f64>),
    /// The containing artist (axes, figure, legend, ...).
    pub parent: Option<ArtistId>,
    /// The artist's own id, assigned when it is inserted into a store.
    id: Option<ArtistId>,
}

impl ArtistBase {
    /// Defaults for an artist drawn at `zorder`.
    pub fn with_zorder(zorder: f64) -> Self {
        Self {
            zorder,
            ..Self::default()
        }
    }

    /// The artist's id within its figure.
    ///
    /// # Panics
    /// If called before the artist was inserted into an `ArtistStore`, which
    /// never happens for artists reachable from a figure.
    pub fn id(&self) -> ArtistId {
        self.id
            .expect("artists are given an id when inserted into a store")
    }

    pub(super) fn assign_id(&mut self, id: ArtistId) {
        self.id = Some(id);
    }

    /// Whether the label marks the artist as excluded from legends.
    pub fn is_label_hidden(&self) -> bool {
        is_hidden_label(&self.label)
    }
}

/// Whether a label keeps what it names out of automatic legends: empty or
/// starting with `_` (matplotlib's `_get_legend_handles_labels`).
pub fn is_hidden_label(label: &str) -> bool {
    label.is_empty() || label.starts_with('_')
}

/// The sketch the `path.sketch` rcParam asks for (matplotlib passes its
/// values on as they are).
pub(crate) fn rc_sketch(rc: &RcParams) -> Option<Sketch> {
    rc.path_sketch.map(|[scale, length, randomness]| Sketch {
        scale,
        length,
        randomness,
    })
}

impl Default for ArtistBase {
    /// matplotlib's `Artist.__init__`: visible at z-order 0, clipped once
    /// given a clip region, sketched and with path effects as the current
    /// `path.sketch` and `path.effects` rcParams say.
    fn default() -> Self {
        let rc = crate::rc::params();
        Self {
            visible: true,
            alpha: None,
            zorder: 0.0,
            label: String::new(),
            transform: None,
            clip_on: true,
            clip_box: None,
            clip_path: None,
            rasterized: false,
            animated: false,
            in_layout: true,
            snap: None,
            sketch: rc_sketch(&rc),
            path_effects: rc.path_effects.clone(),
            gid: None,
            url: None,
            picker: None,
            mouseover: false,
            agg_filter: None,
            sticky_edges: (Vec::new(), Vec::new()),
            parent: None,
            id: None,
        }
    }
}
