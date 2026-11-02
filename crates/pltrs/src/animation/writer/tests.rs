use super::*;
use crate::lines::Line2D;
use crate::scratch::Scratch;

/// A figure of 20 × 20 pixels with a line, and the line.
fn figure() -> (Figure, Line2D) {
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let line = fig.subplot().plot(vec![0.0, 1.0], vec![0.0, 0.0]).unwrap();
    (fig, line)
}

/// Grabs three frames of a rising line with `writer`.
fn grab_three(writer: &mut MovieWriter, line: &Line2D) {
    for i in 0..3 {
        line.set_data([0.0, 1.0], [0.0, f64::from(i)]);
        writer.grab_frame().unwrap();
    }
}

#[test]
fn intervals_print_as_python_floor_divides() {
    assert_eq!(html_interval(5.0), "200");
    assert_eq!(html_interval(7.5), "133.0");
    assert_eq!(html_interval(3.0), "333");
}

#[test]
fn images_hold_every_frame_at_the_writers_rate() {
    let scratch = Scratch::new("writer-images");
    let dir = scratch.path("");
    let (fig, line) = figure();
    let path = dir.join("rise.gif");
    let options = MovieOptions {
        fps: Some(4.0),
        ..MovieOptions::default()
    };
    let mut writer = MovieWriter::new(&fig, &path, None, options).unwrap();
    assert_eq!(writer.frame_size(), (20, 20));
    grab_three(&mut writer, &line);
    writer.finish().unwrap();
    let gif = std::fs::read(&path).unwrap();
    assert_eq!(gif.windows(3).filter(|w| *w == [0x21, 0xF9, 4]).count(), 3);
    // 1000 / 4 = 250 ms: 25 centiseconds.
    let control = gif.windows(3).position(|w| w == [0x21, 0xF9, 4]).unwrap();
    assert_eq!(&gif[control + 4..control + 6], [25, 0]);

    let apng = dir.join("rise.apng");
    let mut writer = MovieWriter::new(&fig, &apng, Some(40.0), MovieOptions::default()).unwrap();
    assert_eq!(writer.frame_size(), (40, 40), "at the resolution given");
    grab_three(&mut writer, &line);
    writer.finish().unwrap();
    let data = std::fs::read(&apng).unwrap();
    assert_eq!(data.windows(4).filter(|w| *w == b"fcTL").count(), 3);
}

#[test]
fn frames_match_those_savefig_renders() {
    let scratch = Scratch::new("writer-webp");
    let dir = scratch.path("");
    let (fig, line) = figure();
    let path = dir.join("rise.webp");
    let mut writer = MovieWriter::new(&fig, &path, None, MovieOptions::default()).unwrap();
    grab_three(&mut writer, &line);
    writer.finish().unwrap();
    let first = pltrs_codec::webp::decode(&std::fs::read(&path).unwrap()).unwrap();
    line.set_data([0.0, 1.0], [0.0, 0.0]);
    assert_eq!(
        first.to_rgba8().concat(),
        render(&fig, 20.0).unwrap().data()
    );
}

#[test]
fn html_pages_load_their_frames_from_beside_them() {
    let scratch = Scratch::new("writer-html");
    let dir = scratch.path("");
    let (fig, line) = figure();
    let path = dir.join("rise.html");
    let mut writer = MovieWriter::new(&fig, &path, None, MovieOptions::default()).unwrap();
    grab_three(&mut writer, &line);
    writer.finish().unwrap();
    let page = std::fs::read_to_string(&path).unwrap();
    assert!(page.contains("rise_frames"));
    assert!(page.contains("200"), "5 frames per second");
    assert!(dir.join("rise_frames/frame0000002.png").exists());
    assert!(!dir.join("rise_frames/frame0000003.png").exists());
}

#[test]
fn files_without_extension_are_refused_and_empty_images_fail() {
    let scratch = Scratch::new("writer-errors");
    let dir = scratch.path("");
    let (fig, _) = figure();
    let refused = MovieWriter::new(&fig, dir.join("movie"), None, MovieOptions::default());
    assert!(matches!(refused, Err(AnimationError::UnsupportedFormat(_))));
    let empty = MovieWriter::new(&fig, dir.join("empty.gif"), None, MovieOptions::default());
    assert!(matches!(
        empty.unwrap().finish(),
        Err(AnimationError::Gif(pltrs_codec::gif::GifError::NoFrames))
    ));
}
