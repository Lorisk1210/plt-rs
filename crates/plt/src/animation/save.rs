//! Saving animations as files: animated PNG, animated GIF (matplotlib's
//! `PillowWriter`) and HTML pages, chosen from the file name as
//! `Animation.save` does.

use std::path::Path;

use plt_codec::gif::{GifEncoder, GifError, GifFrame};
use plt_codec::png::{AnimatedEncoder, EncodeError, Frame, ImageRef};
use plt_render::RgbaImage;

use super::{
    AnimationError, FuncAnimation, HtmlOptions, MovieOptions, interval_millis, render_frame,
};
use crate::figure::SaveError;

impl FuncAnimation {
    /// Writes the animation to `path` in the format its extension names
    /// (matplotlib's `Animation.save`): `.png` or `.apng` for an animated
    /// PNG, `.gif` for an animated GIF, `.html` or `.htm` for a page with
    /// the JavaScript player, and any other extension for a video encoded
    /// by ffmpeg ([`FuncAnimation::save_movie`], matplotlib's default
    /// `animation.writer`).
    ///
    /// # Errors
    ///
    /// When `path` has no extension, a frame cannot be rendered or
    /// encoded, or the file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), AnimationError> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "png" | "apng" => self.save_apng(path),
            "gif" => self.save_gif(path),
            "html" | "htm" => self.save_html(path, &HtmlOptions::default()),
            "" => Err(AnimationError::UnsupportedFormat(extension)),
            _ => self.save_movie(path, &MovieOptions::default()),
        }
    }

    /// Renders every saved frame and encodes them as an animated PNG
    /// showing each frame for one interval, looping if the animation
    /// repeats.
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered or the frames cannot be encoded.
    pub fn to_apng(&self) -> Result<Vec<u8>, AnimationError> {
        let images = self.frame_images()?;
        let millis =
            u16::try_from(self.player.borrow().options.interval.as_millis()).unwrap_or(u16::MAX);
        let frames = images
            .iter()
            .map(|image| {
                let (width, height) = image_size(image);
                Ok(Frame {
                    image: ImageRef::rgba8(width, height, image.data())?,
                    delay: (millis, 1000),
                })
            })
            .collect::<Result<Vec<_>, EncodeError>>()
            .map_err(AnimationError::Encode)?;
        AnimatedEncoder::new()
            .plays(u32::from(self.plays_once()))
            .encode(&frames)
            .map_err(AnimationError::Encode)
    }

    /// Writes [`FuncAnimation::to_apng`] to `path`.
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::to_apng`], or when the file cannot be written.
    pub fn save_apng(&self, path: impl AsRef<Path>) -> Result<(), AnimationError> {
        write(path.as_ref(), &self.to_apng()?)
    }

    /// Renders every saved frame and encodes them as an animated GIF
    /// (matplotlib's `PillowWriter`): one palette of at most 256 colours,
    /// each frame shown for `int(1000 / fps)` milliseconds rounded down to
    /// hundredths of a second, looping if the animation repeats.
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered, or the figure is empty or wider or
    /// taller than GIF's 65 535 pixels.
    pub fn to_gif(&self) -> Result<Vec<u8>, AnimationError> {
        let images = self.frame_images()?;
        let delay = gif_delay(interval_millis(self.player.borrow().options.interval));
        let frames: Vec<GifFrame<'_>> = images
            .iter()
            .map(|image| GifFrame {
                pixels: image.data(),
                delay,
            })
            .collect();
        let Some(first) = images.first() else {
            return Err(AnimationError::Gif(GifError::NoFrames));
        };
        let (width, height) = image_size(first);
        GifEncoder::new()
            .plays(u16::from(self.plays_once()))
            .encode(width, height, &frames)
            .map_err(AnimationError::Gif)
    }

    /// Writes [`FuncAnimation::to_gif`] to `path`.
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::to_gif`], or when the file cannot be written.
    pub fn save_gif(&self, path: impl AsRef<Path>) -> Result<(), AnimationError> {
        write(path.as_ref(), &self.to_gif()?)
    }

    /// Every saved frame rendered as `savefig` would.
    fn frame_images(&self) -> Result<Vec<RgbaImage>, AnimationError> {
        let mut images = Vec::new();
        self.for_each_frame(|figure| {
            images.push(render_frame(figure)?);
            Ok(())
        })?;
        Ok(images)
    }

    /// Whether the saved animation plays once rather than looping.
    fn plays_once(&self) -> bool {
        !self.player.borrow().options.repeat
    }
}

