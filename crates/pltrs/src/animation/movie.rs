//! Videos encoded by ffmpeg (matplotlib's `FFMpegWriter` and
//! `to_html5_video`): every frame is rendered as raw RGBA and piped to the
//! `animation.ffmpeg_path` program, which encodes it with `animation.codec`
//! at `animation.bitrate`.

use std::fmt;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use pltrs_codec::base64;

use super::{AnimationError, FuncAnimation, interval_millis, render_frame};
use crate::figure::{Figure, SaveError};
use crate::rc::Dpi;
use crate::ticker::pyfmt;

/// Settings of an ffmpeg encoding (the arguments of matplotlib's
/// `FFMpegWriter`); unset ones come from the rcParams.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MovieOptions {
    /// Frames per second; one frame per animation interval by default.
    pub fps: Option<f64>,
    /// The video codec (`animation.codec`); containers that imply their
    /// codec (`.apng`, `.avif`, `.gif`, `.webm`, `.webp`) ignore it.
    pub codec: Option<String>,
    /// The bitrate in kilobits per second (`animation.bitrate`; zero or
    /// less lets ffmpeg choose).
    pub bitrate: Option<i64>,
    /// Arguments passed to ffmpeg before the output file
    /// (`animation.ffmpeg_args`).
    pub extra_args: Option<Vec<String>>,
    /// Metadata of the file (`-metadata key=value`).
    pub metadata: Vec<(String, String)>,
}

/// Why ffmpeg could not make a video.
#[derive(Debug)]
pub enum MovieError {
    /// The program could not be started (most often: it is not installed;
    /// set `animation.ffmpeg_path`).
    Spawn {
        /// The program run.
        program: String,
        /// Why it could not start.
        error: std::io::Error,
    },
    /// The program failed.
    Failed {
        /// The program run.
        program: String,
        /// What it wrote to its error output.
        stderr: String,
    },
}

impl fmt::Display for MovieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn { program, error } => write!(
                f,
                "cannot run {program:?} ({error}); install ffmpeg or set animation.ffmpeg_path"
            ),
            Self::Failed { program, stderr } => {
                write!(
                    f,
                    "{program:?} could not encode the video: {}",
                    stderr.trim()
                )
            }
        }
    }
}

impl std::error::Error for MovieError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { error, .. } => Some(error),
            Self::Failed { .. } => None,
        }
    }
}

/// The containers whose extension names their codec.
const SELF_CODED: [&str; 5] = ["apng", "avif", "gif", "webm", "webp"];

/// The ffmpeg command line for `out` (matplotlib's `FFMpegWriter._args`
/// and `output_args`).
fn ffmpeg_args(
    out: &Path,
    (width, height): (usize, usize),
    fps: &str,
    options: &MovieOptions,
) -> Vec<String> {
    let rc = crate::rc::params();
    let mut args: Vec<String> = [
        "-f",
        "rawvideo",
        "-vcodec",
        "rawvideo",
        "-s",
        &format!("{width}x{height}"),
        "-pix_fmt",
        "rgba",
        "-framerate",
        fps,
        "-loglevel",
        "error",
        "-i",
        "pipe:",
    ]
    .map(String::from)
    .to_vec();
    let suffix = out
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_owned();
    let codec = if SELF_CODED.contains(&suffix.as_str()) {
        suffix
    } else {
        let codec = options
            .codec
            .clone()
            .unwrap_or_else(|| rc.animation_codec.clone());
        args.extend(["-vcodec".to_owned(), codec.clone()]);
        codec
    };
    let extra = options
        .extra_args
        .clone()
        .unwrap_or_else(|| rc.animation_ffmpeg_args.clone());
    let has = |flag: &str| extra.iter().any(|arg| arg == flag);
    match codec.as_str() {
        "h264" if !has("-pix_fmt") => args.extend(["-pix_fmt", "yuv420p"].map(String::from)),
        "gif" if !has("-filter_complex") => args.extend(
            [
                "-filter_complex",
                "split [a][b];[a] palettegen [p];[b][p] paletteuse",
            ]
            .map(String::from),
        ),
        "avif" if !has("-filter_complex") => args.extend(
            [
                "-filter_complex",
                "split [rgb][rgba]; [rgba] alphaextract [alpha]",
                "-map",
                "[rgb]",
                "-map",
                "[alpha]",
            ]
            .map(String::from),
        ),
        _ => {}
    }
    let bitrate = options.bitrate.unwrap_or(rc.animation_bitrate);
    if bitrate > 0 {
        args.extend(["-b".to_owned(), format!("{bitrate}k")]);
    }
    for (key, value) in &options.metadata {
        args.extend(["-metadata".to_owned(), format!("{key}={value}")]);
    }
    args.extend(extra);
    args.extend(["-y".to_owned(), out.display().to_string()]);
    args
}

