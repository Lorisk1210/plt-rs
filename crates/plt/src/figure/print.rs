//! Saving with options (matplotlib's `savefig` keywords): resolution,
//! cropping to the drawn area (`bbox_inches`), transparency and
//! background colours.

use std::path::Path as FsPath;

use plt_core::{Bbox, Rgba};
use plt_render::RasterRenderer;

use super::frame::FigureFrame;
use super::save::SaveError;
use super::{Figure, FigureState};
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
    /// Resolution (`savefig.dpi`, the figure's own by default).
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

/// `color`, unless the rcParam asks for a specific one.
fn rc_color(setting: &ColorOrAuto, figure: Rgba) -> Rgba {
    match setting {
        ColorOrAuto::Auto => figure,
        ColorOrAuto::Color(color) => color.resolve().unwrap_or(figure),
    }
}

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

    /// Renders the figure as [`Figure::savefig_with`] would.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be created or some artists cannot be drawn.
    pub fn render_raster_with(&self, options: &SaveOptions) -> Result<RasterRenderer, SaveError> {
        let rc = crate::rc::params();
        let dpi = options.dpi.unwrap_or(match rc.savefig_dpi {
            Dpi::Value(dpi) => dpi,
            Dpi::Figure => self.dpi(),
        });
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
        self.set_dpi(dpi).set_facecolor(face).set_edgecolor(edge);
        for (ax, _) in &saving.axes_faces {
            ax.set_facecolor(Color::none());
        }
        let bbox = options
            .bbox_inches
            .or_else(|| (rc.savefig_bbox.as_deref() == Some("tight")).then_some(BboxInches::Tight));
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
        self.state_mut().print_window = window
            .map(|b| Bbox::from_extents(b.x0() * dpi, b.y0() * dpi, b.x1() * dpi, b.y1() * dpi));
        let mut canvas = Self::raster_canvas_sized(width, height, dpi)?;
        self.draw(&mut canvas).map_err(SaveError::Draw)?;
        drop(saving);
        Ok(canvas)
    }

    /// The figure as PNG bytes with save options.
    ///
    /// # Errors
    ///
    /// As [`Figure::render_raster_with`], or when encoding fails.
    pub fn to_png_with(&self, options: &SaveOptions) -> Result<Vec<u8>, SaveError> {
        self.render_raster_with(options)?
            .to_png()
            .map_err(SaveError::Backend)
    }

    /// Writes the figure to `path` as PNG with save options (matplotlib's
    /// `savefig` keywords).
    ///
    /// # Errors
    ///
    /// As [`Figure::to_png_with`], or when the file cannot be written.
    pub fn savefig_with(
        &self,
        path: impl AsRef<FsPath>,
        options: &SaveOptions,
    ) -> Result<(), SaveError> {
        std::fs::write(path, self.to_png_with(options)?).map_err(SaveError::Io)
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
}
