//! What a text drawn through a renderer is: matplotlib's `Text.draw`
//! hands a renderer each line with the `Text` drawing it (`mtext`) and its
//! font, which a renderer of the program's receives (see
//! [`Renderer::draw_text_lines`](pltrs_render::Renderer::draw_text_lines)).

use crate::artist::ArtistId;
use crate::fonts::FontProperties;
use crate::ticker::AxisName;

/// Which text of an axis a drawn text is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisTextPart {
    /// The axis label.
    Label,
    /// The offset text.
    OffsetText,
    /// The label of a tick.
    TickLabel {
        /// Whether the tick is a minor one.
        minor: bool,
        /// The index of the tick among those its locator places.
        index: usize,
        /// Whether it is the label on the second side (top, right).
        second: bool,
    },
}

/// The artist a drawn text is, as matplotlib's `Text.draw` passes it to
/// the renderer (`mtext`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextOwner {
    /// A text artist.
    Artist(ArtistId),
    /// The text of the cell at `(row, column)` of a table.
    Cell {
        /// The table.
        table: ArtistId,
        /// The cell's row.
        row: i32,
        /// The cell's column.
        column: i32,
    },
    /// A text of an axis of the 2-D Axes `axes`.
    Axis {
        /// The Axes.
        axes: ArtistId,
        /// The axis.
        axis: AxisName,
        /// Which of its texts.
        part: AxisTextPart,
    },
    /// A text pltrs draws without telling which (3-D axes, axisartist).
    Unnamed,
}

/// What a renderer drawing texts whole is told of a text: its owner and
/// its font (the
/// [`TextLines::source`](pltrs_render::TextLines::source) of the texts
/// pltrs draws).
#[derive(Clone, Debug)]
pub struct DrawnText {
    /// The artist the text is.
    pub owner: TextOwner,
    /// Its font.
    pub font: FontProperties,
}