impl FuncAnimation {
    /// Encodes the animation as a video at `path` with ffmpeg
    /// (matplotlib's `save(..., writer="ffmpeg")`): the frames are rendered
    /// as `savefig` would and piped to `animation.ffmpeg_path` as raw RGBA.
    ///
    /// # Errors
    ///
    /// When a frame cannot be rendered, ffmpeg cannot be started or it
    /// fails.
    pub fn save_movie(
        &self,
        path: impl AsRef<Path>,
        options: MovieOptions,
    ) -> Result<(), AnimationError> {
        let path = path.as_ref();
        let fps = options
            .fps
            .unwrap_or_else(|| 1000.0 / interval_millis(self.player.borrow().options.interval));
        let size = self.frame_size()?;
        let program = crate::rc::params().animation_ffmpeg_path.clone();
        let movie = |error| AnimationError::Movie(error);
        let mut child = Command::new(&program)
            .args(ffmpeg_args(path, size, &pyfmt::repr(fps), &options))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                movie(MovieError::Spawn {
                    program: program.clone(),
                    error,
                })
            })?;
        // Drained on its own thread so that a chatty encoder cannot block
        // while its input is still being written.
        let stderr = child.stderr.take().map(|mut pipe| {
            std::thread::spawn(move || {
                let mut text = String::new();
                let _ = pipe.read_to_string(&mut text);
                text
            })
        });
        let written = child.stdin.take().map_or(Ok(()), |mut stdin| {
            self.for_each_frame(|figure| {
                let image = render_frame(figure)?;
                stdin.write_all(image.data()).map_err(SaveError::Io)
            })
        });
        let status = child.wait();
        let stderr = stderr
            .and_then(|thread| thread.join().ok())
            .unwrap_or_default();
        let failed = || {
            movie(MovieError::Failed {
                program: program.clone(),
                stderr: stderr.clone(),
            })
        };
        match status {
            Ok(status) if status.success() => written,
            // A broken pipe only means the encoder stopped early; its
            // error output says why.
            Ok(_) => Err(failed()),
            Err(error) => Err(AnimationError::Save(SaveError::Io(error))),
        }
    }

    /// The animation as an HTML5 `<video>` element with an h264 video
    /// embedded in base64 (matplotlib's `to_html5_video`), autoplaying with
    /// controls and looping when the animation repeats; matplotlib's "Video
    /// too large to embed." when the encoded video reaches
    /// `animation.embed_limit` megabytes.
    ///
    /// # Errors
    ///
    /// As [`FuncAnimation::save_movie`].
    pub fn to_html5_video(&self) -> Result<String, AnimationError> {
        static VIDEOS: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "pltrs-video-{}-{}.m4v",
            std::process::id(),
            VIDEOS.fetch_add(1, Ordering::Relaxed)
        ));
        let options = MovieOptions {
            codec: Some("h264".into()),
            ..MovieOptions::default()
        };
        let encoded = self.save_movie(&path, options).and_then(|()| {
            std::fs::read(&path).map_err(|err| AnimationError::Save(SaveError::Io(err)))
        });
        let _ = std::fs::remove_file(&path);
        let video = base64::encode_lines(&encoded?);
        let limit = crate::rc::params().animation_embed_limit * 1024.0 * 1024.0;
        if byte_count(&video) >= limit {
            return Ok("Video too large to embed.".to_owned());
        }
        let (width, height) = self.frame_size()?;
        let looping = if self.player.borrow().options.repeat {
            " loop"
        } else {
            ""
        };
        Ok(format!(
            "<video width=\"{width}\" height=\"{height}\" controls autoplay{looping}>\n  \
             <source type=\"video/mp4\" src=\"data:video/mp4;base64,{video}\">\n  \
             Your browser does not support the video tag.\n</video>"
        ))
    }

    /// The size of a frame in pixels: the figure at the save resolution
    /// (matplotlib's `MovieWriter.frame_size`).
    fn frame_size(&self) -> Result<(usize, usize), AnimationError> {
        let dpi = match crate::rc::params().savefig_dpi {
            Dpi::Value(dpi) => dpi,
            Dpi::Figure => self.figure.dpi(),
        };
        let (width, height) = self.figure.size_inches();
        let image = Figure::raster_canvas_sized(width, height, dpi)
            .map_err(AnimationError::Save)?
            .to_image();
        Ok((image.width(), image.height()))
    }
}

