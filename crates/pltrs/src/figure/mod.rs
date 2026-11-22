//! Figures, subfigures, the figure canvas and the options to save a
//! figure (matplotlib's `matplotlib.figure`).
//!
//! A [`Figure`] is the whole image. The page of [`Figure`] lists all that
//! a figure can do, grouped by task. Start there.
//!
//! ```
//! use pltrs::figure::Figure;
//!
//! let fig = Figure::with_size(4.0, 3.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! let svg: String = fig.to_svg()?;
//! assert!(svg.contains("<svg"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Figures and their parts
//!
//! | Item | What it is |
//! |------|------------|
//! | [`Figure`] | The whole image, or a subfigure in it. |
//! | [`WeakFigure`] | A handle that does not keep a figure alive. |
//! | [`FigSize`], [`SizeUnit`] | A figure size in inches, centimeters or pixels. |
//! | [`SubfigureOptions`] | The ratios and the spacing of a grid of subfigures. |
//! | [`Share`] | The axes of a subplot grid that share their x or y axis. |
//! | [`Mosaic`], [`MosaicCell`], [`MosaicOptions`] | A layout of labeled cells for [`Figure::subplot_mosaic`]. |
//! | [`MosaicAxes`] | The axes of a mosaic, by label. |
//!
//! # Save
//!
//! | Item | What it is |
//! |------|------------|
//! | [`SaveOptions`] | The options of a save: resolution, crop, transparency, colors, metadata, quality and page. |
//! | [`SaveFormat`] | The 13 file formats that pltrs writes. |
//! | [`BboxInches`], [`PadInches`] | The part of the figure to save, and the padding of a tight crop. |
//! | [`Metadata`], [`PngMetadata`], [`SvgMetadata`] | The document metadata of a saved file. |
//! | [`Paper`], [`PageOrientation`] | The page of a PostScript document. |
//! | [`SvgFontType`] | How an SVG document writes text: as paths or as text. |
//! | [`ImageFile`] | An image file to write beside an SVG document or a PGF picture. |
//! | [`PdfPages`] | A PDF document with one page for each figure. |
//! | [`PgfPages`] | A PDF document that LaTeX typesets, with one page for each figure. |
//! | [`PgfTexSystem`], [`PgfTextMetrics`], [`PdfToPng`] | The LaTeX program, the text measurement and the PNG converter of PGF output. |
//!
//! # Canvas
//!
//! | Item | What it is |
//! |------|------------|
//! | [`FigureCanvas`] | The pixels of the last draw. Use it to draw only the artists that change (blitting). |
//! | [`BufferRegion`] | A rectangle of the canvas that you saved, to put back later. |
//! | [`PixelBox`] | A rectangle of whole canvas pixels. |
//!
//! # Errors
//!
//! | Error | When it occurs |
//! |-------|----------------|
//! | [`SaveError`] | A save fails. |
//! | [`CanvasError`] | A canvas operation fails. |
//! | [`DrawErrors`] | Some artists cannot be drawn. |
//! | [`SizeError`] | A figure size is not valid. |
//! | [`AxesRectError`] | A rectangle cannot hold new axes. |
//! | [`MosaicError`] | A mosaic is not valid. |
//! | [`PageSetupError`] | A paper or orientation name does not exist. |
//! | [`UnknownSvgFontType`] | A name is not an SVG font type. |
//! | [`ProgramError`] | An external program of the PostScript output fails. |
//!
//! # For programs that wrap pltrs
//!
//! pltrs (Python) uses these items to give matplotlib objects to Python
//! code. Most Rust programs do not need them.
//!
//! | Purpose | Items |
//! |---------|-------|
//! | Run code during a draw | [`DrawHook`], [`set_draw_end_handler`], [`clear_draw_end_handler`], [`draw_tick`], [`replaying_changes_from`] |
//! | Call into a figure that a draw uses | [`reenter`], [`FigureInUse`], [`Access`] |
//! | Follow changes | [`take_changed_figures`], [`unseen`], [`unreported`], [`stale_reports_asked`] |
//! | Make the renderer of one page | [`svg_page`], [`ps_page`], [`pgf_page`] |
//! | Prepare text for LaTeX | [`pgf_preamble`], [`pgf_text`], [`measure_pgf_text`], [`log_unknown_font`] |
//! | Distill PostScript | [`DistillOptions`], [`check_distiller`], [`distill_file`] |

mod archive;
mod artists;
mod autofmt;
mod canvas;
mod cell;
mod clear;
mod detach;
mod draw_hook;
mod draw_order;
mod drawn_changes;
mod frame;
mod in_use;
mod lend;
mod mosaic;
mod mosaic_axes;
mod no_output;
mod pages;
mod painting;
mod pdf_pages;
mod pgf;
mod pgf_latex;
mod pgf_metrics;
mod postscript;
mod print;
mod rect;
mod save;
mod size;
mod stale;
mod stale_reports;
mod standing_in;
pub(crate) use standing_in::StandingIn;
mod state;
mod subfigure;
mod subplots;
mod svg;
#[cfg(test)]
mod tests;
mod texts;

use std::fmt;
use std::ops::ControlFlow;
use std::rc::Rc;

use pltrs_core::Rgba;
use pltrs_raster::RasterError;
use pltrs_render::{RasterRenderer, Renderer};

