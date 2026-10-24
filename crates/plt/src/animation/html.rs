//! Animations as web pages with matplotlib's JavaScript player
//! (`Animation.to_jshtml` and the `HTMLWriter`): a slider, playback
//! buttons and the repetition mode, with the frames embedded as base64
//! data or written next to the page.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use plt_codec::base64;
use plt_core::numeric::floor_div;

use super::html_template::{DISPLAY_TEMPLATE, INCLUDED_FRAMES, JS_INCLUDE, STYLE_INCLUDE};
use super::{AnimationError, FuncAnimation, interval_millis};
use crate::figure::{SaveError, SaveFormat, SaveOptions};
use crate::ticker::pyfmt;

/// What the player does after the last frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayMode {
    /// Stop.
    Once,
    /// Start over.
    Loop,
    /// Play backwards, then forwards again.
    Reflect,
}

/// Settings of the HTML player (the arguments of matplotlib's
/// `to_jshtml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HtmlOptions {
    /// Frames per second; by default one frame per animation interval.
    pub fps: Option<NonZeroU32>,
    /// The repetition mode selected when the page opens; by default
    /// [`PlayMode::Loop`] for repeating animations, otherwise
    /// [`PlayMode::Once`].
    pub default_mode: Option<PlayMode>,
}

impl FuncAnimation {
    /// The animation as an HTML fragment playing the frames with
    /// matplotlib's JavaScript player, every frame embedded as a base64
    /// image (matplotlib's `to_jshtml`).
    ///
    /// Frames are PNG images, or SVG documents when
    /// `animation.frame_format` is `"svg"`. Once the encoded frames reach
    /// `animation.embed_limit` megabytes, that frame and the ones after it
    /// are left out, as in matplotlib.
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered.
    pub fn to_jshtml(&self, options: HtmlOptions) -> Result<String, AnimationError> {
        let rc = crate::rc::params();
        let format = frame_format(&rc.animation_frame_format);
        let limit = rc.animation_embed_limit * 1024.0 * 1024.0;
        let mut frames = Vec::new();
        let mut total = 0.0;
        let mut full = false;
        self.for_each_frame(|figure| {
            if full {
                return Ok(());
            }
            let data = base64::encode_lines(&figure.to_format(format, SaveOptions::default())?);
            total += byte_count(&data);
            full = total >= limit;
            if !full {
                frames.push(data);
            }
            Ok(())
        })?;
        Ok(self.page(&embedded_frames(&frames, format), frames.len(), options))
    }