/// The size of `image` in pixels, saturated to `u32` (the encoders reject
/// sizes that large).
fn image_size(image: &RgbaImage) -> (u32, u32) {
    let side = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    (side(image.width()), side(image.height()))
}

/// The centiseconds a GIF frame shows for an animation interval: Pillow's
/// `int(duration / 10)` of matplotlib's `duration = int(1000 / fps)`, with
/// `fps = 1000 / interval`.
fn gif_delay(interval_millis: f64) -> u16 {
    let duration = (1000.0 / (1000.0 / interval_millis)).trunc();
    let centiseconds = (duration / 10.0).trunc().clamp(0.0, f64::from(u16::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to an integer in u16's range"
    )]
    let delay = centiseconds as u16;
    delay
}

/// Writes `data` to `path`.
fn write(path: &Path, data: &[u8]) -> Result<(), AnimationError> {
    std::fs::write(path, data).map_err(|err| AnimationError::Save(SaveError::Io(err)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationOptions, Frames};
    use crate::figure::Figure;

    fn animation(options: AnimationOptions) -> FuncAnimation {
        let fig = Figure::with_size(1.0, 1.0, 20.0);
        let ax = fig.subplot();
        let line = ax.plot(vec![0.0, 1.0], vec![0.0, 0.0]).unwrap();
        FuncAnimation::new(&fig, Frames::Count(3), options, move |i| {
            line.set_data(&[0.0, 1.0], &[0.0, i as f64]);
        })
    }

    #[test]
    fn gif_delays_follow_pillow() {
        assert_eq!(gif_delay(200.0), 20);
        assert_eq!(gif_delay(33.0), 3);
        assert_eq!(gif_delay(5.0), 0);
        assert_eq!(gif_delay(1e9), u16::MAX);
    }

    #[test]
    fn gifs_hold_every_frame() {
        let gif = animation(AnimationOptions::default()).to_gif().unwrap();
        assert!(gif.starts_with(b"GIF89a"));
        assert_eq!(&gif[6..10], [20, 0, 20, 0], "20 × 20 pixels");
        assert!(gif.windows(11).any(|w| w == b"NETSCAPE2.0"));
        let frames = gif.windows(3).filter(|w| *w == [0x21, 0xF9, 4]).count();
        assert_eq!(frames, 3);
    }

    #[test]
    fn single_runs_do_not_loop() {
        let once = AnimationOptions {
            repeat: false,
            ..AnimationOptions::default()
        };
        let gif = animation(once).to_gif().unwrap();
        assert!(!gif.windows(11).any(|w| w == b"NETSCAPE2.0"));
    }

    #[test]
    fn files_are_written_by_extension() {
        let dir = std::env::temp_dir().join(format!("plt-anim-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let anim = animation(AnimationOptions::default());
        anim.save(dir.join("a.gif")).unwrap();
        anim.save(dir.join("a.APNG")).unwrap();
        assert!(
            std::fs::read(dir.join("a.gif"))
                .unwrap()
                .starts_with(b"GIF89a")
        );
        assert!(
            std::fs::read(dir.join("a.APNG"))
                .unwrap()
                .starts_with(b"\x89PNG")
        );
        assert!(matches!(
            anim.save(dir.join("movie")),
            Err(AnimationError::UnsupportedFormat(ext)) if ext.is_empty()
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
