//! Printing with options (matplotlib's `savefig` keywords): resolution,
//! cropping to the drawn area (`bbox_inches`), transparency and
//! background colours, for the raster canvas and the vector backends.
//! The options themselves are in `options`, the tight box in `tight`.

mod options;
mod tight;

pub use options::{BboxInches, PadInches, SaveOptions};

use pltrs_core::{Bbox, Rgba};
use pltrs_render::{MixedModeRenderer, PdfRenderer, RasterRenderer, Renderer, VectorError};

use super::save::SaveError;
use super::{DrawErrors, Figure};
use crate::axes::Axes;
use crate::colors::Color;
use crate::rc::{ColorOrAuto, Dpi, SavefigBbox};

/// What a save changes on the figure, put back when dropped (also when
/// rendering fails).
struct Saving<'a> {
    figure: &'a Figure,
    dpi: f64,
    colors: (Color, Color),
    /// The background and border colours of the subfigures made
    /// transparent.
    subfigure_colors: Vec<(Figure, Color, Color)>,
    /// The backgrounds of the axes made transparent.
    axes_faces: Vec<(Axes, Color)>,
}

impl Saving<'_> {
    /// Leaves out the backgrounds of every subfigure and axes, insets
    /// included, for a transparent save (matplotlib's
    /// `_recursively_make_subfig_transparent` and
    /// `_recursively_make_axes_transparent`).
    fn make_transparent(&mut self) {
        for sub in self.figure.subfigures_nested() {
            self.subfigure_colors
                .push((sub.clone(), sub.facecolor(), sub.edgecolor()));
            sub.set_facecolor(Rgba::TRANSPARENT)
                .set_edgecolor(Rgba::TRANSPARENT);
        }
        let mut pending = self.figure.axes();
        while let Some(ax) = pending.pop() {
            pending.extend(ax.child_axes());
            self.axes_faces.push((ax.clone(), ax.facecolor()));
            ax.set_facecolor(Color::none());
        }
    }
}

impl Drop for Saving<'_> {
    fn drop(&mut self) {
        let figure = self.figure;
        {
            let mut state = figure.state_mut();
            state.print_window = None;
            state.saving = false;
            state.draws_per_layout = 1;
        }
        figure.set_dpi(self.dpi);
        figure.set_facecolor(self.colors.0.clone());
        figure.set_edgecolor(self.colors.1.clone());
        for (sub, face, edge) in &self.subfigure_colors {
            sub.set_facecolor(face.clone()).set_edgecolor(edge.clone());
        }
        for (ax, face) in &self.axes_faces {
            ax.set_facecolor(face.clone());
        }
    }
}

/// The canvas a prepared print draws into.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Page {
    /// Width in inches.
    width: f64,
    /// Height in inches.
    height: f64,
    /// The save resolution (images of vector formats are rasterized at it).
    dpi: f64,
    /// Whether matplotlib draws the figure once before printing it,
    /// without rendering (with a layout engine, or to cut it to its tight
    /// box): a draw failing there fails before anything is printed.
    predrawn: bool,
}

/// The figure's colour `figure`, unless the rcParam asks for a specific
/// one.
fn rc_color(setting: &ColorOrAuto, figure: &Color) -> Color {
    match setting {
        ColorOrAuto::Auto => figure.clone(),
        ColorOrAuto::Color(color) => color.clone(),
    }
}

/// The resolution vector backends draw at: one unit per point.
const POINTS_PER_INCH: f64 = 72.0;

impl Figure {
    /// The resolution the figure is saved at with `options`: theirs, else
    /// `savefig.dpi` (`figure` being the one the figure was made at).
    pub(crate) fn save_dpi(&self, options: &SaveOptions) -> f64 {
        options
            .dpi
            .unwrap_or(match crate::rc::params().savefig_dpi {
                Dpi::Value(dpi) => dpi,
                Dpi::Figure => self.state().original_dpi,
            })
    }