pub use canvas::{BufferRegion, CanvasError, FigureCanvas, PixelBox};
pub(crate) use canvas::{CanvasState, Presentation, Presenting};
pub(crate) use cell::FigureCell;
pub use cell::{WeakFigure, take_changed_figures};
pub(crate) use detach::attach;
pub use draw_hook::DrawHook;
pub(crate) use draw_order::draws_before;
pub(crate) use drawn_changes::next_draw_tick;
pub use drawn_changes::{
    clear_draw_end_handler, draw_tick, replaying_changes_from, set_draw_end_handler,
};
pub(crate) use frame::{FigureFrame, SuperText};
pub use in_use::{Access, FigureInUse, reenter};
pub use mosaic::{Mosaic, MosaicCell, MosaicError, MosaicOptions};
pub use mosaic_axes::MosaicAxes;
pub use pages::{pgf_page, ps_page, svg_page};
pub use pdf_pages::PdfPages;
pub use pgf::{PgfTexSystem, log_unknown_font, pgf_preamble, pgf_text};
pub use pgf_latex::{PdfToPng, PgfPages};
pub use pgf_metrics::{PgfTextMetrics, measure_pgf_text};
pub use pltrs_render::{
    ImageFile, Metadata, PageOrientation, PageSetupError, Paper, PngMetadata, SvgFontType,
    SvgMetadata, UnknownSvgFontType,
};
pub use postscript::{DistillOptions, ProgramError, check_distiller, distill_file};
pub(crate) use postscript::{Executable, Workspace};
pub use print::{BboxInches, PadInches, SaveOptions};
pub use rect::AxesRectError;
pub(crate) use rect::axes_bounds;
use save::whole_pixels;
pub use save::{SaveError, SaveFormat};
pub use size::{FigSize, SizeError, SizeUnit};
pub(crate) use stale::FreshArtists;
pub use stale::unseen;
pub use stale_reports::{stale_reports_asked, unreported};
pub(crate) use state::{FigureState, Painting, StateRole, host_to_figure};
pub use subfigure::SubfigureOptions;
pub use subplots::Share;
pub(crate) use subplots::{TITLE_PLACES, axes_title, insert_axes, make_axes};

use crate::artist::{
    Artist, ArtistHandle, ArtistId, ArtistLookup, DrawError, DrawErrorKind, RawHandle, TypedHandle,
    downcast_ref,
};
use crate::axes::{Axes, AxesState, PlotError};
use crate::colors::Color;
use crate::events::Event;
use crate::gridspec::{GridSpecError, SubplotOverrides, SubplotParams};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::transforms::Transform;

/// The problems found while drawing a figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawErrors(pub Vec<DrawError>);

impl fmt::Display for DrawErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(
            f,
            "could not draw {} artist(s): {}",
            self.0.len(),
            messages.join("; ")
        )
    }
}

impl std::error::Error for DrawErrors {}

impl DrawErrors {
    /// The outcome `drawn` of drawing `figure` into `renderer` with, first,
    /// the memory the renderer was refused meanwhile (a canvas for
    /// filtered artists, say: matplotlib's `MemoryError`) and the paths
    /// too large for Agg (matplotlib's `OverflowError`); the rasterizer's
    /// other refusals are dropped, as figure draws do not report them.
    pub(crate) fn with_refused_memory(
        drawn: Result<(), Self>,
        renderer: &mut RasterRenderer,
        figure: ArtistId,
    ) -> Result<(), Self> {
        let mut errors: Vec<DrawError> = renderer
            .take_errors()
            .into_iter()
            .filter_map(|err| {
                let kind = match err {
                    RasterError::OutOfMemory(_) => DrawErrorKind::OutOfMemory,
                    RasterError::UnsplittablePath { .. } => DrawErrorKind::Overflow,
                    _ => return None,
                };
                Some(DrawError {
                    artist: figure,
                    message: err.to_string(),
                    kind,
                })
            })
            .collect();
        if let Err(Self(others)) = drawn {
            errors.extend(others);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Self(errors))
        }
    }
}

