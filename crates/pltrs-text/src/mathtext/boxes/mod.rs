//! The TeX box model of matplotlib's mathtext (after Knuth, *TeX: The
//! Program*): characters, boxes, rules, glue and kerns stacked in
//! horizontal and vertical lists. As in TeX, y grows downwards.

mod auto;
mod pack;
mod ship;
mod tree;

pub(crate) use auto::{auto_height_char, auto_width_char};
pub(crate) use pack::Packing;
pub(crate) use ship::{Shipped, ship};
pub use tree::MathNode;

use super::font_name::FontName;
use super::fonts::{FontMetrics, Fonts, GlyphError, GlyphKey};

/// How much text shrinks per script level.
pub(crate) const SHRINK_FACTOR: f64 = 0.7;
/// Script levels beyond which text stops shrinking.
const NUM_SIZE_LEVELS: u32 = 6;

/// Font selection in effect while parsing (matplotlib's `ParserState`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct State {
    /// Current TeX font.
    pub(crate) font: FontName,
    /// Current font class (`rm`, `it`, `bf`, `bfit` or `normal`).
    pub(crate) font_class: FontName,
    /// Size in points.
    pub(crate) fontsize: f64,
}

impl State {
    /// Selects a font; the basic styles also become the font class.
    pub(crate) fn set_font(&mut self, font: FontName) {
        if font.sets_class() {
            self.font_class = font;
        }
        self.font = font;
    }
}

/// Stretchability of glue (matplotlib's `_GlueSpec`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlueSpec {
    width: f64,
    stretch: f64,
    stretch_order: usize,
    shrink: f64,
    shrink_order: usize,
}

impl GlueSpec {
    /// `fill`: infinitely stretchable (second order).
    pub(crate) const FILL: Self = Self {
        width: 0.0,
        stretch: 1.0,
        stretch_order: 2,
        shrink: 0.0,
        shrink_order: 0,
    };
    /// `ss`: stretches and shrinks (first order), for centring.
    pub(crate) const SS: Self = Self {
        width: 0.0,
        stretch: 1.0,
        stretch_order: 1,
        shrink: -1.0,
        shrink_order: 1,
    };
}

/// A single character with its font and metrics.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Char {
    /// The symbol (`x`, `\alpha`, ...).
    pub(crate) symbol: String,
    /// TeX font name.
    pub(crate) font: FontName,
    /// Font class.
    pub(crate) font_class: FontName,
    /// Size in points.
    pub(crate) fontsize: f64,
    /// Metrics from the last update (not rescaled when shrinking).
    pub(crate) metrics: FontMetrics,
    /// Box width.
    pub(crate) width: f64,
    /// Box height.
    pub(crate) height: f64,
    /// Box depth.
    pub(crate) depth: f64,
    /// Accents are positioned by their ink box.
    pub(crate) accent: bool,
    level: u32,
}

impl Char {
    /// A character in the current font.
    pub(crate) fn new(
        symbol: &str,
        state: &State,
        fonts: &mut Fonts<'_>,
    ) -> Result<Self, GlyphError> {
        Self::build(symbol, state, fonts, false)
    }

    /// An accent: its box is the ink box of the glyph.
    pub(crate) fn accent(
        symbol: &str,
        state: &State,
        fonts: &mut Fonts<'_>,
    ) -> Result<Self, GlyphError> {
        Self::build(symbol, state, fonts, true)
    }

    fn build(
        symbol: &str,
        state: &State,
        fonts: &mut Fonts<'_>,
        accent: bool,
    ) -> Result<Self, GlyphError> {
        let mut c = Self {
            symbol: symbol.to_owned(),
            font: state.font,
            font_class: state.font_class,
            fontsize: state.fontsize,
            metrics: FontMetrics::default(),
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            accent,
            level: 0,
        };
        c.update_metrics(fonts)?;
        Ok(c)
    }

