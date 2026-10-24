use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{GraphicsContext, Renderer, RgbaImage};

use super::*;

/// Records the `draw_path` calls it receives.
#[derive(Default)]
struct Recorder {
    calls: Vec<(GraphicsContext, Path, Affine2D, Option<Rgba>)>,
}

impl Renderer for Recorder {
    fn canvas_size(&self) -> (f64, f64) {
        (100.0, 100.0)
    }

    fn dpi(&self) -> f64 {
        144.0
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.calls
            .push((gc.clone(), path.clone(), *transform, fill));
    }

    fn draw_gouraud_triangles(
        &mut self,
        _: &GraphicsContext,
        _: &[[Point; 3]],
        _: &[[Rgba; 3]],
        _: &Affine2D,
    ) {
    }

    fn draw_image(&mut self, _: &GraphicsContext, _: Point, _: &RgbaImage, _: Option<&Affine2D>) {}
}

fn segment() -> Path {
    Path::polyline(vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)])
}

fn draw(effects: Vec<PathEffect>, gc: &GraphicsContext, fill: Option<Rgba>) -> Recorder {
    let mut recorder = Recorder::default();
    PathEffectRenderer::new(effects, &mut recorder)
        .unwrap()
        .draw_path(gc, &segment(), &Affine2D::IDENTITY, fill);
    recorder
}

#[test]
fn with_stroke_draws_the_stroke_then_the_path() {
    let stroke = Stroke::new(GcUpdate::default().linewidth(4.0).foreground("red"));
    let gc = GraphicsContext::default();
    let calls = draw(vec![PathEffect::with_stroke(stroke)], &gc, None).calls;
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0.linewidth, 4.0);
    assert_eq!(calls[0].0.foreground, Rgba::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(calls[1].0, gc);
}

#[test]
fn patch_shadows_darken_the_face_and_offset_in_points() {
    let gc = GraphicsContext::default();
    let face = Rgba::new(1.0, 0.5, 0.0, 1.0);
    let calls = draw(
        vec![PathEffect::simple_patch_shadow(PatchShadow::default())],
        &gc,
        Some(face),
    )
    .calls;
    let (shadow_gc, _, affine, fill) = &calls[0];
    assert_eq!(shadow_gc.alpha, Some(0.3));
    assert_eq!(shadow_gc.linewidth, 0.0);
    assert_eq!(*fill, Some(Rgba::new(0.3, 0.15, 0.0, 1.0)));
    // 2 points at 144 dpi are 4 pixels.
    assert_eq!(affine.transform_point(Point::ORIGIN), Point::new(4.0, -4.0));
}

#[test]
fn line_shadows_are_unfilled() {
    let gc = GraphicsContext::default();
    let calls = draw(
        vec![
            PathEffect::simple_line_shadow(LineShadow::default()),
            PathEffect::normal(),
        ],
        &gc,
        Some(Rgba::WHITE),
    )
    .calls;
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].3, None);
    assert_eq!(calls[0].0.foreground, Rgba::BLACK);
    assert_eq!(calls[1].3, Some(Rgba::WHITE));
}

#[test]
fn ticks_follow_the_path() {
    let gc = GraphicsContext::default();
    let ticks = TickedStroke {
        angle: 90.0,
        length: 1.0,
        ..TickedStroke::default()
    };
    let calls = draw(vec![PathEffect::ticked_stroke(ticks)], &gc, None).calls;
    let (_, path, affine, _) = &calls[0];
    // 10 points at 144 dpi = 20 px: ceil(100 / 20) - 1 = 4 ticks spread
    // from 10 to 90 px.
    assert_eq!(path.len(), 8);
    let start = affine.transform_point(path.vertices()[0]);
    let end = affine.transform_point(path.vertices()[1]);
    assert!((start - Point::new(10.0, 0.0)).length() < 1e-9);
    assert!((end - Point::new(10.0, 20.0)).length() < 1e-9, "{end:?}");
}

