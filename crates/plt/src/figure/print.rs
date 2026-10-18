//! Printing with options (matplotlib's `savefig` keywords): resolution,
//! cropping to the drawn area (`bbox_inches`), transparency and
//! background colours, for the raster canvas and the vector backends.

use plt_core::{Bbox, Rgba};
use plt_render::{Metadata, MixedModeRenderer, PdfRenderer, RasterRenderer, Renderer, VectorError};

use super::frame::FigureFrame;
use super::save::{SaveError, SaveFormat};
use super::{DrawErrors, Figure, FigureState};
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::{Axes, AxesState};
use crate::colors::Color;
use crate::rc::{ColorOrAuto, Dpi};

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
    /// layout.
    fn artist_extent(&self, id: ArtistId) -> Option<Bbox> {
        let artist = self.store.get(id)?;
        let base = artist.base();
        (base.visible && base.in_layout)
            .then(|| artist.window_extent(&LayoutContext::new(self)))
            .flatten()
    }
}

/// What a save changes on the figure, put back when dropped (also when
/// rendering fails).
struct Saving<'a> {
    figure: &'a Figure,
    dpi: f64,
    colors: (Rgba, Rgba),
    axes_faces: Vec<(Axes, Color)>,
}

impl Drop for Saving<'_> {
    fn drop(&mut self) {
        let figure = self.figure;
        figure.state_mut().print_window = None;
        figure.set_dpi(self.dpi);
        figure.set_facecolor(self.colors.0);
        figure.set_edgecolor(self.colors.1);
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
        let dpi = state.dpi;
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
        let saving = Saving {
            figure: self,
            dpi: self.dpi(),
            colors: (self.facecolor(), self.edgecolor()),
            axes_faces: if transparent {
                self.axes()
                    .into_iter()
                    .map(|ax| {
                        let face = ax.facecolor();
                        (ax, face)
                    })
                    .collect()
            } else {
                Vec::new()
            },
        };
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
        for (ax, _) in &saving.axes_faces {
            ax.set_facecolor(Color::none());
        }
        let bbox = options
            .bbox_inches
            .or_else(|| (rc.savefig_bbox == "tight").then_some(BboxInches::Tight));
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
        self.state_mut().print_window = window.map(|b| {
            Bbox::from_extents(
                b.x0() * canvas_dpi,
                b.y0() * canvas_dpi,
                b.x1() * canvas_dpi,
                b.y1() * canvas_dpi,
            )
        });
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
        crate::text::without_hinting(|| {
            let (saving, page) = self.begin_print(options, Some(POINTS_PER_INCH));
            document
                .new_page(page.width, page.height, page.dpi)
                .map_err(SaveError::Vector)?;
            self.draw_mixed(document, page)?;
            drop(saving);
            Ok(())
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
