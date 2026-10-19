//! Printing with options (matplotlib's `savefig` keywords): resolution,
//! cropping to the drawn area (`bbox_inches`), transparency and
//! background colours, for the raster canvas and the vector backends.

use plt_core::{Bbox, Rgba};
use plt_render::{Metadata, MixedModeRenderer, PdfRenderer, RasterRenderer, Renderer, VectorError};

use super::frame::FigureFrame;
use super::save::{SaveError, SaveFormat};
use super::{DrawErrors, Figure, FigureState};
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::{Axes, AxesState, clipped_extent};
use crate::colors::Color;
use crate::rc::{ColorOrAuto, Dpi, SavefigBbox};

/// Which part of the figure is saved (matplotlib's `bbox_inches`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BboxInches {
    /// The area everything is drawn in, padded (`"tight"`).
    Tight,
    /// This box, in inches from the figure's bottom-left corner.
    Inches(Bbox),
}

/// Options of [`Figure::savefig_with`]; unset ones follow the `savefig.*`
/// rcParams.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveOptions {
    /// Resolution (`savefig.dpi`, the figure's own by default). Vector
    /// formats are drawn in points and rasterize images at this resolution.
    pub dpi: Option<f64>,
    /// The part of the figure saved (`savefig.bbox`, the whole figure by
    /// default).
    pub bbox_inches: Option<BboxInches>,
    /// Padding around a tight box in inches (`savefig.pad_inches`).
    pub pad_inches: Option<f64>,
    /// Whether the figure and axes backgrounds are left out
    /// (`savefig.transparent`).
    pub transparent: Option<bool>,
    /// Figure background (`savefig.facecolor`, the figure's own by
    /// default).
    pub facecolor: Option<Rgba>,
    /// Figure border colour (`savefig.edgecolor`).
    pub edgecolor: Option<Rgba>,
    /// The file format, overriding the file name's extension
    /// (matplotlib's `format`).
    pub format: Option<SaveFormat>,
    /// Document metadata of the vector formats (plt-rs as creator and
    /// the current or `SOURCE_DATE_EPOCH` date by default).
    pub metadata: Option<Metadata>,
    /// JPEG quality from 1 to 100 (Pillow's `quality` in matplotlib's
    /// `pil_kwargs`; 75 by default).
    pub jpeg_quality: Option<u8>,
}

impl FigureState {
    /// The area everything visible is drawn in, in display pixels
    /// (matplotlib's `Figure.get_tightbbox`): the figure-level artists
    /// that take part in layout and the tight boxes of the visible axes;
    /// `None` when nothing is drawn.
    pub(crate) fn figure_tight_bbox(&mut self) -> Option<Bbox> {
        self.prepare();
        let mut boxes = Vec::new();
        let mut frames = vec![self.root];
        let mut axes = Vec::new();
        while let Some(frame) = frames.pop() {
            let Some(node) = self.store.get_as::<FigureFrame>(frame) else {
                continue;
            };
            for &child in &node.children {
                if self.store.get_as::<FigureFrame>(child).is_some() {
                    frames.push(child);
                } else if self.store.get_as::<AxesState>(child).is_some() {
                    axes.push(child);
                } else if crate::mplot3d::is_axes3d(&self.store, child) {
                    boxes.extend(crate::mplot3d::axes3d_tight_bbox(self, child));
                } else {
                    boxes.extend(self.artist_extent(child));
                }
            }
        }
        boxes.extend(
            axes.into_iter()
                .filter_map(|id| self.axes_tight_bbox(id, false)),
        );
        boxes
            .into_iter()
            .filter(|b| {
                b.width().is_finite()
                    && b.height().is_finite()
                    && (b.width() != 0.0 || b.height() != 0.0)
            })
            .reduce(|a, b| a.union(&b))
    }

    /// The extent of a visible figure-level artist that takes part in
    /// layout, reduced by its clipping (matplotlib's
    /// `Artist.get_tightbbox`).
    fn artist_extent(&self, id: ArtistId) -> Option<Bbox> {
        let artist = self.store.get(id)?;
        let base = artist.base();
        if !(base.visible && base.in_layout) {
            return None;
        }
        let layout = LayoutContext::new(self);
        clipped_extent(layout, base, artist.window_extent(&layout)?)
    }
}

