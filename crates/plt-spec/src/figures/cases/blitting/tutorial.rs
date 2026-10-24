//! The figures of `galleries/users_explain/animations/blitting.py` after
//! their 100 updates: the minimal example, and the class-based one with
//! its `BlitManager` and frame counter.

use std::cell::RefCell;
use std::error::Error;
use std::f64::consts::PI;
use std::rc::{Rc, Weak};

use plt::artist::RawHandle;
use plt::events::EventKind;
use plt::figure::{BufferRegion, CanvasError, FigureCanvas};
use plt::prelude::*;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt::transforms::Point;

use super::{blitting, shifted, wave_x};
use crate::figures::{CaseResult, FigureCase};

/// The phase of update `j` of 100 (`(j / 100) * np.pi`).
fn phase(j: u32) -> f64 {
    f64::from(j) / 100.0 * PI
}

fn minimal(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = wave_x();
    let ln = ax.plot(x.clone(), shifted(&x, 0.0, f64::sin))?;
    ln.set_animated(true);
    let canvas = fig.canvas();
    canvas.draw()?;
    let bg = canvas.copy_from_bbox(&fig.bbox())?;
    ax.draw_artist(&ln)?;
    canvas.blit(Some(&fig.bbox()))?;
    for j in 0..100 {
        canvas.restore_region(&bg)?;
        ln.set_data(&x, &shifted(&x, phase(j), f64::sin));
        ax.draw_artist(&ln)?;
        canvas.blit(Some(&fig.bbox()))?;
    }
    Ok(())
}

/// The tutorial's `BlitManager`: saves the background on every draw of
/// the figure and draws its animated artists over it.
struct BlitManager {
    canvas: FigureCanvas,
    background: Option<BufferRegion>,
    artists: Vec<RawHandle>,
    /// The first problem met by the draw callback, which cannot return it.
    problem: Option<CanvasError>,
}

impl BlitManager {
    /// Manages `artists` on `canvas`: they become animated, and the
    /// manager saves the background whenever the figure is drawn.
    fn new(canvas: FigureCanvas, artists: Vec<RawHandle>) -> Rc<RefCell<Self>> {
        for artist in &artists {
            artist.set_animated(true);
        }
        let figure = canvas.figure().clone();
        let manager = Rc::new(RefCell::new(Self {
            canvas,
            background: None,
            artists,
            problem: None,
        }));
        let managing: Weak<RefCell<Self>> = Rc::downgrade(&manager);
        figure.connect(EventKind::Draw, move |_| {
            if let Some(manager) = managing.upgrade() {
                let mut manager = manager.borrow_mut();
                if let Err(problem) = manager.on_draw() {
                    manager.problem.get_or_insert(problem);
                }
            }
        });
        manager
    }

    /// The draw callback: saves the background, then draws the artists.
    fn on_draw(&mut self) -> Result<(), CanvasError> {
        let figure = self.canvas.figure();
        self.background = Some(self.canvas.copy_from_bbox(&figure.bbox())?);
        self.draw_animated()
    }

    fn draw_animated(&self) -> Result<(), CanvasError> {
        let figure = self.canvas.figure();
        self.artists
            .iter()
            .try_for_each(|artist| figure.draw_artist(artist))
    }

    /// Shows the artists' new state: the background put back, the artists
    /// drawn over it and the figure blitted.
    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(problem) = self.problem.take() {
            return Err(problem.into());
        }
        let Some(background) = &self.background else {
            return Ok(self.on_draw()?);
        };
        self.canvas.restore_region(background)?;
        self.draw_animated()?;
        self.canvas.blit(Some(&self.canvas.figure().bbox()))?;
        Ok(())
    }
}

fn blit_manager(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = wave_x();
    let ln = ax.plot(x.clone(), shifted(&x, 0.0, f64::sin))?;
    let options = AnnotateOptions {
        xytext: Some(Point::new(10.0, -10.0)),
        xycoords: "axes fraction".parse()?,
        textcoords: Some("offset points".parse()?),
        ..AnnotateOptions::default()
    };
    let fr_number = ax.annotate("0", (0.0, 1.0), options)?;
    fr_number.set_halign(HAlign::Left).set_valign(VAlign::Top);
    let artists = vec![ln.raw().clone(), fr_number.raw().clone()];
    let bm = BlitManager::new(fig.canvas(), artists);
    fig.canvas().draw()?;
    for j in 0..100 {
        ln.set_data(&x, &shifted(&x, phase(j), f64::sin));
        fr_number.set_text(format!("frame: {j}"));
        bm.borrow_mut().update()?;
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    blitting("blitting/minimal", minimal),
    blitting("blitting/blit_manager", blit_manager),
];