/// The length of `data` in bytes, as a number to compare with the limit.
fn byte_count(data: &str) -> f64 {
    data.len() as f64
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use super::*;
    use crate::animation::{AnimationOptions, Frames};

    /// A path for a file in the temporary directory.
    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("pltrs-movie-{}-{name}", std::process::id()))
    }

    /// A stand-in for ffmpeg: records its arguments and input, writes a
    /// fake movie to its last argument, and fails when asked.
    fn fake_ffmpeg(name: &str, fail: bool) -> PathBuf {
        let script = temp_path(name);
        let body = if fail {
            "cat > /dev/null; echo 'Unknown encoder' >&2; exit 1".to_owned()
        } else {
            format!(
                "echo \"$@\" > {0}.args; cat > {0}.frames; for last; do :; done; printf MOVIE > \"$last\"",
                script.display()
            )
        };
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn animation() -> FuncAnimation {
        let fig = Figure::with_size(1.0, 0.5, 20.0);
        let ax = fig.subplot();
        let line = ax.plot(vec![0.0, 1.0], vec![0.0, 0.0]).unwrap();
        FuncAnimation::new(
            &fig,
            Frames::Count(3),
            AnimationOptions::default(),
            move |i| {
                line.set_data([0.0, 1.0], [0.0, i as f64]);
            },
        )
    }

    #[test]
    fn frames_are_piped_to_ffmpeg() {
        let script = fake_ffmpeg("ok", false);
        let out = temp_path("movie.mp4");
        crate::rc::context(|| {
            crate::rc::set("animation.ffmpeg_path", &script.display().to_string()).unwrap();
            animation().save(&out).unwrap();
        });
        let args = std::fs::read_to_string(format!("{}.args", script.display())).unwrap();
        assert!(args.starts_with("-f rawvideo -vcodec rawvideo -s 20x10 -pix_fmt rgba"));
        assert!(args.contains("-framerate 5.0 -loglevel error -i pipe: -vcodec h264"));
        assert!(args.contains("-pix_fmt yuv420p -y"));
        let frames = std::fs::read(format!("{}.frames", script.display())).unwrap();
        assert_eq!(frames.len(), 3 * 20 * 10 * 4, "three RGBA frames");
        assert_eq!(std::fs::read(&out).unwrap(), b"MOVIE");
        crate::rc::context(|| {
            crate::rc::set("animation.ffmpeg_path", &script.display().to_string()).unwrap();
            let tag = animation().to_html5_video().unwrap();
            assert!(tag.starts_with("<video width=\"20\" height=\"10\" controls autoplay loop>"));
            assert!(tag.contains("base64,TU9WSUU="), "{tag}");
        });
    }

    #[test]
    fn ffmpeg_failures_are_reported() {
        let script = fake_ffmpeg("fail", true);
        crate::rc::context(|| {
            crate::rc::set("animation.ffmpeg_path", &script.display().to_string()).unwrap();
            let err = animation().save(temp_path("bad.mp4")).unwrap_err();
            assert!(err.to_string().contains("Unknown encoder"), "{err}");
            crate::rc::set("animation.ffmpeg_path", "/nonexistent/ffmpeg").unwrap();
            let err = animation().save(temp_path("none.mp4")).unwrap_err();
            assert!(matches!(
                err,
                AnimationError::Movie(MovieError::Spawn { .. })
            ));
        });
    }

    #[test]
    fn containers_choose_their_codec() {
        let options = MovieOptions {
            bitrate: Some(800),
            metadata: vec![("artist".into(), "pltrs".into())],
            ..MovieOptions::default()
        };
        let args = ffmpeg_args(Path::new("out.gif"), (4, 2), "10", &options).join(" ");
        assert!(!args.contains("-vcodec h264"));
        assert!(args.contains("palettegen"));
        assert!(args.contains("-b 800k -metadata artist=pltrs"));
        assert!(args.ends_with("-y out.gif"));
    }
}