    /// The glyph lookup key of this character.
    pub(crate) fn key(&self) -> GlyphKey<'_> {
        GlyphKey {
            font: self.font,
            class: self.font_class,
            symbol: &self.symbol,
            fontsize: self.fontsize,
        }
    }

    /// Re-reads the metrics at the current font and size.
    pub(crate) fn update_metrics(&mut self, fonts: &mut Fonts<'_>) -> Result<(), GlyphError> {
        let metrics = fonts.info(self.key())?.metrics;
        self.metrics = metrics;
        if self.accent {
            self.width = metrics.xmax - metrics.xmin;
            self.height = metrics.ymax - metrics.ymin;
            self.depth = 0.0;
        } else {
            self.width = if self.symbol == " " {
                metrics.advance
            } else {
                metrics.width
            };
            self.height = metrics.iceberg;
            self.depth = -(metrics.iceberg - metrics.height);
        }
        Ok(())
    }

    fn shrink(&mut self, fonts: &mut Fonts<'_>) -> Result<(), GlyphError> {
        self.level += 1;
        if self.level < NUM_SIZE_LEVELS {
            self.fontsize *= SHRINK_FACTOR;
            self.width *= SHRINK_FACTOR;
            self.height *= SHRINK_FACTOR;
            self.depth *= SHRINK_FACTOR;
        }
        if self.accent {
            self.update_metrics(fonts)?;
        }
        Ok(())
    }

    /// Space to add after this character: the advance beyond the ink
    /// width plus kerning with a following character.
    fn kerning(&self, next: Option<&Node>, fonts: &mut Fonts<'_>) -> Result<f64, GlyphError> {
        let advance = self.metrics.advance - self.width;
        let kern = match next {
            Some(Node::Char(next)) => fonts.kern(self.key(), next.key())?,
            _ => 0.0,
        };
        Ok(advance + kern)
    }
}

/// A box with a size and no content (`Box`, `Hbox`, `Vbox`), or a solid
/// rule whose infinite dimensions run to the enclosing box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dims {
    /// Width.
    pub(crate) width: f64,
    /// Height.
    pub(crate) height: f64,
    /// Depth.
    pub(crate) depth: f64,
    level: u32,
    /// How many slots of the enclosing list hold this very box. matplotlib
    /// builds the `\substack` separators as `[Vbox(...)] * n`, one object
    /// in every slot, so shrinking the list shrinks that object once per
    /// slot; each copy here replays all of those shrinks.
    copies: u32,
    /// What matplotlib made it as (its box tree names it so).
    pub(crate) kind: DimsKind,
}

/// The class matplotlib makes an empty box or a rule of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DimsKind {
    /// `Box(width, height, depth)`.
    Box,
    /// `Hbox(width)`.
    Hbox,
    /// `Vbox(height, depth)`.
    Vbox,
    /// `Hrule(state, thickness)`.
    Hrule,
}

impl Dims {
    /// A box of the given size.
    pub(crate) fn new(width: f64, height: f64, depth: f64) -> Self {
        Self {
            width,
            height,
            depth,
            level: 0,
            copies: 1,
            kind: DimsKind::Box,
        }
    }

    /// The box made as `kind`.
    const fn made_as(self, kind: DimsKind) -> Self {
        Self { kind, ..self }
    }

    fn shrink(&mut self) {
        for _ in 0..self.copies {
            self.level += 1;
            if self.level < NUM_SIZE_LEVELS {
                self.width *= SHRINK_FACTOR;
                self.height *= SHRINK_FACTOR;
                self.depth *= SHRINK_FACTOR;
            }
        }
    }
}

/// A horizontal or vertical list of nodes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct List {
    /// Vertical (`Vlist`) rather than horizontal (`Hlist`).
    pub(crate) vertical: bool,
    /// The content.
    pub(crate) children: Vec<Node>,
    /// Width.
    pub(crate) width: f64,
    /// Height.
    pub(crate) height: f64,
    /// Depth.
    pub(crate) depth: f64,
    /// Offset perpendicular to the enclosing list (down or right).
    pub(crate) shift_amount: f64,
    /// How much each unit of glue stretches or shrinks.
    pub(crate) glue_set: f64,
    /// 1 when stretching, -1 when shrinking, 0 for natural size.
    pub(crate) glue_sign: i8,
    /// The order of infinity of the glue that stretches or shrinks.
    pub(crate) glue_order: usize,
    /// Drawn invisibly (`\phantom`).
    pub(crate) is_phantom: bool,
    /// The function this list typesets (`\lim`, ...).
    pub(crate) function_name: Option<String>,
    /// The subclass of `Hlist` matplotlib made it as, if any.
    pub(crate) class: ListClass,
    level: u32,
}

/// The subclasses of `Hlist` matplotlib makes lists of (its box tree names
/// them so).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListClass {
    /// A plain `Hlist` or `Vlist`.
    Plain,
    /// `HCentered`: content centred between stretchable glue.
    Centered,
    /// `AutoHeightChar`: a delimiter sized to a height.
    AutoHeightChar,
    /// `AutoWidthChar`: an accent sized to a width.
    AutoWidthChar,
}