/// A figure is the whole image, with its background, its axes and the other
/// artists on it.
///
/// A figure has a size in inches and a resolution in dots per inch (dpi).
/// The size in pixels is the size in inches times the resolution. For
/// example, a figure of 6 by 4 inches at 100 dpi has 600 by 400 pixels. The
/// figure holds axes, and each axes holds the plots of your data.
///
/// `Figure` is a cheap handle. Its clones refer to the same figure. A
/// subfigure is also a `Figure`, for a part of its parent figure.
///
/// ```
/// use pltrs::figure::Figure;
///
/// let fig = Figure::with_size(6.0, 4.0, 100.0);
/// let axes = fig.subplots(2, 3)?;
/// axes[(0, 0)].plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
/// fig.suptitle("Six panels");
/// let png: Vec<u8> = fig.to_png()?; // `fig.savefig("plot.png")?` writes a file
/// assert!(png.starts_with(b"\x89PNG"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Make a figure
///
/// A new figure takes its size, resolution and colors from the runtime
/// settings `figure.figsize`, `figure.dpi`, `figure.facecolor` and
/// `figure.edgecolor` ([`rc`](crate::rc)).
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`new`](Self::new) | Makes a figure with the size and resolution of the runtime settings. |
/// | [`with_size`](Self::with_size) | Makes a figure with a size in inches and a resolution. |
/// | [`with_figsize`](Self::with_figsize) | Makes a figure with a size in inches, centimeters or pixels ([`FigSize`]). |
/// | [`size_inches`](Self::size_inches), [`set_size_inches`](Self::set_size_inches) | Get or set the size in inches. |
/// | [`set_figwidth`](Self::set_figwidth), [`set_figheight`](Self::set_figheight) | Set the width or the height in inches. |
/// | [`size_pixels`](Self::size_pixels), [`width_height`](Self::width_height) | Give the size in pixels, as a decimal value or in whole pixels. |
/// | [`dpi`](Self::dpi), [`set_dpi`](Self::set_dpi) | Get or set the resolution. |
/// | [`facecolor`](Self::facecolor), [`set_facecolor`](Self::set_facecolor) | Get or set the background color. |
/// | [`edgecolor`](Self::edgecolor), [`set_edgecolor`](Self::set_edgecolor) | Get or set the color of the frame around the figure. |
/// | [`linewidth`](Self::linewidth), [`set_linewidth`](Self::set_linewidth) | Get or set the width of that frame in points. |
/// | [`frameon`](Self::frameon), [`set_frameon`](Self::set_frameon) | Get or set whether pltrs draws the background and the frame. |
/// | [`patch_style`](Self::patch_style), [`set_patch_style`](Self::set_patch_style) | Get or set the style of the background patch. |
/// | [`patch_outline`](Self::patch_outline), [`set_patch_outline`](Self::set_patch_outline) | Get or set a shape for the background other than a rectangle. |
/// | [`patch_gid`](Self::patch_gid), [`set_patch_gid`](Self::set_patch_gid) | Get or set the SVG id of the background. |
///
/// # Add axes
///
/// Axes are the areas that show data. Each method adds axes to the figure
/// and returns them. A method that makes a grid returns a
/// [`Grid`](crate::numeric::Grid). Index it by `(row, column)`.
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`subplot`](Self::subplot) | Adds one axes in the default subplot area. |
/// | [`subplots`](Self::subplots) | Adds a grid of `nrows` by `ncols` axes. |
/// | [`subplots_shared`](Self::subplots_shared) | Adds a grid of axes that share their x or y axes ([`Share`]). |
/// | [`add_subplot`](Self::add_subplot) | Adds axes in one cell of a grid. Cells count from 1, row by row. |
/// | [`add_subplot_span`](Self::add_subplot_span) | Adds axes over a range of cells of a grid. |
/// | [`add_gridspec`](Self::add_gridspec) | Makes a grid specification ([`GridSpec`](crate::gridspec::GridSpec)). [`GridSpec::with_ratios`](crate::gridspec::GridSpec::with_ratios) makes one with width and height ratios. |
/// | [`add_subplot_spec`](Self::add_subplot_spec) | Adds axes on one cell or a block of cells of a grid specification. |
/// | [`subplots_on`](Self::subplots_on) | Adds one axes for each cell of a grid specification. |
/// | [`subplot_mosaic`](Self::subplot_mosaic) | Adds axes from a layout of labels, such as `"AAB;CCB"`, and gives them by label ([`MosaicAxes`]). |
/// | [`add_axes`](Self::add_axes) | Adds axes at `[left, bottom, width, height]` in figure fractions. |
/// | [`subfigures`](Self::subfigures) | Divides the figure into a grid of subfigures. Each subfigure has its own axes, titles and legends. |
/// | [`add_subfigure`](Self::add_subfigure), [`subfigs`](Self::subfigs) | Add one subfigure, or give the subfigures. |
/// | [`gca`](Self::gca), [`current_axes`](Self::current_axes), [`sca`](Self::sca) | Get or set the current axes. |
/// | [`axes`](Self::axes), [`axes3d`](Self::axes3d) | Give all axes, or all 3-D axes. |
/// | [`delaxes`](Self::delaxes) | Removes axes from the figure. |
///
/// ```
/// use pltrs::figure::Figure;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
/// ax.set_xlim(0.0, 10.0)?;
/// assert_eq!(ax.xlim(), (0.0, 10.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Axes with another projection have their own methods:
///
/// | Projection | Methods |
/// |------------|---------|
/// | Polar | [`polar_subplot`](Self::polar_subplot), [`polar_subplots`](Self::polar_subplots), [`add_polar_subplot`](Self::add_polar_subplot), [`add_polar_subplot_spec`](Self::add_polar_subplot_spec), [`add_polar_axes`](Self::add_polar_axes) |
/// | 3-D | [`subplot_3d`](Self::subplot_3d), [`subplot_3d_with`](Self::subplot_3d_with), [`add_subplot_3d`](Self::add_subplot_3d), [`add_subplot_spec_3d`](Self::add_subplot_spec_3d), [`add_axes_3d`](Self::add_axes_3d) |
/// | Geographic: Aitoff, Hammer, Lambert, Mollweide | [`geo_subplot`](Self::geo_subplot), [`add_geo_subplot`](Self::add_geo_subplot), [`add_geo_subplot_spec`](Self::add_geo_subplot_spec), [`add_geo_axes`](Self::add_geo_axes) |
/// | Any projection, by name or by value | [`subplot_with_projection`](Self::subplot_with_projection), [`subplots_with_projection`](Self::subplots_with_projection), [`add_subplot_with_projection`](Self::add_subplot_with_projection), [`add_subplot_spec_with_projection`](Self::add_subplot_spec_with_projection), [`subplots_on_with_projection`](Self::subplots_on_with_projection), [`add_axes_with_projection`](Self::add_axes_with_projection) |
/// | Any projection, sharing axes | [`add_subplot_spec_sharing`](Self::add_subplot_spec_sharing), [`add_axes_sharing`](Self::add_axes_sharing) |
///
/// # Add titles, legends and other figure artists
///
/// These artists belong to the figure, not to one axes. Their positions are
/// in figure fractions. The point `(0, 0)` is the bottom-left corner, and
/// `(1, 1)` is the top-right corner.
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`suptitle`](Self::suptitle), [`supxlabel`](Self::supxlabel), [`supylabel`](Self::supylabel) | Set the title, the x label and the y label of the whole figure. |
/// | [`set_suptitle_y`](Self::set_suptitle_y), [`set_supxlabel_y`](Self::set_supxlabel_y), [`set_supylabel_x`](Self::set_supylabel_x) | Move the title or a label. |
/// | [`suptitle_text`](Self::suptitle_text), [`supxlabel_text`](Self::supxlabel_text), [`supylabel_text`](Self::supylabel_text) | Give the text of the title or a label. |
/// | [`text`](Self::text), [`texts`](Self::texts) | Add text at a point, or give the texts of the figure. |
/// | [`legend`](Self::legend), [`legends`](Self::legends) | Add a legend for the labeled artists of all axes, or give the legends. |
/// | [`colorbar`](Self::colorbar) | Adds a colorbar for an image, a contour set or another color-mapped artist. |
/// | [`figimage`](Self::figimage), [`images`](Self::images) | Add an image in figure pixels without resampling, or give these images. |
/// | [`set_suppress_composite`](Self::set_suppress_composite), [`suppress_composite`](Self::suppress_composite) | Set or get whether pltrs combines adjacent images into one. |
/// | [`add_line`](Self::add_line), [`add_patch`](Self::add_patch), [`add_artist`](Self::add_artist) | Add a line or a patch. |
/// | [`add_custom_artist`](Self::add_custom_artist) | Adds an artist type of your own (the [`Artist`] trait). |
/// | [`add_anchored_box`](Self::add_anchored_box), [`add_offset_box`](Self::add_offset_box), [`add_annotation_bbox`](Self::add_annotation_bbox) | Add an offset box, such as a box of text or an image that points to a location. |
/// | [`autofmt_xdate`](Self::autofmt_xdate) | Turns long date tick labels so that they do not overlap. |
///
/// # Arrange the axes
///
/// A layout engine moves the axes at each draw so that their labels do not
/// overlap. The modern look uses constrained layout. The matplotlib look
/// uses no layout engine.
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`set_layout_engine`](Self::set_layout_engine), [`layout_engine`](Self::layout_engine) | Set or get the layout engine, such as constrained layout or tight layout ([`LayoutEngine`](crate::layout::LayoutEngine)). |
/// | [`tight_layout`](Self::tight_layout) | Runs tight layout once. |
/// | [`subplots_adjust`](Self::subplots_adjust) | Changes the margins and the spacing of the subplot grids, and moves the axes. |
/// | [`subplot_params`](Self::subplot_params), [`set_subplot_params`](Self::set_subplot_params) | Get or set the margins and the spacing for the axes that you add next. |
/// | [`align_labels`](Self::align_labels), [`align_xlabels`](Self::align_xlabels), [`align_ylabels`](Self::align_ylabels) | Align the axis labels of subplots. |
/// | [`align_titles`](Self::align_titles) | Aligns the titles of the subplots in a row. |
/// | [`layout`](Self::layout) | Runs the layout without drawing. After it, positions, limits and ticks have their final values. |
/// | [`bbox`](Self::bbox), [`tight_bbox`](Self::tight_bbox) | Give the box of the figure in pixels, or the box of all visible artists in inches. |
///
/// # Save the figure
///
/// [`savefig`](Self::savefig) writes a file in the format of its extension.
/// [`savefig_with`](Self::savefig_with) and each `_with` method take
/// [`SaveOptions`]. The options set the resolution, a tight crop
/// ([`BboxInches::Tight`]), a transparent background and the background
/// color. They also set the document metadata, the JPEG and AVIF quality
/// and the PostScript page. Each format also has a method that returns the
/// file contents:
///
/// | Format | Extensions | Into memory |
/// |--------|------------|-------------|
/// | PNG | `.png` | [`to_png`](Self::to_png), [`to_png_with`](Self::to_png_with) |
/// | PDF | `.pdf` | [`to_pdf`](Self::to_pdf), [`to_pdf_with`](Self::to_pdf_with) |
/// | SVG | `.svg` | [`to_svg`](Self::to_svg), [`to_svg_with`](Self::to_svg_with), [`to_svg_document`](Self::to_svg_document) |
/// | PostScript | `.ps` | [`to_ps`](Self::to_ps), [`to_ps_with`](Self::to_ps_with) |
/// | Encapsulated PostScript | `.eps` | [`to_eps`](Self::to_eps), [`to_eps_with`](Self::to_eps_with) |
/// | PGF picture for LaTeX | `.pgf` | [`to_pgf`](Self::to_pgf), [`to_pgf_with`](Self::to_pgf_with), [`to_pgf_document`](Self::to_pgf_document) |
/// | Compressed SVG, JPEG, TIFF, GIF, lossless WebP, AVIF, raw RGBA | `.svgz`, `.jpg`, `.tif`, `.gif`, `.webp`, `.avif`, `.raw` | [`to_format`](Self::to_format) |
///
/// [`to_format`](Self::to_format) gives each format with a [`SaveFormat`].
/// If LaTeX is installed, [`to_pgf_pdf`](Self::to_pgf_pdf),
/// [`to_pgf_pdf_with`](Self::to_pgf_pdf_with),
/// [`to_pgf_png`](Self::to_pgf_png) and
/// [`to_pgf_png_with`](Self::to_pgf_png_with) give a PDF or PNG file with
/// text that LaTeX typesets. [`PdfPages`] collects several figures as the
/// pages of one PDF document.
///
/// ```
/// use pltrs::figure::{BboxInches, Figure, SaveOptions};
///
/// let fig = Figure::new();
/// fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
/// let options = SaveOptions {
///     dpi: Some(200.0),
///     bbox_inches: Some(BboxInches::Tight),
///     ..SaveOptions::default()
/// };
/// let pdf = fig.to_pdf_with(options)?; // `fig.savefig_with("plot.pdf", options)?` writes a file
/// assert!(pdf.starts_with(b"%PDF"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// To draw the figure yourself, use these methods:
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`render_raster`](Self::render_raster), [`render_raster_with`](Self::render_raster_with) | Draw the figure into pixels, as a PNG save does. |
/// | [`raster_canvas`](Self::raster_canvas) | Makes an empty raster renderer with the size of the figure. |
/// | [`draw`](Self::draw) | Draws the figure through a [`Renderer`] of your choice. |
/// | [`draw_without_rendering`](Self::draw_without_rendering) | Draws the figure without output, so that artists get their final size. |
///
/// # Show the figure and react to input
///
/// [`gui::show`](crate::gui::show) shows figures in windows and returns
/// when the user closes them. [`web::serve`](crate::web::serve) shows them
/// in a web browser. [`animation`](crate::animation) plays and saves
/// animations. Synthetic input from an
/// [`EventSource`](crate::events::EventSource) calls the same callbacks
/// without a window.
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`connect`](Self::connect), [`disconnect`](Self::disconnect) | Call a closure for each event of one kind ([`EventKind`](crate::events::EventKind)), such as a key press, a click or a draw. |
/// | [`process_event`](Self::process_event) | Sends an event to the connected callbacks. |
/// | [`block_callbacks`](Self::block_callbacks), [`with_callbacks_blocked`](Self::with_callbacks_blocked) | Stop the callbacks for a time. |
/// | [`pick`](Self::pick) | Sends pick events for the artists under the pointer that have a picker. |
/// | [`add_timer`](Self::add_timer), [`remove_timer`](Self::remove_timer), [`timers`](Self::timers), [`fire_timer`](Self::fire_timer) | Call a closure at an interval while the figure is shown. |
/// | [`canvas`](Self::canvas) | Gives the [`FigureCanvas`]. Use it to draw only the artists that change (blitting). |
/// | [`draw_artist`](Self::draw_artist), [`draw_cell`](Self::draw_cell) | Draw one artist or one table cell onto the canvas. |
/// | [`add_tool`](Self::add_tool), [`remove_tool`](Self::remove_tool) | Add a toolbar button and keyboard shortcut of your own to the windows, or remove one. |
/// | [`tool`](Self::tool), [`tools`](Self::tools), [`tool_names`](Self::tool_names) | Give the tools of the figure. |
/// | [`set_tool_keys`](Self::set_tool_keys) | Changes the keyboard shortcuts of a tool. |
/// | [`trigger_tool`](Self::trigger_tool), [`tool_toggled`](Self::tool_toggled), [`active_toggle`](Self::active_toggle) | Use a tool, or find which toggle tools are on. |
/// | [`on_tool_change`](Self::on_tool_change), [`disconnect_tool_change`](Self::disconnect_tool_change), [`tool_message`](Self::tool_message) | Follow the changes to the tools, and show a message in the status line. |
/// | [`grab_mouse`](Self::grab_mouse), [`release_mouse`](Self::release_mouse), [`mouse_grabber`](Self::mouse_grabber) | Send all mouse events to one axes. |
/// | [`set_capture_keys`](Self::set_capture_keys), [`captures_keys`](Self::captures_keys) | Send key presses to a widget instead of the window shortcuts. |
/// | [`lock_widgets`](Self::lock_widgets), [`release_widgets`](Self::release_widgets), [`widgets_available`](Self::widgets_available), [`widgets_locked`](Self::widgets_locked) | Let one widget use the pointer while the others wait. |
/// | [`set_cursor`](Self::set_cursor), [`cursor`](Self::cursor) | Set or get the cursor shape over the figure. |
///
/// # Find, remove and clear artists
///
/// `Figure` implements [`ArtistHandle`]. Thus
/// [`findobj`](ArtistHandle::findobj) finds the artists of the figure that
/// match a condition. [`ArtistHandle::remove`] removes any artist.
///
/// | Method | What it does |
/// |--------|--------------|
/// | [`axes`](Self::axes), [`texts`](Self::texts), [`legends`](Self::legends), [`images`](Self::images), [`subfigs`](Self::subfigs) | Give the axes, texts, legends, images or subfigures. |
/// | [`artists`](Self::artists) | Gives the other artists that you added to the figure. |
/// | [`current_image`](Self::current_image) | Gives the image of the current axes, or else of the last axes with an image. |
/// | [`contains_artist`](Self::contains_artist) | Tells if an artist is in this figure or in one of its subfigures. |
/// | [`attach`](Self::attach) | Adds a removed artist back to the figure. |
/// | [`clear`](Self::clear), [`clf`](Self::clf) | Remove all axes and artists, and keep the size, the colors and the layout engine. |
/// | [`is_stale`](Self::is_stale) | Tells if the figure changed after the last draw. |
///
/// # Archive a figure
///
/// [`to_archive`](Self::to_archive) writes the figure and all its artists
/// to bytes. [`from_archive`](Self::from_archive) reads them back, as
/// matplotlib reads a pickled figure. [`downgrade`](Self::downgrade) gives a
/// [`WeakFigure`], which does not keep the figure alive.
///
/// # The look
///
/// pltrs has two looks: the modern look and the matplotlib look
/// ([`look`](crate::look)). A look sets the defaults of the runtime
/// settings. Thus it sets the fonts, colors, grid and layout engine of new
/// figures. Select the look before you make a figure.
///
/// # Methods for programs that wrap pltrs
///
/// pltrs (Python) uses these methods to give matplotlib objects to Python
/// code. Most Rust programs do not need them.
///
/// | Purpose | Methods |
/// |---------|---------|
/// | Make axes outside the figure, then add them with [`attach`](Self::attach) | [`removed_axes`](Self::removed_axes), [`removed_subplot`](Self::removed_subplot), [`removed_axes_3d`](Self::removed_axes_3d), [`removed_subplot_3d`](Self::removed_subplot_3d), [`removed_axes_with_projection`](Self::removed_axes_with_projection), [`removed_subplot_with_projection`](Self::removed_subplot_with_projection) |
/// | Add artists that were made outside any figure | [`adopt_artist`](Self::adopt_artist), [`take_artist`](Self::take_artist), [`list_among_texts`](Self::list_among_texts) |
/// | Run code of the program during layout and drawing | [`set_layout_hook`](Self::set_layout_hook), [`set_draw_hook`](Self::set_draw_hook), [`set_evaluation_hook`](Self::set_evaluation_hook), [`set_tick_updater`](Self::set_tick_updater) |
/// | Follow changes | [`report_stale`](Self::report_stale), [`take_stale_reports`](Self::take_stale_reports), [`watch_changes`](Self::watch_changes) |
/// | Read transforms | [`resolve_transform`](Self::resolve_transform), [`resolve_transform_as_is`](Self::resolve_transform_as_is), [`tree_leaves`](Self::tree_leaves) |
/// | Read the layout | [`execute_layout`](Self::execute_layout), [`set_layout_when_drawn_only`](Self::set_layout_when_drawn_only), [`tight_layout_params`](Self::tight_layout_params), [`tight_extent`](Self::tight_extent), [`print_window`](Self::print_window), [`super_text_artists`](Self::super_text_artists) |
/// | Align texts of axes on different grids | [`join_aligned`](Self::join_aligned), [`aligned_with`](Self::aligned_with) |
/// | Draw parts of a figure | [`render_raster_partial`](Self::render_raster_partial), [`draw_artist_unpainted`](Self::draw_artist_unpainted), [`pick_order`](Self::pick_order) |
#[derive(Clone)]
pub struct Figure {
    state: Rc<FigureCell>,
    root: RawHandle,
}