/// What a save changes on the figure, put back when dropped (also when
/// rendering fails).
struct Saving<'a> {
    figure: &'a Figure,
    dpi: f64,
    colors: (Rgba, Rgba),
    /// The background and border colours of the subfigures made
    /// transparent.
    subfigure_colors: Vec<(Figure, Rgba, Rgba)>,
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
        }
        figure.set_dpi(self.dpi);
        figure.set_facecolor(self.colors.0);
        figure.set_edgecolor(self.colors.1);
        for (sub, face, edge) in &self.subfigure_colors {
            sub.set_facecolor(*face).set_edgecolor(*edge);
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
}

/// `color`, unless the rcParam asks for a specific one.
fn rc_color(setting: &ColorOrAuto, figure: Rgba) -> Rgba {
    match setting {
        ColorOrAuto::Auto => figure,
        ColorOrAuto::Color(color) => color.resolve().unwrap_or(figure),
    }
}

/// The resolution vector backends draw at: one unit per point.
const POINTS_PER_INCH: f64 = 72.0;

impl Figure {
    /// The area everything visible is drawn in, in inches from the
    /// figure's bottom-left corner (matplotlib's `get_tightbbox`); the
    /// whole figure when nothing is drawn.
    pub fn tight_bbox(&self) -> Bbox {
        let mut state = self.state_mut();
        let dpi = state.dpi();
        state.figure_tight_bbox().map_or_else(
            || {
                let (w, h) = state.size_inches;
                Bbox::from_bounds(0.0, 0.0, w, h)
            },
            |b| Bbox::from_extents(b.x0() / dpi, b.y0() / dpi, b.x1() / dpi, b.y1() / dpi),
        )
    }