    /// The part of the figure a print running now is cut to, in inches
    /// from the figure's bottom-left corner (a tight box or the
    /// `bbox_inches` given); `None` outside prints and for whole figures.
    /// matplotlib's print moves `transFigure` by the box's corner then,
    /// and makes `fig.bbox` the box's size.
    #[must_use]
    pub fn print_window(&self) -> Option<Bbox> {
        self.state().print_window
    }

    /// The part of the figure `options` print: theirs, or `savefig.bbox`.
    fn bbox_inches(options: &SaveOptions) -> Option<BboxInches> {
        options.bbox_inches.or_else(|| {
            (crate::rc::params().savefig_bbox == SavefigBbox::Tight).then_some(BboxInches::Tight)
        })
    }

    /// Prepares the figure for printing with `options` as matplotlib's
    /// `print_figure` does: resolution (`savefig.dpi: figure` being the
    /// one the figure was made at), background colours and the part
    /// printed. The figure is drawn at `canvas_dpi` (the save resolution
    /// for raster output, 72 for the vector formats, which also measure
    /// the tight box at 72 dpi).
    fn begin_print(&self, options: &SaveOptions, canvas_dpi: Option<f64>) -> (Saving<'_>, Page) {
        let rc = crate::rc::params();
        let dpi = self.save_dpi(options);
        let canvas_dpi = canvas_dpi.unwrap_or(dpi);
        let transparent = options.transparent.unwrap_or(rc.savefig_transparent);
        let mut saving = Saving {
            figure: self,
            dpi: self.dpi(),
            colors: (self.facecolor(), self.edgecolor()),
            subfigure_colors: Vec::new(),
            axes_faces: Vec::new(),
        };
        self.state_mut().saving = true;
        if transparent {
            saving.make_transparent();
        }
        let (face, edge) = if transparent {
            (
                Color::from(Rgba::TRANSPARENT),
                Color::from(Rgba::TRANSPARENT),
            )
        } else {
            (
                options
                    .facecolor
                    .clone()
                    .unwrap_or_else(|| rc_color(&rc.savefig_facecolor, &saving.colors.0)),
                options
                    .edgecolor
                    .clone()
                    .unwrap_or_else(|| rc_color(&rc.savefig_edgecolor, &saving.colors.1)),
            )
        };
        self.set_dpi(canvas_dpi)
            .set_facecolor(face)
            .set_edgecolor(edge);
        let bbox = Self::bbox_inches(options);
        // matplotlib draws a figure with a layout engine, or to be cut to
        // its tight box, once before printing it (without rendering), and
        // prints it without the engine: the program's tick helpers see
        // both draws.
        let predrawn = self.layout_engine().is_some();
        match bbox {
            // The draw measuring the figure.
            Some(BboxInches::Tight) => self.layout(),
            // The layout the window is cut from, at the print resolution.
            Some(BboxInches::Inches(_)) => {
                self.state_mut().draws_per_layout = usize::from(predrawn);
                // A hook stopping the draw leaves the window as it was.
                let _ = self.prepare_layout();
            }
            None => {}
        }
        let window = match bbox {
            None => None,
            Some(BboxInches::Inches(bbox)) => Some(bbox),
            Some(BboxInches::Tight) => {
                let (w_pad, h_pad) = self.tight_padding(options.pad_inches, rc.savefig_pad_inches);
                Some(self.tight_bbox().padded(w_pad, h_pad))
            }
        };
        let (width, height) =
            window.map_or_else(|| self.size_inches(), |b| (b.width(), b.height()));
        let mut state = self.state_mut();
        state.print_window = window;
        state.saving = true;
        if window.is_none() && predrawn {
            state.draws_per_layout = 2;
        }
        drop(state);
        let predrawn = predrawn || bbox == Some(BboxInches::Tight);
        (
            saving,
            Page {
                width,
                height,
                dpi,
                predrawn,
            },
        )
    }

