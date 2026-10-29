//! Lines shared by several axes (matplotlib's `MultiCursor`).

use std::rc::Rc;

use pltrs_core::Point;

use super::{Backgrounds, CursorOptions, Switches, cursor_switches, save_on_draw};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{Event, EventKind, Location};
use crate::figure::{Figure, FigureCanvas};
use crate::lines::Line2D;
use crate::widgets::{Connections, data_in};

/// Lines shared by several axes, possibly of several figures, that follow
/// the pointer over any of them (matplotlib's `MultiCursor`): vertical
/// lines by default, to compare plots sharing an x axis.
#[derive(Debug)]
pub struct MultiCursor {
    switches: Rc<Switches>,
    blit: Option<Rc<Backgrounds>>,
    vertical_lines: Vec<Line2D>,
    horizontal_lines: Vec<Line2D>,
    _connections: Vec<Connections>,
}

cursor_switches!(MultiCursor);

/// The lines of a multicursor, moved together.
#[derive(Debug)]
struct SharedLines {
    axes: Vec<Axes>,
    vertical: Vec<Line2D>,
    horizontal: Vec<Line2D>,
    /// The saved backgrounds of the figures, when blitting.
    blit: Option<Rc<Backgrounds>>,
    /// The canvases of the figures the lines are in.
    canvases: Vec<FigureCanvas>,
}

impl SharedLines {
    fn moved_to(&self, data: Point, (vertical, horizontal): (bool, bool)) {
        for line in &self.vertical {
            line.set_xdata(vec![data.x, data.x]).set_visible(vertical);
        }
        for line in &self.horizontal {
            line.set_ydata(vec![data.y, data.y]).set_visible(horizontal);
        }
        if vertical || horizontal {
            self.redraw(vertical, horizontal);
        }
    }

    /// Draws the lines switched on over the saved backgrounds and blits
    /// every figure. Without blitting the lines changed the figures, which
    /// their windows draw again.
    fn redraw(&self, vertical: bool, horizontal: bool) {
        let Some(backgrounds) = &self.blit else {
            return;
        };
        backgrounds.restore();
        let shown = |on: bool, lines: &[Line2D]| -> Vec<(Axes, Line2D)> {
            if on {
                self.axes
                    .iter()
                    .cloned()
                    .zip(lines.iter().cloned())
                    .collect()
            } else {
                Vec::new()
            }
        };
        let mut drawn = shown(vertical, &self.vertical);
        drawn.extend(shown(horizontal, &self.horizontal));
        for (ax, line) in drawn {
            ax.figure().canvas().report(ax.draw_artist(&line));
        }
        for canvas in &self.canvases {
            canvas.report(canvas.blit(None));
        }
    }
}

impl MultiCursor {
    /// Lines across each of `axes`, starting hidden at the middle of the
    /// last one's view (see [`CursorOptions::multicursor`] for matplotlib's
    /// defaults).
    pub fn new(axes: &[Axes], options: CursorOptions) -> Self {
        let props = options.line.clone().visible(false);
        let (xmid, ymid) = axes.last().map_or((0.5, 0.5), |ax| {
            let ((x0, x1), (y0, y1)) = (ax.xlim(), ax.ylim());
            (f64::midpoint(x0, x1), f64::midpoint(y0, y1))
        });
        let vertical: Vec<Line2D> = axes
            .iter()
            .map(|ax| ax.axvline(xmid, props.clone()))
            .collect();
        let horizontal: Vec<Line2D> = axes
            .iter()
            .map(|ax| ax.axhline(ymid, props.clone()))
            .collect();
        let blit = options.useblit.then(|| {
            for line in vertical.iter().chain(&horizontal) {
                line.set_animated(true);
            }
            Rc::new(Backgrounds::default())
        });
        let mut figures: Vec<Figure> = Vec::new();
        for ax in axes {
            let figure = ax.figure();
            if !figures.contains(&figure) {
                figures.push(figure);
            }
        }
        let switches = Rc::new(Switches::new(&options));
        let lines = Rc::new(SharedLines {
            axes: axes.to_vec(),
            vertical: vertical.clone(),
            horizontal: horizontal.clone(),
            blit: blit.clone(),
            canvases: figures.iter().map(Figure::canvas).collect(),
        });
        let connections = figures
            .into_iter()
            .map(|figure| {
                let mut connections = Connections::new(figure.clone());
                if let Some(backgrounds) = &blit {
                    let (figure, backgrounds) = (figure.clone(), Rc::clone(backgrounds));
                    save_on_draw(&mut connections, &switches, move || {
                        backgrounds.save(&figure.canvas(), &figure.bbox());
                    });
                }
                let members: Vec<Axes> = axes
                    .iter()
                    .filter(|ax| ax.figure() == figure)
                    .cloned()
                    .collect();
                let (switches, lines) = (Rc::clone(&switches), Rc::clone(&lines));
                connections.connect(EventKind::Motion, move |event| {
                    if let Event::Motion(mouse) = event
                        && switches.takes(&figure)
                        && let Some(data) = topmost_data(&members, &mouse.location)
                    {
                        lines.moved_to(data, switches.shown());
                    }
                });
                connections
            })
            .collect();
        Self {
            switches,
            blit,
            vertical_lines: vertical,
            horizontal_lines: horizontal,
            _connections: connections,
        }
    }

    /// The vertical lines, one per axes (matplotlib's `vlines`).
    pub fn vertical_lines(&self) -> &[Line2D] {
        &self.vertical_lines
    }

    /// The horizontal lines, one per axes (matplotlib's `hlines`).
    pub fn horizontal_lines(&self) -> &[Line2D] {
        &self.horizontal_lines
    }
}

/// The data coordinates of `location` in the topmost of `axes` under it,
/// the last of the highest z-order (matplotlib's `_topmost_artist`).
fn topmost_data(axes: &[Axes], location: &Location) -> Option<Point> {
    let at = Point::new(location.x, location.y);
    let ax = axes
        .iter()
        .filter(|ax| ax.contains_point(at))
        .reduce(|top, ax| if ax.zorder() >= top.zorder() { ax } else { top })?;
    data_in(ax, location)
}