#[test]
fn colours_are_resolved_when_drawn() {
    let bad = PatchShadow::default().with_color("not a colour");
    let effects = vec![PathEffect::simple_patch_shadow(bad), PathEffect::normal()];
    let mut recorder = Recorder::default();
    assert!(PathEffectRenderer::new(effects.clone(), &mut recorder).is_err());

    let fig = crate::figure::Figure::new();
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    ax.set_axis_off();
    let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
    crate::artist::ArtistHandle::set_path_effects(&line, effects);
    let mut renderer = crate::testing::RecordingRenderer::new(640.0, 480.0, 100.0);
    let errors = fig.draw(&mut renderer).unwrap_err();
    assert_eq!(errors.0.len(), 1, "the shadow is reported");
    let widths: Vec<f64> = renderer
        .calls
        .iter()
        .filter_map(|call| match call {
            crate::testing::Call::Path { linewidth, .. } => Some(*linewidth),
            _ => None,
        })
        .collect();
    assert_eq!(widths, [0.0], "the background, but not the line");
}

#[test]
fn degenerate_paths_get_no_ticks() {
    let gc = GraphicsContext::default();
    let mut recorder = Recorder::default();
    PathEffectRenderer::new(
        vec![PathEffect::ticked_stroke(TickedStroke::default())],
        &mut recorder,
    )
    .unwrap()
    .draw_path(
        &gc,
        &Path::polyline(vec![Point::ORIGIN]),
        &Affine2D::IDENTITY,
        None,
    );
    assert_eq!(recorder.calls.len(), 0);
}

#[test]
fn markers_are_drawn_once_per_effect() {
    let gc = GraphicsContext::default();
    let mut recorder = Recorder::default();
    let marker = Path::unit_rectangle();
    let points = Path::polyline(vec![Point::new(1.0, 1.0), Point::new(f64::NAN, 2.0)]);
    PathEffectRenderer::new(
        vec![PathEffect::normal(), PathEffect::normal()],
        &mut recorder,
    )
    .unwrap()
    .draw_markers(
        &gc,
        &marker,
        &Affine2D::IDENTITY,
        &points,
        &Affine2D::IDENTITY,
        None,
    );
    assert_eq!(recorder.calls.len(), 2, "one finite position, two effects");
}

/// The paths an artist figure draws, recorded.
fn drawn(fig: &crate::figure::Figure) -> Vec<crate::testing::Call> {
    let mut renderer = crate::testing::RecordingRenderer::new(640.0, 480.0, 100.0);
    fig.draw(&mut renderer).expect("drawable");
    renderer.calls
}

#[test]
fn lines_draw_through_their_effects() {
    use crate::artist::ArtistHandle;
    let fig = crate::figure::Figure::new();
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    ax.set_axis_off();
    let halo = Stroke::new(GcUpdate::default().linewidth(6.0));
    ax.plot([0.0, 1.0], [0.0, 1.0])
        .expect("same length")
        .set_path_effects(vec![PathEffect::with_stroke(halo)]);
    let widths: Vec<f64> = drawn(&fig)
        .iter()
        .filter_map(|call| match call {
            crate::testing::Call::Path { linewidth, .. } => Some(*linewidth),
            _ => None,
        })
        .collect();
    // The figure background, then the stroke and the line.
    assert_eq!(widths, [0.0, 6.0, 1.5]);
}

#[test]
fn text_with_effects_is_drawn_as_one_outline() {
    use crate::artist::ArtistHandle;
    let fig = crate::figure::Figure::new();
    let shadowed = PatchShadow::default();
    fig.text(0.5, 0.5, "ab")
        .set_path_effects(vec![PathEffect::with_simple_patch_shadow(shadowed)]);
    let calls = drawn(&fig);
    assert_eq!(calls.len(), 3, "the background, the shadow, then the text");
    let crate::testing::Call::Path {
        fill, linewidth, ..
    } = &calls[2]
    else {
        panic!("text outlines are paths");
    };
    assert_eq!((*fill, *linewidth), (Some(Rgba::BLACK), 0.0));
}

#[test]
fn rc_files_can_only_clear_the_path_effects() {
    let mut rc = crate::rc::RcParams::matplotlib();
    assert!(rc.set("path.effects", "[]").is_ok());
    assert_eq!(rc.path_effects, Vec::new());
    assert!(rc.set("path.effects", "withStroke").is_err());
    rc.path_effects = vec![PathEffect::normal()];
    let shown = rc.get("path.effects").expect("a parameter");
    assert!(shown.contains("Normal"), "{shown}");
}
