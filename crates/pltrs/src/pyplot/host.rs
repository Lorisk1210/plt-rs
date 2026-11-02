//! What shows pyplot's figures, and those [`gui::show`](crate::gui::show)
//! is given: the window session, the web server, or nothing, as the
//! backend says.

use std::time::Duration;

use super::backend::{Backend, get_backend, record_backend};
use crate::events::GinputOptions;
use crate::figure::Figure;
use crate::gui::{GuiError, Session};
use crate::web::{self, WebOptions, WebServer};

/// A figure to show: its number, the figure and its title.
pub(super) type Shown = (u32, Figure, String);

/// What stands in for windows that cannot open (matplotlib's backend
/// fallback). On Linux and the other Unix systems, that means a machine
/// without a display, where matplotlib falls back to `agg` and nothing is
/// shown; macOS and Windows always have a display, so there the browser
/// takes over.
fn fallback() -> Backend {
    if cfg!(all(unix, not(target_os = "macos"))) {
        Backend::Agg
    } else {
        Backend::WebAgg
    }
}

/// Shows `figures`, titled "Figure 1", "Figure 2", ..., with the thread's
/// backend, and returns once they are all closed (see
/// [`gui::show`](crate::gui::show)).
pub(crate) fn show_until_closed(figures: &[Figure]) -> Result<(), GuiError> {
    let (mut host, backend) = Host::start(get_backend())?;
    record_backend(backend);
    let shown: Vec<Shown> = (1..)
        .zip(figures)
        .map(|(number, figure)| (number, figure.clone(), format!("Figure {number}")))
        .collect();
    host.show(&shown)?;
    host.run()
}

/// The running backend.
#[derive(Debug)]
pub(super) enum Host {
    /// Figure windows.
    Windows(Session),
    /// The web server.
    Web(WebServer),
    /// Nothing is displayed.
    Agg,
}

impl Host {
    /// Starts `backend`; returns the host and the backend it runs, which
    /// is the [`fallback`] when windows were asked for but cannot open and
    /// `backend_fallback` is on.
    pub(super) fn start(backend: Backend) -> Result<(Self, Backend), GuiError> {
        match backend {
            Backend::Agg => Ok((Self::Agg, Backend::Agg)),
            Backend::WebAgg => Ok((Self::web()?, Backend::WebAgg)),
            Backend::Windows => match Session::new() {
                Ok(session) => Ok((Self::Windows(session), Backend::Windows)),
                Err(GuiError::Platform(_)) if crate::rc::params().backend_fallback => {
                    Self::start(fallback())
                }
                Err(err) => Err(err),
            },
        }
    }

    /// Starts the web server the `webagg.*` rcParams describe and tells
    /// the user where it is.
    fn web() -> Result<Self, GuiError> {
        let options = WebOptions::from_rc()?;
        let open_in_browser = options.open_in_browser;
        let server = WebServer::bind(options)?;
        web::announce(&server, open_in_browser);
        Ok(Self::Web(server))
    }