    /// Prepares the figure for printing with `options` as matplotlib's
    /// `print_figure` does: resolution, background colours and the part
    /// printed. The figure is drawn at `canvas_dpi` (the save resolution
    /// for raster output, 72 for the vector formats, which also measure
    /// the tight box at 72 dpi).
    fn begin_print(&self, options: &SaveOptions, canvas_dpi: Option<f64>) -> (Saving<'_>, Page) {
        let rc = crate::rc::params();
        let dpi = options.dpi.unwrap_or(match rc.savefig_dpi {
            Dpi::Value(dpi) => dpi,
            Dpi::Figure => self.dpi(),
        });
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
            (Rgba::TRANSPARENT, Rgba::TRANSPARENT)
        } else {
            (
                options
                    .facecolor
                    .unwrap_or_else(|| rc_color(&rc.savefig_facecolor, saving.colors.0)),
                options
                    .edgecolor
                    .unwrap_or_else(|| rc_color(&rc.savefig_edgecolor, saving.colors.1)),
            )
        };
        self.set_dpi(canvas_dpi)
            .set_facecolor(face)
            .set_edgecolor(edge);
        let bbox = options
            .bbox_inches
            .or_else(|| (rc.savefig_bbox == SavefigBbox::Tight).then_some(BboxInches::Tight));
        match bbox {
            // matplotlib draws the figure once without rendering to measure
            // it.
            Some(BboxInches::Tight) => self.layout(),
            // The layout the window is cut from, at the print resolution.
            Some(BboxInches::Inches(_)) => self.prepare_layout(),
            None => {}
        }
        let window = match bbox {
            None => None,
            Some(BboxInches::Inches(bbox)) => Some(bbox),
            Some(BboxInches::Tight) => {
                let pad = options.pad_inches.unwrap_or(rc.savefig_pad_inches);
                Some(self.tight_bbox().padded(pad, pad))
            }
        };
        let (width, height) =
            window.map_or_else(|| self.size_inches(), |b| (b.width(), b.height()));
        let mut state = self.state_mut();
        state.print_window = window;
        state.saving = true;
        drop(state);
        (saving, Page { width, height, dpi })
    }

    /// Renders the figure as [`Figure::savefig_with`] would for raster
    /// output.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be created or some artists cannot be drawn.
    pub fn render_raster_with(&self, options: &SaveOptions) -> Result<RasterRenderer, SaveError> {
        let (canvas, problems) = self.render_raster_partial(options)?;
        problems.map_or(Ok(canvas), |errors| Err(SaveError::Draw(errors)))
    }

    /// Renders the figure as [`Figure::savefig_with`] would for raster
    /// output, skipping the artists that cannot be drawn: the canvas, and
    /// the problems if any.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be created.
    pub fn render_raster_partial(
        &self,
        options: &SaveOptions,
    ) -> Result<(RasterRenderer, Option<DrawErrors>), SaveError> {
        let (saving, page) = self.begin_print(options, None);
        let mut canvas = Self::raster_canvas_sized(page.width, page.height, page.dpi)?;
        let problems = self.draw(&mut canvas).err();
        drop(saving);
        Ok((canvas, problems))
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
        // matplotlib's vector backends lay text out unhinted.
        crate::text::without_hinting(|| {
            let (saving, page) = self.begin_print(options, Some(POINTS_PER_INCH));
            let mut renderer =
                make(page.width, page.height, page.dpi).map_err(SaveError::Vector)?;
            self.draw_mixed(&mut renderer, page)?;
            drop(saving);
            Ok(renderer)
        })
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
    use super::*;
    use crate::artist::{ArtistHandle, ClipPathSpec};

    #[test]
    fn tight_saves_crop_to_the_drawing() {
        let fig = Figure::with_size(6.0, 4.0, 50.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let full = fig.render_raster_with(&SaveOptions::default()).unwrap();
        assert_eq!(full.to_image().width(), 300);
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            ..SaveOptions::default()
        };
        let cropped = fig.render_raster_with(&tight).unwrap().to_image();
        assert!(cropped.width() < 300 && cropped.height() < 200);
        // The background reaches the corners and the figure is restored.
        assert_eq!(cropped.pixel(0, 0), Some([255, 255, 255, 255]));
        assert_eq!(fig.size_pixels(), (300.0, 200.0));
    }

    #[test]
    fn tight_boxes_take_the_clipping_of_figure_artists() {
        // matplotlib's test_bbox_inches_tight_clipping: a huge patch
        // clipped to a small path adds only the path's extent.
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let patch = fig.add_patch(crate::patches::Rectangle::new((-50.0, -50.0), 100.0, 100.0));
        patch.set_clip_path(Some(ClipPathSpec::Path {
            path: plt_core::Path::unit_rectangle(),
            transform: crate::transforms::Transform::Inches,
        }));
        assert_eq!(fig.tight_bbox(), Bbox::from_extents(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn tight_saves_keep_the_layout_they_were_cut_from() {
        // matplotlib's test_bboxtight: the layout engine does not run again
        // on the cropped canvas, so the window still frames the axes.
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let layout = crate::layout::ConstrainedLayout::from_rc(&crate::rc::params());
        fig.set_layout_engine(Some(crate::layout::LayoutEngine::Constrained(layout)));
        let ax = fig.subplot();
        ax.set_aspect(crate::rc::Aspect::Ratio(1.0)).unwrap();
        ax.set_facecolor("k");
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
        let options = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            pad_inches: Some(0.0),
            ..SaveOptions::default()
        };
        let image = fig.render_raster_with(&options).unwrap().to_image();
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
        let image = fig.render_raster_with(&options).unwrap().to_image();
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
        sub.set_facecolor(Rgba::new(1.0, 0.0, 0.0, 1.0));
        let ax = sub.add_axes([0.0, 0.0, 1.0, 1.0]);
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
        let image = fig.render_raster_with(&options).unwrap().to_image();
        assert_eq!(image.pixel(50, 50).map(|p| p[3]), Some(0), "centre");
        assert_eq!(image.pixel(5, 5).map(|p| p[3]), Some(0), "subfigure");
        assert_eq!(sub.facecolor(), Rgba::new(1.0, 0.0, 0.0, 1.0), "restored");
        assert_ne!(inset.facecolor(), Color::none(), "inset restored");
    }

    #[test]
    fn saves_draw_the_animated_artists_of_axes() {
        // matplotlib's test_animated_with_canvas_change: blitting leaves
        // animated artists to the program, saving draws them.
        let fig = Figure::with_size(1.0, 1.0, 50.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
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
            plt_render::SvgRenderer::new(
                width * 72.0,
                height * 72.0,
                dpi,
                plt_render::SvgOptions::default(),
            )
        })
        .unwrap();
        assert_eq!(seen, Some((1.0, 0.5, 50.0)));
        assert_eq!(fig.dpi(), 50.0, "resolution restored");
    }
}
