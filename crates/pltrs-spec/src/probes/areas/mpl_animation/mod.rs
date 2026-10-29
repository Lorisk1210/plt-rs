//! Probe operations for the `mpl_animation` area: matplotlib's own tests
//! of `matplotlib.animation` (`tests/test_animation.py`; see
//! `spec/reference/probes/test_mpl_animation.py`): saving animations as
//! GIFs, animated PNGs and HTML pages, the frame size, transparency, the
//! embed limit and the notebook representation. [`movies`] reads the
//! saved files as Pillow does.

mod movies;

use std::path::Path;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation, HtmlOptions};
use pltrs::figure::Figure;
use pltrs::numeric::linspace;
use pltrs::rc;

use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Op, Outcome, ProbeError};
use crate::scratch::ScratchDir;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Animation.save", save),
    Op::new("Animation.save.frame_size", frame_size),
    Op::new("Animation.save.invalid_path", invalid_path),
    Op::new("Animation.save.transparent", transparent),
    Op::new("Animation.to_jshtml.embed_limit", embed_limit),
    Op::new("Animation._repr_html_", repr_html),
];

/// The `anim` fixture of `test_animation.py`: a line set to `sin(x + i)`
/// in frame `i`, emptied by the init function.
fn anim(frames: usize, repeat: bool) -> Result<FuncAnimation, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let line = ax
        .plot(Vec::<f64>::new(), Vec::<f64>::new())
        .map_err(ProbeError::rejected)?;
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(-1.0, 1.0))
        .map_err(ProbeError::rejected)?;
    let options = AnimationOptions {
        repeat,
        ..AnimationOptions::default()
    };
    let cleared = line.clone();
    Ok(FuncAnimation::with_init(
        &fig,
        Frames::Count(frames),
        options,
        move || {
            cleared.set_data(Vec::<f64>::new(), Vec::<f64>::new());
        },
        move |i| {
            let x = linspace(0.0, 10.0, 100);
            let y: Vec<f64> = x.iter().map(|x| (x + i as f64).sin()).collect();
            line.set_data(&x, &y);
        },
    ))
}

/// What the probe records of the file at `path`, saved as `format`.
fn read_movie(path: &Path, format: &str) -> Outcome {
    let data = std::fs::read(path).map_err(ProbeError::rejected)?;
    let movie = match format {
        "gif" => movies::gif(&data)?,
        _ => movies::apng(&data)?,
    };
    Ok(movie.to_json())
}

/// The file name the probe saves `format` to, checked.
fn movie_name(format: &str) -> Result<String, ProbeError> {
    match format {
        "gif" | "png" | "html" => Ok(format!("movie.{format}")),
        other => Err(ProbeError::BadArgs(format!("unknown format {other:?}"))),
    }
}

/// `anim.save("movie.<format>")`: what Pillow reads of a GIF or animated
/// PNG, or the frame files of an HTML page.
fn save(args: Args<'_>) -> Outcome {
    styled(args, || {
        let frames = usize::try_from(args.i64("frames")?)
            .map_err(|_| ProbeError::BadArgs("frames is negative".into()))?;
        let animation = anim(frames, args.bool("repeat")?)?;
        let format = args.str("format")?;
        let scratch = ScratchDir::new("mpl-animation").map_err(ProbeError::rejected)?;
        let path = scratch.path(&movie_name(format)?);
        animation.save(&path).map_err(ProbeError::rejected)?;
        if format != "html" {
            return read_movie(&path, format);
        }
        let mut names = std::fs::read_dir(scratch.path("movie_frames"))
            .map_err(ProbeError::rejected)?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .map_err(ProbeError::rejected)
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        Ok(Json::Array(names.into_iter().map(Json::from).collect()))
    })
}

/// The size of the frames of a one-frame animation of an empty figure
/// saved as a GIF.
fn frame_size(args: Args<'_>) -> Outcome {
    styled(args, || {
        let &[width, height] = args.f64_vec("figsize")?.as_slice() else {
            return Err(ProbeError::BadArgs("figsize is malformed".into()));
        };
        let fig = Figure::with_size(width, height, args.f64("dpi")?);
        let animation =
            FuncAnimation::new(&fig, Frames::Count(1), AnimationOptions::default(), |_| {});
        let scratch = ScratchDir::new("mpl-animation").map_err(ProbeError::rejected)?;
        let path = scratch.path("movie.gif");
        animation.save(&path).map_err(ProbeError::rejected)?;
        let size = movies::gif(&std::fs::read(&path).map_err(ProbeError::rejected)?)?.size;
        Ok(Json::Array(vec![
            Json::from(f64::from(size.0)),
            Json::from(f64::from(size.1)),
        ]))
    })
}

/// `anim.save("missing/movie.<format>")` in an empty directory.
fn invalid_path(args: Args<'_>) -> Outcome {
    styled(args, || {
        let scratch = ScratchDir::new("mpl-animation").map_err(ProbeError::rejected)?;
        let path = scratch
            .path("missing")
            .join(movie_name(args.str("format")?)?);
        anim(5, true)?.save(&path).map_err(ProbeError::rejected)?;
        Ok(Json::Null)
    })
}

/// The smallest alpha of the first frame of the animation saved as a GIF
/// with `savefig.transparent` set.
fn transparent(args: Args<'_>) -> Outcome {
    styled(args, || {
        rc::set("savefig.transparent", "True").map_err(ProbeError::rejected)?;
        let scratch = ScratchDir::new("mpl-animation").map_err(ProbeError::rejected)?;
        let path = scratch.path("movie.gif");
        anim(5, true)?.save(&path).map_err(ProbeError::rejected)?;
        let data = std::fs::read(&path).map_err(ProbeError::rejected)?;
        let first = pltrs_codec::gif::decode(&data).map_err(ProbeError::rejected)?;
        let alpha = first.to_rgba8().iter().map(|pixel| pixel[3]).min();
        Ok(alpha.map_or(Json::Null, |alpha| Json::from(f64::from(alpha))))
    })
}

/// How many frames `to_jshtml` embeds under `animation.embed_limit`.
fn embed_limit(args: Args<'_>) -> Outcome {
    styled(args, || {
        let limit = args.f64("embed_limit")?;
        rc::set("animation.embed_limit", &limit.to_string()).map_err(ProbeError::rejected)?;
        let frames = usize::try_from(args.i64("frames")?)
            .map_err(|_| ProbeError::BadArgs("frames is negative".into()))?;
        let html = anim(frames, true)?
            .to_jshtml(HtmlOptions::default())
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(
            html.matches("data:image/png;base64,").count() as f64
        ))
    })
}

/// The notebook representation under `animation.html`: null, or whether
/// it holds a script.
fn repr_html(args: Args<'_>) -> Outcome {
    styled(args, || {
        rc::set("animation.html", args.str("html")?).map_err(ProbeError::rejected)?;
        let representation = anim(5, true)?.repr_html().map_err(ProbeError::rejected)?;
        Ok(representation.map_or(Json::Null, |html| Json::from(html.contains("<script "))))
    })
}