    /// Renders the figure as [`Figure::savefig_with`] would for raster
    /// output.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be created or some artists cannot be drawn.
    pub fn render_raster_with(&self, options: SaveOptions) -> Result<RasterRenderer, SaveError> {
        let (canvas, problems) = self.render_raster_partial(options)?;
        problems.map_or(Ok(canvas), |errors| Err(SaveError::Draw(errors)))
    }

    /// Renders the figure as [`Figure::savefig_with`] would for raster
    /// output, skipping the artists that cannot be drawn: the canvas, and
    /// the problems if any. The print is drawn as the figure's canvas
    /// ([`Figure::canvas`]) is, as matplotlib's raster prints are: what the
    /// callbacks of its [`Event::Draw`](crate::events::Event::Draw) draw on
    /// the canvas, or draw it again with, is printed too, and the canvas
    /// keeps the print.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be created.
    pub fn render_raster_partial(
        &self,
        options: SaveOptions,
    ) -> Result<(RasterRenderer, Option<DrawErrors>), SaveError> {
        let (saving, page) = self.begin_print(&options, None);
        let mut canvas = Self::raster_canvas_sized(page.width, page.height, page.dpi)?;
        let drawn = self.draw_without_event(&mut canvas);
        let problems = DrawErrors::with_refused_memory(drawn, &mut canvas, self.root.id()).err();
        let printed = self.canvas().printing(canvas).map_err(SaveError::Backend);
        drop(saving);
        Ok((printed?, problems))
    }

    /// Renders the figure with a vector backend as [`Figure::savefig_with`]
    /// would: the figure is drawn at 72 dpi into the renderer `make`
    /// creates from the page width and height in inches and the image
    /// resolution (matplotlib's `print_svg`, `print_pdf` and `print_ps`).
    ///
    /// Rasterized artists are drawn at the image resolution into images
    /// placed in the document (matplotlib's `MixedModeRenderer`).
    pub(crate) fn render_vector_with<R: Renderer>(
        &self,
        options: &SaveOptions,
        make: impl FnOnce(f64, f64, f64) -> Result<R, VectorError>,
    ) -> Result<R, SaveError> {
        self.render_vector_noting(options, make, |_| {})
    }

    /// [`render_vector_with`](Self::render_vector_with), handing `drawn`
    /// the renderer once the figure is drawn into it, also when the draw
    /// failed (unless the draw matplotlib makes before the print fails
    /// first): what a renderer notes as it draws, matplotlib's reports
    /// before an artist's failure ends the print.
    pub(crate) fn render_vector_noting<R: Renderer>(
        &self,
        options: &SaveOptions,
        make: impl FnOnce(f64, f64, f64) -> Result<R, VectorError>,
        drawn: impl FnOnce(&R),
    ) -> Result<R, SaveError> {
        // matplotlib's vector backends lay text out unhinted.
        crate::text::without_hinting(|| {
            self.render_into(options, Some(POINTS_PER_INCH), make, drawn)
        })
    }

    /// Renders the figure into the renderer `make` creates, drawing at the
    /// save resolution with text measured as on the raster canvas
    /// (matplotlib's `print_pgf`, whose canvas keeps the figure's
    /// resolution).
    pub(crate) fn render_at_save_resolution<R: Renderer>(
        &self,
        options: &SaveOptions,
        make: impl FnOnce(f64, f64, f64) -> Result<R, VectorError>,
    ) -> Result<R, SaveError> {
        self.render_into(options, None, make, |_| {})
    }

