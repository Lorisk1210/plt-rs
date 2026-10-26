//! Saving animations as files: animated PNG, animated GIF (matplotlib's
//! `PillowWriter`), lossless animated WebP and HTML pages, chosen from the
//! file name as `Animation.save` does.

use std::path::Path;

use plt_codec::gif::{GifEncoder, GifError, GifFrame};
use plt_codec::png::{AnimatedEncoder, EncodeError, Frame, ImageRef};
use plt_codec::webp::{self, AnimationFrame};
use plt_render::RgbaImage;

use super::{
    AnimationError, FuncAnimation, HtmlOptions, MovieOptions, interval_millis, render_frame,
};
use crate::figure::SaveError;

/// The loop count of an animation file that plays forever.
const LOOP_FOREVER: u16 = 0;

impl FuncAnimation {
    /// Writes the animation to `path` in the format its extension names
    /// (matplotlib's `Animation.save`): `.png` or `.apng` for an animated
    /// PNG, `.gif` for an animated GIF, `.webp` for a lossless animated
    /// WebP, `.html` or `.htm` for a page with
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
            "webp" => self.save_webp(path),
            "html" | "htm" => self.save_html(path, HtmlOptions::default()),
            "" => Err(AnimationError::UnsupportedFormat(extension)),
            _ => self.save_movie(path, MovieOptions::default()),
        }
    }

    /// Renders every saved frame and encodes them as an animated PNG
    /// showing each frame for one interval and looping forever (as
    /// matplotlib's `PillowWriter` saves them, whether the animation
    /// repeats or not).
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
    /// hundredths of a second, looping forever (Pillow's `loop=0`, whether
    /// the animation repeats or not).
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

    /// Renders every saved frame and encodes them as a lossless animated
    /// WebP showing each frame for one interval and looping forever (as
    /// matplotlib's `PillowWriter` saves them, whether the animation
    /// repeats or not).
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered or the frames cannot be encoded.
    pub fn to_webp(&self) -> Result<Vec<u8>, AnimationError> {
        let images = self.frame_images()?;
        let interval = interval_millis(self.player.borrow().options.interval);
        // Whole milliseconds, as WebP stores them.
        let duration_ms = interval.round().clamp(0.0, f64::from(0xFF_FFFF)) as u32;
        let frames: Vec<AnimationFrame<'_>> = images
            .iter()
            .map(|image| AnimationFrame {
                pixels: image.data(),
                duration_ms,
            })
            .collect();
        let (width, height) = images.first().map_or((0, 0), image_size);
        webp::encode_animation_rgba8(width, height, &frames, LOOP_FOREVER)
            .map_err(AnimationError::Webp)
    }

    /// Writes [`FuncAnimation::to_webp`] to `path`.
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::to_webp`], and when the file cannot be written.
    pub fn save_webp(&self, path: impl AsRef<Path>) -> Result<(), AnimationError> {
        write(path.as_ref(), &self.to_webp()?)
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
    centiseconds as u16
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
            line.set_data([0.0, 1.0], [0.0, i as f64]);
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
    fn webps_hold_every_frame_and_open_on_the_first() {
        let anim = animation(AnimationOptions::default());
        let webp = anim.to_webp().unwrap();
        assert_eq!(&webp[8..16], b"WEBPVP8X");
        let frames = webp.windows(4).filter(|w| *w == b"ANMF").count();
        assert_eq!(frames, 3);
        let first = plt_codec::webp::decode(&webp).unwrap();
        assert_eq!((first.width(), first.height()), (20, 20));
        assert_eq!(
            first.to_rgba8().concat(),
            anim.frame_images().unwrap()[0].data()
        );
    }

    #[test]
    fn files_loop_forever_even_without_repetition() {
        let once = AnimationOptions {
            repeat: false,
            ..AnimationOptions::default()
        };
        let anim = animation(once);
        let gif = anim.to_gif().unwrap();
        let netscape = gif.windows(11).position(|w| w == b"NETSCAPE2.0").unwrap();
        assert_eq!(
            &gif[netscape + 11..netscape + 15],
            [3, 1, 0, 0],
            "loop count 0"
        );
        let apng = anim.to_apng().unwrap();
        let actl = apng.windows(4).position(|w| w == b"acTL").unwrap();
        assert_eq!(&apng[actl + 8..actl + 12], [0, 0, 0, 0], "num_plays 0");
    }

    #[test]
    fn files_are_written_by_extension() {
        let dir = std::env::temp_dir().join(format!("plt-anim-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let anim = animation(AnimationOptions::default());
        anim.save(dir.join("a.gif")).unwrap();
        anim.save(dir.join("a.APNG")).unwrap();
        anim.save(dir.join("a.webp")).unwrap();
        assert!(
            std::fs::read(dir.join("a.webp"))
                .unwrap()
                .starts_with(b"RIFF")
        );
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