impl fmt::Debug for Figure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        f.debug_struct("Figure")
            .field("size_inches", &state.size_inches)
            .field("dpi", &state.dpi())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Figure {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Default for Figure {
    fn default() -> Self {
        Self::new()
    }
}

impl Figure {
    /// A figure of size `figure.figsize` at `figure.dpi`, coloured by
    /// `figure.facecolor` and `figure.edgecolor`.
    pub fn new() -> Self {
        let rc = crate::rc::params();
        let [width, height] = rc.figure_figsize;
        Self::with_size(width, height, rc.figure_dpi)
    }

    /// A figure of the given size in inches and resolution.
    pub fn with_size(width: f64, height: f64, dpi: f64) -> Self {
        let rc = crate::rc::params();
        // The rc colours are resolved now, as matplotlib's figure patch
        // resolves its colours when made.
        let color = |c: &Color| Color::from(c.resolve().unwrap_or(Rgba::WHITE));
        let state = FigureState::new(
            (width, height),
            dpi,
            color(&rc.figure_facecolor),
            color(&rc.figure_edgecolor),
        );
        let root = state.root;
        let state = Rc::new(FigureCell::new(state));
        let root = RawHandle::new(Rc::clone(&state), root);
        Self { state, root }
    }

    /// Canvas size in inches.
    pub fn size_inches(&self) -> (f64, f64) {
        self.state.borrow().size_inches
    }

    /// Canvas resolution in dots per inch.
    pub fn dpi(&self) -> f64 {
        self.state.borrow().dpi()
    }

    /// Changes the canvas resolution. As in matplotlib, saving with
    /// `savefig.dpi: figure` keeps the resolution the figure was made at:
    /// give [`SaveOptions::dpi`] to save at another.
    ///
    /// ```
    /// use pltrs::figure::{Figure, SaveOptions};
    /// let fig = Figure::with_size(2.0, 1.0, 100.0);
    /// fig.set_dpi(50.0);
    /// assert_eq!(fig.width_height(), (100, 50));
    /// let saved = fig.render_raster_with(SaveOptions::default())?.to_image();
    /// assert_eq!((saved.width(), saved.height()), (200, 100));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_dpi(&self, dpi: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.set_dpi(dpi);
        state.stale = true;
        self
    }

    /// Canvas size in pixels.
    pub fn size_pixels(&self) -> (f64, f64) {
        self.state.borrow().size_pixels()
    }

    /// The whole pixels of a raster canvas for this figure (matplotlib's
    /// `canvas.get_width_height()`): the size in pixels truncated, but
    /// rounded up when within 1e-8 of the next pixel, so that a 9.2 in
    /// figure at 100 dpi spans 920 pixels despite floating-point error.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// assert_eq!(Figure::with_size(9.2, 5.0, 100.0).width_height(), (920, 500));
    /// assert_eq!(Figure::with_size(2.555, 1.0, 100.0).width_height(), (255, 100));
    /// ```
    pub fn width_height(&self) -> (u32, u32) {
        let (width, height) = self.size_pixels();
        (whole_pixels(width), whole_pixels(height))
    }

    /// Sets the background colour (matplotlib's `set_facecolor`); a
    /// colour that does not resolve is reported when the figure is drawn.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.style.facecolor = Some(color));
        self
    }

    /// Sets the colour of the frame around the canvas (matplotlib's
    /// `set_edgecolor`); it shows once the frame has a width.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.style.edgecolor = Some(color));
        self
    }

    /// Sets the width of the frame around the canvas in points
    /// (matplotlib's `set_linewidth`).
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.style.linewidth = Some(width));
        self
    }

    /// The width of the frame around the canvas in points (matplotlib's
    /// `get_linewidth`).
    pub fn linewidth(&self) -> f64 {
        self.root.with::<FigureFrame, _>(|frame| {
            (frame.style.linewidth).unwrap_or_else(|| crate::rc::params().patch_linewidth)
        })
    }

    /// Sets whether the background and the frame around the canvas are
    /// drawn (matplotlib's `set_frameon`); without them the canvas stays
    /// transparent where nothing else is drawn.
    pub fn set_frameon(&self, frameon: bool) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.frameon = frameon);
        self
    }

    /// Whether the background and the frame around the canvas are drawn;
    /// initially they are, whatever `figure.frameon` says, as in figures
    /// made by pyplot's `figure`.
    pub fn frameon(&self) -> bool {
        self.root.with::<FigureFrame, _>(|frame| frame.frameon)
    }

    /// The background colour.
    pub fn facecolor(&self) -> Color {
        self.root.with::<FigureFrame, _>(|frame| {
            (frame.style.facecolor.clone())
                .unwrap_or_else(|| crate::rc::params().patch_facecolor.clone())
        })
    }

    /// The colour of the frame around the canvas.
    pub fn edgecolor(&self) -> Color {
        self.root
            .with::<FigureFrame, _>(|frame| frame.style.edge_spec(&crate::rc::params(), false))
    }

    /// The margins and spacing of subplot grids.
    pub fn subplot_params(&self) -> SubplotParams {
        self.root
            .with::<FigureFrame, _>(|frame| frame.subplot_params)
    }

    /// Changes the subplot margins and spacing of the figure and its
    /// subfigures, which share them as in matplotlib, and moves every
    /// grid-placed axes accordingly (`fig.subplots_adjust(...)`).
    pub fn subplots_adjust(&self, overrides: SubplotOverrides) -> Result<&Self, GridSpecError> {
        let params = self.subplot_params().updated(overrides)?;
        self.state.borrow_mut().set_subplot_params(params)?;
        Ok(self)
    }

    /// Changes the subplot margins and spacing of this figure or subfigure
    /// alone, for the subplots added after and the next
    /// [`subplots_adjust`](Self::subplots_adjust), leaving the axes placed
    /// so far where they are (matplotlib's `fig.subplotpars =
    /// SubplotParams(...)`: the figure keeps the object it is given).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::gridspec::SubplotOverrides;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let placed = ax.position();
    /// fig.set_subplot_params(SubplotOverrides { left: Some(0.3), ..Default::default() })?;
    /// assert_eq!(ax.position(), placed);
    /// assert_eq!(fig.subplot_params().left, 0.3);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the margins leave no room (left not below right, bottom not
    /// below top).
    pub fn set_subplot_params(&self, overrides: SubplotOverrides) -> Result<&Self, GridSpecError> {
        let params = self.subplot_params().updated(overrides)?;
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.subplot_params = params);
        Ok(self)
    }

    /// Adds a line through `(x, y)` in figure fractions, styled from the
    /// rc parameters and `props` without the property cycle (matplotlib's
    /// `fig.add_artist(Line2D(...))`).
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn add_line(&self, x: &[f64], y: &[f64], props: LineProps) -> Result<Line2D, PlotError> {
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let mut line = LineArtist::new(x.to_vec(), y.to_vec(), crate::rc::params());
        props.apply(&mut line);
        if line.base().transform.is_none() {
            line.base_mut().transform = Some(Transform::Figure(self.root.id()));
        }
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(line));
        Ok(Line2D::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
    }

    /// Adds a patch in figure fractions (matplotlib's
    /// `fig.add_artist(patch)`); give it another coordinate system with
    /// [`ArtistHandle::set_transform`].
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        patch.base_mut().transform = Some(Transform::Figure(self.root.id()));
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(patch));
        Patch::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// Every axes of the figure, in creation order.
    /// The figure lists the axes of its subfigures too (matplotlib's
    /// shared axes stack); a subfigure lists only its own.
    pub fn axes(&self) -> Vec<Axes> {
        self.scoped::<AxesState>()
            .into_iter()
            .map(|id| Axes::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
            .collect()
    }

    /// The artists of type `T` placed directly in this figure's frame or,
    /// for the root figure, in any frame (the scope of [`Figure::axes`]).
    pub(crate) fn scoped<T: Artist>(&self) -> Vec<ArtistId> {
        let state = self.state.borrow();
        let frame_id = self.root.id();
        if !state.is_attached(frame_id) {
            // A removed subfigure still holds its artists.
            let children = self
                .root
                .with::<FigureFrame, _>(|frame| frame.children.clone());
            return children
                .into_iter()
                .filter(|&id| state.artist_as::<T>(id).is_some())
                .collect();
        }
        let in_scope = |parent: Option<ArtistId>| match parent {
            Some(parent) if frame_id == state.root => {
                state.store.get_as::<FigureFrame>(parent).is_some()
            }
            Some(parent) => parent == frame_id,
            None => false,
        };
        state
            .store
            .ids()
            .filter(|&id| {
                state
                    .store
                    .get_as::<T>(id)
                    .is_some_and(|artist| in_scope(artist.base().parent))
            })
            .collect()
    }

    /// Renders the figure through `renderer`, then sends [`Event::Draw`]
    /// to the figure's callbacks (matplotlib's `draw_event`).
    ///
    /// Everything drawable is drawn; artists with invalid properties (for
    /// example an unknown colour name) are skipped and reported.
    pub fn draw(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        let drawn = self.draw_without_event(renderer);
        self.process_event(&Event::Draw);
        drawn
    }

    /// [`Figure::draw`] without the draw event, for callers that send it
    /// themselves once they are done.
    /// A hook of the program's stopping the draw (see
    /// [`DrawHook::stopping`]) leaves the rest of it undone.
    pub(crate) fn draw_without_event(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        if self.prepare_layout().is_break() {
            return Ok(());
        }
        // A layout that failed as matplotlib's engine fails ends the draw
        // before anything is drawn, as the exception ends matplotlib's.
        let failed = self.state.borrow_mut().take_layout_failure();
        if let Some(failure) = failed {
            return Err(DrawErrors(vec![failure]));
        }
        crate::inset::choose_connectors(&mut self.state.borrow_mut());
        if self.run_draw_hook().is_break() {
            return Ok(());
        }
        // The program's drawing runs in each of matplotlib's draws this
        // one stands for (a print drawn once before lays it out), the
        // last one rendered; those pltrs does not paint shape the artists
        // with the program's styles too. A print cut to a window ran the
        // draw before as it began (`Figure::predraw_program`).
        let draws = crate::text::font_notes::draws_of_print();
        let first = if crate::text::font_notes::program_predrawn() {
            draws
        } else {
            1
        };
        for draw in first..=draws {
            let rendered = draw == draws;
            if !rendered {
                self.root_handle().predraw_program_shapes();
            }
            self.root_handle()
                .prepare_painters(&*renderer, true, rendered);
        }
        let errors = self.state.draw(renderer);
        // The changes the program's code asked for meanwhile, made before
        // the figure counts as drawn (see `drawn_changes`).
        drawn_changes::run_draw_end_handler();
        let mut state = self.state.borrow_mut();
        state.stale = false;
        state.last_draw_dpi = Some(state.dpi());
        state.pending_errors.clear();
        state.aspects_failed.clear();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
    }

    /// Runs the layout pass (the layout engine, autoscaling and tick
    /// placement) without drawing, so positions, limits and ticks can be
    /// queried; from then on the figure counts as drawn (matplotlib's
    /// `draw_without_rendering`, or `canvas.draw()`).
    pub fn layout(&self) {
        if self.prepare_layout().is_break() {
            return;
        }
        let mut state = self.state.borrow_mut();
        state.last_draw_dpi = Some(state.dpi());
    }

    /// The layout pass of [`Figure::layout`], before a draw, then the
    /// offset boxes of the program's measured where the figure is laid
    /// out, then the limit callbacks its autoscaling queued; whether the
    /// draw goes on. A layout hook of the program's stopping the draw
    /// leaves the figure laid out, as matplotlib's draw fails in the
    /// program's code it runs once the figure is laid out.
    pub(crate) fn prepare_layout(&self) -> ControlFlow<()> {
        let flow = self.run_layout_hook();
        let painting = if flow.is_continue() {
            Painting::Painted
        } else {
            Painting::Stopped
        };
        // The program's measures, which only an engine laying the figure
        // out asks for while it is drawn (matplotlib's draws ask no artist
        // for its tight box otherwise), once per measuring pass; none in
        // the draw of a printed window, which keeps the layout measured
        // before (matplotlib prints with the engine turned off).
        let passes = if self.print_window().is_some() {
            0
        } else {
            self.layout_engine()
                .map_or(0, |engine| engine.measure_passes())
        };
        for _ in 0..passes {
            self.root_handle().measure_painters_for_layout();
        }
        self.state.release_unheld();
        self.state.work_on(|state| state.prepare_for_draw(painting));
        crate::offsetbox::measure_program_boxes(&self.root_handle());
        crate::axes::run_limit_callbacks(&self.state);
        flow
    }

    /// The handle of the root figure's frame (a subfigure's handle is
    /// its own frame's).
    pub(crate) fn root_handle(&self) -> RawHandle {
        let root = self.state().root;
        self.raw().sibling(root)
    }

    /// The root figure owning `state`.
    pub(crate) fn root_of(state: &Rc<FigureCell>) -> Self {
        let root = state.borrow().root;
        Self {
            state: Rc::clone(state),
            root: RawHandle::new(Rc::clone(state), root),
        }
    }

    /// The figure or subfigure holding `artist` most closely: the first
    /// figure frame among its ancestors, the figure itself for the root.
    pub(crate) fn parent_of(artist: &RawHandle) -> Self {
        let state = artist.figure_state();
        let frame = {
            let figure = state.borrow();
            let parent_of = |id| figure.artist(id).and_then(|a| a.base().parent);
            // An axes made outside its figure is for the (sub)figure it was
            // made for, and one removed from it still of the (sub)figure
            // it was in, as the artists removed with their subfigure are.
            let home = (figure.detached.get(artist.id()))
                .and_then(downcast_ref::<AxesState>)
                .and_then(|axes| axes.home);
            let mut ancestor = home.or_else(|| parent_of(artist.id()));
            loop {
                match ancestor {
                    Some(id) if figure.artist_as::<FigureFrame>(id).is_some() => break id,
                    Some(id) => ancestor = parent_of(id),
                    None => break figure.root,
                }
            }
        };
        Self {
            state: Rc::clone(state),
            root: RawHandle::new(Rc::clone(state), frame),
        }
    }

    /// Sends every mouse event to `ax` until [`Figure::release_mouse`]:
    /// their axes and data coordinates are those of `ax` wherever the
    /// pointer is (matplotlib's `canvas.grab_mouse`).
    pub fn grab_mouse(&self, ax: &Axes) {
        self.state.borrow_mut().mouse_grabber = Some(ax.id());
    }

    /// Ends a [`Figure::grab_mouse`] by `ax` (matplotlib's
    /// `canvas.release_mouse`).
    pub fn release_mouse(&self, ax: &Axes) {
        let mut state = self.state.borrow_mut();
        if state.mouse_grabber == Some(ax.id()) {
            state.mouse_grabber = None;
        }
    }

    /// The axes grabbing the mouse, if any.
    pub fn mouse_grabber(&self) -> Option<Axes> {
        let id = self.state.borrow().mouse_grabber?;
        self.axes().into_iter().find(|ax| ax.id() == id)
    }

    /// Makes a widget receive key presses without the window acting on its
    /// keyboard shortcuts, or gives them back (matplotlib's text boxes
    /// clear the `keymap.*` rcParams while typing).
    pub fn set_capture_keys(&self, capture: bool) -> &Self {
        self.state.borrow_mut().capture_keys = capture;
        self
    }

    /// Whether key presses are captured by a widget.
    pub fn captures_keys(&self) -> bool {
        self.state.borrow().capture_keys
    }

    /// Whether `artist` belongs to this figure (or one of its
    /// subfigures).
    pub fn contains_artist(&self, artist: &impl ArtistHandle) -> bool {
        Rc::ptr_eq(&self.state, artist.raw().figure_state())
    }

    /// The cell sharing the figure's state.
    pub(crate) fn state_cell(&self) -> &Rc<FigureCell> {
        &self.state
    }

    /// A handle that does not keep the figure alive (see [`WeakFigure`]).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let weak = fig.downgrade();
    /// assert!(weak.upgrade().is_some_and(|alive| alive == fig));
    /// drop(fig);
    /// assert!(weak.upgrade().is_none());
    /// ```
    pub fn downgrade(&self) -> WeakFigure {
        WeakFigure {
            state: Rc::downgrade(&self.state),
            frame: self.root.id(),
        }
    }

    /// Read access to the figure's state.
    pub(crate) fn state(&self) -> std::cell::Ref<'_, FigureState> {
        self.state.borrow()
    }

    /// Write access to the figure's state.
    pub(crate) fn state_mut(&self) -> std::cell::RefMut<'_, FigureState> {
        self.state.borrow_mut()
    }

    /// Whether something changed since the last draw.
    pub fn is_stale(&self) -> bool {
        self.state.borrow().stale
    }
}

/// A figure frame's handle is the figure or subfigure it frames.
impl TypedHandle for Figure {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<FigureFrame>().then(|| Self {
            state: Rc::clone(raw.figure_state()),
            root: raw.clone(),
        })
    }
}

impl ArtistHandle for Figure {
    fn raw(&self) -> &RawHandle {
        &self.root
    }
}