    /// Draws the figure at `canvas_dpi` (default: the save resolution)
    /// into the renderer `make` creates from the page size in inches and
    /// the save resolution, then hands `drawn` the renderer, unless the
    /// draw failed where matplotlib's draw before the print fails first.
    fn render_into<R: Renderer>(
        &self,
        options: &SaveOptions,
        canvas_dpi: Option<f64>,
        make: impl FnOnce(f64, f64, f64) -> Result<R, VectorError>,
        drawn: impl FnOnce(&R),
    ) -> Result<R, SaveError> {
        let (saving, page) = self.begin_print(options, canvas_dpi);
        let mut renderer = make(page.width, page.height, page.dpi).map_err(SaveError::Vector)?;
        let result = self.draw_mixed(&mut renderer, page);
        if result.is_ok() || !page.predrawn {
            drawn(&renderer);
        }
        result?;
        drop(saving);
        Ok(renderer)
    }

    /// Draws the figure as one more page of a PDF document, as
    /// [`Figure::savefig_with`] draws a PDF file (matplotlib's `print_pdf`
    /// into a `PdfPages`).
    pub(crate) fn render_pdf_page(
        &self,
        options: &SaveOptions,
        document: &mut PdfRenderer,
    ) -> Result<(), SaveError> {
        let core_fonts = document.afm_directory();
        crate::text::with_afm_metrics(core_fonts, || {
            crate::text::without_hinting(|| {
                let (saving, page) = self.begin_print(options, Some(POINTS_PER_INCH));
                document
                    .new_page(page.width, page.height, page.dpi)
                    .map_err(SaveError::Vector)?;
                self.draw_mixed(document, page)?;
                drop(saving);
                Ok(())
            })
        })
    }

    /// Draws the prepared figure into the vector `renderer` of `page`,
    /// rasterized artists at the page's resolution.
    fn draw_mixed(&self, renderer: &mut dyn Renderer, page: Page) -> Result<(), SaveError> {
        let mut mixed = MixedModeRenderer::new(
            renderer,
            page.width,
            page.height,
            page.dpi,
            super::save::simplify_settings(),
        );
        let drawn = self.draw(&mut mixed).map_err(SaveError::Draw);
        let rasterized = mixed.finish().map_err(SaveError::Backend);
        drawn.and(rasterized)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;

    use super::*;
    use crate::artist::ArtistHandle;

    #[test]
    fn tight_saves_crop_to_the_drawing() {
        let fig = Figure::with_size(6.0, 4.0, 50.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let full = fig.render_raster_with(SaveOptions::default()).unwrap();
        assert_eq!(full.to_image().width(), 300);
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            ..SaveOptions::default()
        };
        let cropped = fig.render_raster_with(tight).unwrap().to_image();
        assert!(cropped.width() < 300 && cropped.height() < 200);
        // The background reaches the corners and the figure is restored.
        assert_eq!(cropped.pixel(0, 0), Some([255, 255, 255, 255]));
        assert_eq!(fig.size_pixels(), (300.0, 200.0));
    }

    #[test]
    fn tight_saves_keep_the_layout_they_were_cut_from() {
        // matplotlib's test_bboxtight: the layout engine does not run again
        // on the cropped canvas, so the window still frames the axes.
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let layout = crate::layout::ConstrainedLayout::from_rc(&crate::rc::params());
        fig.set_layout_engine(Some(crate::layout::LayoutEngine::Constrained(layout)))
            .unwrap();
        let ax = fig.subplot();
        ax.set_aspect(crate::rc::Aspect::Ratio(1.0)).unwrap();
        ax.set_facecolor("k");
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
        let options = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            pad_inches: Some(PadInches::Inches(0.0)),
            ..SaveOptions::default()
        };
        let image = fig.render_raster_with(options).unwrap().to_image();
        let (w, h) = (image.width(), image.height());
        for (x, y) in [
            (1, 1),
            (w - 2, 1),
            (1, h - 2),
            (w - 2, h - 2),
            (w / 2, h / 2),
        ] {
            let pixel = image.pixel(x, y).unwrap();
            assert!(pixel[0] < 64, "({x}, {y}) is {pixel:?}");
        }
    }