/// A node of the box model.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    /// A character.
    Char(Char),
    /// Fixed space (horizontal in an `Hlist`, vertical in a `Vlist`).
    Kern {
        /// The space.
        width: f64,
        /// Script level.
        level: u32,
    },
    /// Stretchable space.
    Glue {
        /// Its stretchability.
        spec: GlueSpec,
        /// Script level.
        level: u32,
    },
    /// An empty box.
    Box(Dims),
    /// A solid rule.
    Rule(Dims),
    /// A nested list.
    List(List),
}

impl Node {
    /// A kern of `width`.
    pub(crate) fn kern(width: f64) -> Self {
        Self::Kern { width, level: 0 }
    }

    /// Glue of the given kind.
    pub(crate) fn glue(spec: GlueSpec) -> Self {
        Self::Glue { spec, level: 0 }
    }

    /// An empty box of width `width` (`Hbox`).
    pub(crate) fn hbox(width: f64) -> Self {
        Self::Box(Dims::new(width, 0.0, 0.0).made_as(DimsKind::Hbox))
    }

    /// An empty box of the given height and depth (`Vbox`).
    pub(crate) fn vbox(height: f64, depth: f64) -> Self {
        Self::Box(Dims::new(0.0, height, depth).made_as(DimsKind::Vbox))
    }

    /// One of `copies` slots of a list holding the same empty box of the
    /// given height and depth (a `Vbox` repeated by reference).
    pub(crate) fn shared_vbox(height: f64, depth: f64, copies: u32) -> Self {
        Self::Box(Dims {
            copies,
            ..Dims::new(0.0, height, depth).made_as(DimsKind::Vbox)
        })
    }

    /// A horizontal rule running across the enclosing box (`Hrule`).
    pub(crate) fn hrule(thickness: f64) -> Self {
        Self::Rule(
            Dims::new(f64::INFINITY, thickness * 0.5, thickness * 0.5).made_as(DimsKind::Hrule),
        )
    }

    /// Width of the node (zero for glue).
    pub(crate) fn width(&self) -> f64 {
        match self {
            Self::Char(c) => c.width,
            Self::Kern { width, .. } => *width,
            Self::Glue { .. } => 0.0,
            Self::Box(dims) | Self::Rule(dims) => dims.width,
            Self::List(list) => list.width,
        }
    }

    /// Height of the node.
    pub(crate) fn height(&self) -> f64 {
        match self {
            Self::Char(c) => c.height,
            Self::Kern { .. } | Self::Glue { .. } => 0.0,
            Self::Box(dims) | Self::Rule(dims) => dims.height,
            Self::List(list) => list.height,
        }
    }

    /// Depth of the node.
    pub(crate) fn depth(&self) -> f64 {
        match self {
            Self::Char(c) => c.depth,
            Self::Kern { .. } | Self::Glue { .. } => 0.0,
            Self::Box(dims) | Self::Rule(dims) => dims.depth,
            Self::List(list) => list.depth,
        }
    }

    /// Shift of a list (zero for other nodes).
    pub(crate) fn shift_amount(&self) -> f64 {
        match self {
            Self::List(list) => list.shift_amount,
            _ => 0.0,
        }
    }

    /// Shrinks the node one script level.
    pub(crate) fn shrink(&mut self, fonts: &mut Fonts<'_>) -> Result<(), GlyphError> {
        match self {
            Self::Char(c) => c.shrink(fonts)?,
            Self::Kern { width, level } => {
                *level += 1;
                if *level < NUM_SIZE_LEVELS {
                    *width *= SHRINK_FACTOR;
                }
            }
            Self::Glue { spec, level } => {
                *level += 1;
                if *level < NUM_SIZE_LEVELS {
                    spec.width *= SHRINK_FACTOR;
                }
            }
            Self::Box(dims) | Self::Rule(dims) => dims.shrink(),
            Self::List(list) => list.shrink(fonts)?,
        }
        Ok(())
    }

    /// TeX's `char_node` notion as mathtext uses it: no vertical lists
    /// anywhere inside.
    pub(crate) fn is_char_node(&self) -> bool {
        match self {
            Self::List(list) => !list.vertical && list.children.iter().all(Self::is_char_node),
            _ => true,
        }
    }
}

impl List {
    fn empty(vertical: bool, children: Vec<Node>) -> Self {
        Self {
            vertical,
            children,
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            shift_amount: 0.0,
            glue_set: 0.0,
            glue_sign: 0,
            glue_order: 0,
            is_phantom: false,
            function_name: None,
            class: ListClass::Plain,
            level: 0,
        }
    }