    /// The animation's representation in a notebook, as
    /// `animation.html` selects it (matplotlib's `_repr_html_`): an HTML5
    /// video (`"html5"`), the JavaScript player (`"jshtml"`), or nothing
    /// (`"none"`, the default).
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::to_html5_video`] or [`FuncAnimation::to_jshtml`].
    pub fn repr_html(&self) -> Result<Option<String>, AnimationError> {
        match crate::rc::params().animation_html.as_str() {
            "html5" => self.to_html5_video().map(Some),
            "jshtml" => self.to_jshtml(HtmlOptions::default()).map(Some),
            _ => Ok(None),
        }
    }

    /// Writes the animation as an HTML page at `path` (`.html` or
    /// `.htm`) with its frames in the directory `<stem>_frames` beside it,
    /// named `frame0000000.png` and so on (matplotlib's `HTMLWriter`, the
    /// writer of `save("….html")`).
    ///
    /// # Errors
    ///
    /// When `path` is not an HTML file name, a frame cannot be rendered or
    /// a file cannot be written.
    pub fn save_html(
        &self,
        path: impl AsRef<Path>,
        options: HtmlOptions,
    ) -> Result<(), AnimationError> {
        let path = path.as_ref();
        let (Some(stem), Some("html" | "htm")) = (
            path.file_stem().and_then(|stem| stem.to_str()),
            path.extension().and_then(|ext| ext.to_str()),
        ) else {
            return Err(AnimationError::NotHtml(path.to_path_buf()));
        };
        let frame_dir = format!("{stem}_frames");
        let directory = path.with_file_name(&frame_dir);
        std::fs::create_dir_all(&directory).map_err(io_error)?;
        let format = frame_format(&crate::rc::params().animation_frame_format);
        let mut count = 0;
        self.for_each_frame(|figure| {
            let name = format!("frame{count:07}.{}", format.extension());
            let frame = figure.to_format(format, SaveOptions::default())?;
            std::fs::write(directory.join(name), frame).map_err(SaveError::Io)?;
            count += 1;
            Ok(())
        })?;
        let fill_frames = fill(
            INCLUDED_FRAMES,
            &[
                ("Nframes", &count.to_string()),
                ("frame_dir", &frame_dir),
                ("frame_format", format.extension()),
            ],
        );
        std::fs::write(path, self.page(&fill_frames, count, options)).map_err(io_error)
    }

    /// The page showing `count` frames that `fill_frames` loads.
    fn page(&self, fill_frames: &str, count: usize, options: HtmlOptions) -> String {
        let (interval, repeat) = {
            let player = self.player.borrow();
            (player.options.interval, player.options.repeat)
        };
        let mode = options.default_mode.unwrap_or(if repeat {
            PlayMode::Loop
        } else {
            PlayMode::Once
        });
        let checked = |shown: PlayMode| if shown == mode { "checked" } else { "" };
        let display = fill(
            DISPLAY_TEMPLATE,
            &[
                ("id", &player_id(fill_frames)),
                ("Nframes", &count.to_string()),
                ("fill_frames", fill_frames),
                ("interval", &interval_text(options.fps, interval)),
                ("once_checked", checked(PlayMode::Once)),
                ("loop_checked", checked(PlayMode::Loop)),
                ("reflect_checked", checked(PlayMode::Reflect)),
            ],
        );
        [JS_INCLUDE, STYLE_INCLUDE, &display].concat()
    }
}

/// The frame format named by `animation.frame_format`: SVG or PNG (the
/// player's other formats, JPEG and TIFF, fall back to PNG).
fn frame_format(name: &str) -> SaveFormat {
    if name == "svg" {
        SaveFormat::Svg
    } else {
        SaveFormat::Png
    }
}

/// An error writing the page or its frame directory.
fn io_error(err: std::io::Error) -> AnimationError {
    AnimationError::Save(SaveError::Io(err))
}

/// The length of `data` in bytes, as a number to compare with the limit.
fn byte_count(data: &str) -> f64 {
    data.len() as f64
}

/// The milliseconds between frames the player is given: `1000 // fps`,
/// a float when the rate comes from the animation's interval (matplotlib
/// derives it as `1000 / interval`).
fn interval_text(fps: Option<NonZeroU32>, interval: Duration) -> String {
    if let Some(fps) = fps {
        return (1000 / fps.get()).to_string();
    }
    pyfmt::repr(floor_div(1000.0, 1000.0 / interval_millis(interval)))
}

/// The script lines setting each embedded frame (matplotlib's
/// `_embedded_frames`), its base64 lines continued with backslashes.
fn embedded_frames(frames: &[String], format: SaveFormat) -> String {
    let mime = match format {
        SaveFormat::Svg => "svg+xml",
        _ => format.extension(),
    };
    let lines = frames.iter().enumerate().map(|(index, data)| {
        let data = data.replace('\n', "\\\n");
        format!("  frames[{index}] = \"data:image/{mime};base64,{data}\"\n")
    });
    std::iter::once(String::from("\n")).chain(lines).collect()
}

/// A fresh id for the player's elements (matplotlib's `uuid4().hex`): 32
/// hex digits, different for every page this process makes.
fn player_id(content: &str) -> String {
    static PAGES: AtomicU64 = AtomicU64::new(0);
    let page = PAGES.fetch_add(1, Ordering::Relaxed);
    let half = |salt: u8| {
        let mut hasher = DefaultHasher::new();
        (salt, page, content).hash(&mut hasher);
        hasher.finish()
    };
    format!("{:016x}{:016x}", half(0), half(1))
}

/// `template` with each `{name}` placeholder listed in `values` replaced
/// by its value in one pass; other braces (the player's script) stay.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after.find('}').and_then(|close| {
            let name = &after[..close];
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value, close))
        });
        if let Some((value, close)) = value {
            out.push_str(value);
            rest = &after[close + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationOptions, Frames};
    use crate::figure::Figure;

    fn animation(count: usize, options: AnimationOptions) -> FuncAnimation {
        let fig = Figure::with_size(1.0, 1.0, 20.0);
        let ax = fig.subplot();
        let line = ax.plot(vec![0.0, 1.0], vec![0.0, 0.0]).unwrap();
        FuncAnimation::new(&fig, Frames::Count(count), options, move |i| {
            line.set_data(&[0.0, 1.0], &[0.0, i as f64]);
        })
    }

    #[test]
    fn placeholders_are_filled_once() {
        let filled = fill("f() { a({id}, {n}) } {other}", &[("id", "{n}"), ("n", "3")]);
        assert_eq!(filled, "f() { a({n}, 3) } {other}");
        assert_eq!(fill("{", &[]), "{");
    }

    #[test]
    fn intervals_print_like_python() {
        let ms = Duration::from_millis;
        assert_eq!(interval_text(None, ms(200)), "200.0");
        assert_eq!(interval_text(None, ms(30)), "29.0", "1000 // (1000 / 30)");
        assert_eq!(interval_text(None, Duration::ZERO), "0.0");
        assert_eq!(interval_text(NonZeroU32::new(30), ms(200)), "33");
    }

    /// The repetition modes a page starts with selected.
    fn selected_modes(html: &str) -> Vec<&'static str> {
        ["once", "loop", "reflect"]
            .into_iter()
            .filter(|mode| {
                let input = &html[html.find(&format!("value=\"{mode}\"")).unwrap()..];
                input[..input.find('>').unwrap()].contains("checked")
            })
            .collect()
    }

    #[test]
    fn frames_are_embedded_as_png_data() {
        let html = animation(3, AnimationOptions::default())
            .to_jshtml(HtmlOptions::default())
            .unwrap();
        assert!(html.starts_with(JS_INCLUDE));
        assert!(html.contains("var frames = new Array(3);"));
        assert!(html.contains("  frames[2] = \"data:image/png;base64,iVBORw0KGgo"));
        assert!(html.contains("img_id, slider_id, 200.0,"));
        assert_eq!(selected_modes(&html), ["loop"]);
    }

    #[test]
    fn modes_follow_repetition() {
        let once = AnimationOptions {
            repeat: false,
            ..AnimationOptions::default()
        };
        let html = animation(1, once)
            .to_jshtml(HtmlOptions::default())
            .unwrap();
        assert_eq!(selected_modes(&html), ["once"]);
        let reflect = HtmlOptions {
            default_mode: Some(PlayMode::Reflect),
            fps: NonZeroU32::new(24),
        };
        let html = animation(1, once).to_jshtml(reflect).unwrap();
        assert_eq!(selected_modes(&html), ["reflect"]);
        assert!(html.contains("img_id, slider_id, 41,"));
    }

    #[test]
    fn frames_past_the_limit_are_dropped() {
        let html = crate::rc::context(|| {
            crate::rc::set("animation.embed_limit", "0.000001").unwrap();
            animation(2, AnimationOptions::default())
                .to_jshtml(HtmlOptions::default())
                .unwrap()
        });
        assert!(html.contains("new Array(0);"));
        assert!(!html.contains("base64,"));
    }

    #[test]
    fn notebooks_get_what_animation_html_asks_for() {
        let anim = animation(1, AnimationOptions::default());
        assert_eq!(anim.repr_html().unwrap(), None);
        let page = crate::rc::context(|| {
            crate::rc::set("animation.html", "jshtml").unwrap();
            anim.repr_html().unwrap()
        });
        assert!(page.is_some_and(|page| page.contains("new Array(1);")));
    }

    #[test]
    fn pages_get_distinct_ids() {
        assert_ne!(player_id("same"), player_id("same"));
        assert_eq!(player_id("").len(), 32);
    }

    #[test]
    fn saving_writes_frames_beside_the_page() {
        let dir = std::env::temp_dir().join(format!("plt-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let anim = animation(2, AnimationOptions::default());
        let page = dir.join("anim.html");
        anim.save_html(&page, HtmlOptions::default()).unwrap();
        let html = std::fs::read_to_string(&page).unwrap();
        assert!(html.contains("frames[i] = \"anim_frames/frame\" + (\"0000000\" + i).slice(-7) +"));
        assert!(html.contains("\".png\";"));
        let frame = std::fs::read(dir.join("anim_frames/frame0000001.png")).unwrap();
        assert!(frame.starts_with(b"\x89PNG"));
        assert!(matches!(
            anim.save_html(dir.join("anim.txt"), HtmlOptions::default()),
            Err(AnimationError::NotHtml(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