    /// Shows the figures not shown yet and redraws the changed ones.
    pub(super) fn show(&mut self, figures: &[Shown]) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => {
                let titled: Vec<(Figure, String)> = figures
                    .iter()
                    .map(|(_, figure, title)| (figure.clone(), title.clone()))
                    .collect();
                session.show(&titled)
            }
            Self::Web(server) => figures.iter().try_for_each(|(number, figure, title)| {
                server
                    .add_figure_as(*number, figure, title)
                    .map_err(GuiError::from)
            }),
            Self::Agg => Ok(()),
        }
    }

    /// Stops showing `figure`.
    pub(super) fn close(&mut self, figure: &Figure) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => session.close(figure),
            Self::Web(server) => {
                server.close(figure);
                Ok(())
            }
            Self::Agg => Ok(()),
        }
    }

    /// The figures the user closed since the last call.
    pub(super) fn take_closed(&mut self) -> Vec<Figure> {
        match self {
            Self::Windows(session) => session.take_closed(),
            Self::Web(server) => server.take_closed(),
            Self::Agg => Vec::new(),
        }
    }

    /// Runs until every shown figure is closed.
    pub(super) fn run(&mut self) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => session.run(None, || false),
            Self::Web(server) => {
                server.run(None, || false);
                Ok(())
            }
            Self::Agg => Ok(()),
        }
    }

    /// Keeps the figures responsive for `duration`.
    pub(super) fn pause(&mut self, duration: Duration) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => session.pause(duration),
            Self::Web(server) => {
                server.pause(duration);
                Ok(())
            }
            Self::Agg => {
                std::thread::sleep(duration);
                Ok(())
            }
        }
    }

    /// `ginput` in the shown `figure`.
    pub(super) fn ginput(
        &mut self,
        figure: &Figure,
        n: usize,
        timeout: Option<Duration>,
        options: GinputOptions,
    ) -> Result<Vec<(f64, f64)>, GuiError> {
        match self {
            Self::Windows(session) => session.ginput(figure, n, timeout, options),
            Self::Web(server) => Ok(server.ginput(figure, n, timeout, options)),
            Self::Agg => Ok(Vec::new()),
        }
    }

    /// `waitforbuttonpress` in the shown `figure`.
    pub(super) fn wait_for_button_press(
        &mut self,
        figure: &Figure,
        timeout: Option<Duration>,
    ) -> Result<Option<bool>, GuiError> {
        match self {
            Self::Windows(session) => session.wait_for_button_press(figure, timeout),
            Self::Web(server) => Ok(server.wait_for_button_press(figure, timeout)),
            Self::Agg => Ok(None),
        }
    }

    /// Resizes the canvas showing `figure` to `width × height` physical
    /// pixels (matplotlib's `manager.resize`): its window, or the figure
    /// itself for the web pages, which follow it; nothing is shown to
    /// resize without a display.
    pub(super) fn resize(
        &mut self,
        figure: &Figure,
        width: u32,
        height: u32,
    ) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => session.resize(figure, width, height),
            Self::Web(_) => {
                let dpi = figure.dpi();
                // At a degenerate resolution the size is invalid: the
                // figure keeps its size, as it does in a window.
                let _ = figure.set_size_inches(f64::from(width) / dpi, f64::from(height) / dpi);
                Ok(())
            }
            Self::Agg => Ok(()),
        }
    }

    /// Opens the subplot configuration tool of `figure`'s window; the web
    /// and non-interactive backends have none.
    pub(super) fn configure_subplots(&mut self, figure: &Figure) -> Result<(), GuiError> {
        match self {
            Self::Windows(session) => session.configure_subplots(figure),
            Self::Web(_) | Self::Agg => Err(GuiError::NotShown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{get_backend, switch_backend};

    #[test]
    fn figures_made_by_hand_show_with_the_backend() {
        switch_backend(Backend::Agg);
        let fig = Figure::new();
        fig.subplot();
        assert!(
            crate::gui::show(&[fig]).is_ok(),
            "agg shows nothing and returns"
        );
        assert_eq!(get_backend(), Backend::Agg);
    }

    #[test]
    fn web_pages_resize_figures_at_any_resolution() {
        let server = WebServer::bind(WebOptions::new("127.0.0.1", 0)).unwrap();
        let mut host = Host::Web(server);
        let fig = Figure::new();
        host.resize(&fig, 320, 240).unwrap();
        assert_eq!(fig.size_inches(), (3.2, 2.4));
        // A degenerate resolution used to panic: the figure keeps its size.
        for dpi in [0.0, -100.0, f64::NAN] {
            fig.set_dpi(dpi);
            host.resize(&fig, 640, 480).unwrap();
            assert_eq!(fig.size_inches(), (3.2, 2.4));
        }
    }
}