    /// An `Hlist` at its natural width, with kerns inserted between
    /// characters.
    pub(crate) fn hlist(children: Vec<Node>, fonts: &mut Fonts<'_>) -> Result<Self, GlyphError> {
        let mut list = Self::empty(false, children);
        list.kern(fonts)?;
        list.hpack(0.0, Packing::Additional);
        Ok(list)
    }

    /// An `Hlist` without automatic kerning.
    pub(crate) fn hlist_unkerned(children: Vec<Node>) -> Self {
        let mut list = Self::empty(false, children);
        list.hpack(0.0, Packing::Additional);
        list
    }

    /// An `Hlist` centring its content between `ss` glue (`HCentered`).
    pub(crate) fn hcentered(children: Vec<Node>) -> Self {
        let mut all = Vec::with_capacity(children.len() + 2);
        all.push(Node::glue(GlueSpec::SS));
        all.extend(children);
        all.push(Node::glue(GlueSpec::SS));
        Self {
            class: ListClass::Centered,
            ..Self::hlist_unkerned(all)
        }
    }

    /// A `Vlist` at its natural height.
    pub(crate) fn vlist(children: Vec<Node>) -> Self {
        let mut list = Self::empty(true, children);
        list.vpack(0.0, Packing::Additional, f64::INFINITY);
        list
    }

    /// Inserts kerns after characters (matplotlib's `Hlist.kern`).
    pub(crate) fn kern(&mut self, fonts: &mut Fonts<'_>) -> Result<(), GlyphError> {
        let mut kerns = Vec::with_capacity(self.children.len());
        for (i, node) in self.children.iter().enumerate() {
            kerns.push(match node {
                Node::Char(c) => c.kerning(self.children.get(i + 1), fonts)?,
                _ => 0.0,
            });
        }
        let children = std::mem::take(&mut self.children);
        for (node, kern) in children.into_iter().zip(kerns) {
            self.children.push(node);
            if kern != 0.0 {
                self.children.push(Node::kern(kern));
            }
        }
        Ok(())
    }

    /// Shrinks the list and its content one script level.
    pub(crate) fn shrink(&mut self, fonts: &mut Fonts<'_>) -> Result<(), GlyphError> {
        for child in &mut self.children {
            child.shrink(fonts)?;
        }
        self.level += 1;
        if self.level < NUM_SIZE_LEVELS {
            self.width *= SHRINK_FACTOR;
            self.height *= SHRINK_FACTOR;
            self.depth *= SHRINK_FACTOR;
            self.shift_amount *= SHRINK_FACTOR;
            self.glue_set *= SHRINK_FACTOR;
        }
        Ok(())
    }

    /// Wraps the list into a node.
    pub(crate) fn into_node(self) -> Node {
        Node::List(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinking_stops_after_six_levels() {
        let mut dims = Dims::new(1.0, 1.0, 1.0);
        for _ in 0..10 {
            dims.shrink();
        }
        assert_eq!(dims.width, SHRINK_FACTOR.powi(5));
        let Node::Box(mut shared) = Node::shared_vbox(0.0, 1.0, 2) else {
            panic!("a shared vbox is a box");
        };
        shared.shrink();
        assert_eq!(shared.depth, SHRINK_FACTOR * SHRINK_FACTOR);
        for _ in 0..3 {
            shared.shrink();
        }
        assert_eq!(shared.depth, SHRINK_FACTOR.powi(5));
        let mut state = State {
            font: FontName::Default,
            font_class: FontName::Rm,
            fontsize: 10.0,
        };
        state.set_font(FontName::Bf);
        assert_eq!((state.font, state.font_class), (FontName::Bf, FontName::Bf));
        state.set_font(FontName::Cal);
        assert_eq!(
            (state.font, state.font_class),
            (FontName::Cal, FontName::Bf)
        );
    }

    #[test]
    fn char_nodes_exclude_vertical_lists() {
        let v = List::vlist(vec![Node::vbox(1.0, 2.0)]);
        assert_eq!((v.height, v.depth), (1.0, 2.0));
        let h = List::hlist_unkerned(vec![Node::hbox(3.0), v.into_node()]);
        assert!(!Node::List(h.clone()).is_char_node());
        assert_eq!(h.width, 3.0);
        assert!(Node::hbox(1.0).is_char_node());
        let centred = List::hcentered(vec![Node::hbox(2.0)]);
        assert_eq!(centred.children.len(), 3);
        assert_eq!(Node::hrule(2.0).height(), 1.0);
        assert_eq!(Node::kern(2.0).width(), 2.0);
    }
}