    #[test]
    fn transparency_and_resolution() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        fig.subplot();
        let options = SaveOptions {
            dpi: Some(100.0),
            transparent: Some(true),
            ..SaveOptions::default()
        };
        let image = fig.render_raster_with(options).unwrap().to_image();
        assert_eq!((image.width(), image.height()), (200, 100));
        assert_eq!(
            image.pixel(0, 0).map(|p| p[3]),
            Some(0),
            "transparent corner"
        );
        assert_eq!(fig.dpi(), 50.0);
        assert_ne!(
            fig.axes()[0].facecolor(),
            Color::none(),
            "axes colour restored"
        );
    }

    #[test]
    fn transparency_reaches_insets_and_subfigures() {
        // matplotlib's test_savefig_transparent: nothing of the inset
        // axes or of the coloured subfigure is drawn.
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let grid = crate::gridspec::GridSpec::new(1, 1).unwrap();
        let sub = fig.add_subfigure(&grid.cell(0, 0).unwrap());
        sub.set_facecolor("red");
        let ax = sub
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_axis_off();
        let inset = ax.inset_axes([0.25, 0.25, 0.5, 0.5]);
        let nested = inset.inset_axes([0.25, 0.25, 0.5, 0.5]);
        for inner in [&inset, &nested] {
            inner
                .set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new());
            for spine in inner.spines().iter() {
                spine.set_visible(false);
            }
        }
        let options = SaveOptions {
            transparent: Some(true),
            ..SaveOptions::default()
        };
        let image = fig.render_raster_with(options).unwrap().to_image();
        assert_eq!(image.pixel(50, 50).map(|p| p[3]), Some(0), "centre");
        assert_eq!(image.pixel(5, 5).map(|p| p[3]), Some(0), "subfigure");
        assert_eq!(sub.facecolor(), Color::from("red"), "restored");
        assert_ne!(inset.facecolor(), Color::none(), "inset restored");
    }

    #[test]
    fn save_colours_are_resolved_when_drawn() {
        let fig = Figure::with_size(1.0, 1.0, 10.0);
        let options = SaveOptions {
            facecolor: Some("tab:nope".into()),
            ..SaveOptions::default()
        };
        assert!(matches!(
            fig.render_raster_with(options),
            Err(SaveError::Draw(_))
        ));
        assert_eq!(fig.facecolor(), Color::from(Rgba::WHITE), "restored");
        fig.set_edgecolor("not a colour");
        assert!(fig.render_raster().is_err());
    }

    #[test]
    fn saves_draw_the_animated_artists_of_axes() {
        // matplotlib's test_animated_with_canvas_change: blitting leaves
        // animated artists to the program, saving draws them.
        let fig = Figure::with_size(1.0, 1.0, 50.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_axis_off();
        let props = crate::lines::LineProps::default().linewidth(20.0);
        ax.plot_with(vec![0.0, 1.0], vec![0.5, 0.5], props)
            .unwrap()
            .set_animated(true);
        let centre = |canvas: &RasterRenderer| canvas.to_image().pixel(25, 25);
        let mut canvas = fig.raster_canvas().unwrap();
        fig.draw(&mut canvas).unwrap();
        assert_eq!(centre(&canvas), Some([255, 255, 255, 255]), "left out");
        let saved = fig.render_raster().unwrap();
        assert_ne!(centre(&saved), Some([255, 255, 255, 255]), "drawn");
    }

    #[test]
    fn vector_pages_are_drawn_at_72_dpi() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        fig.subplot();
        let options = SaveOptions {
            bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(0.5, 0.25, 1.0, 0.5))),
            ..SaveOptions::default()
        };
        let mut seen = None;
        fig.render_vector_with(&options, |width, height, dpi| {
            seen = Some((width, height, dpi));
            pltrs_render::SvgRenderer::new(
                width * 72.0,
                height * 72.0,
                dpi,
                pltrs_render::SvgOptions::default(),
            )
        })
        .unwrap();
        assert_eq!(seen, Some((1.0, 0.5, 50.0)));
        assert_eq!(fig.dpi(), 50.0, "resolution restored");
    }
}
