//! `_native.web`: pltrs (Rust)'s web server for the `webagg` backend
//! (`backends/backend_webagg.py`), a [`WebRelay`]: it serves matplotlib's
//! pages, client script and files, and hands the messages of the WebSocket
//! connections and the downloads to matplotlib's `FigureManagerWebAgg`s,
//! which answer through it. Also the files of matplotlib's browser client
//! (`FigureManagerWebAgg.get_static_file_path`, the notebook's script).

use std::time::Duration;

use pltrs::web::{RelayEvent, WebError, WebOptions, WebRelay};

use crate::binding::prelude::*;
use crate::binding::wrap_native;

impl IntoPy for WebRelay {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_native(py, self)
    }
}

impl IntoPy for RelayEvent {
    /// The event as a tuple naming it first: `("open", connection,
    /// figure)`, `("message", connection, text)`, `("close", connection)`
    /// and `("download", request, figure, format)`.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::Opened { connection, figure } => ("open", connection, figure).into_py(py),
            Self::Message { connection, text } => ("message", connection, text).into_py(py),
            Self::Closed { connection } => ("close", connection).into_py(py),
            Self::Download {
                request,
                figure,
                format,
            } => ("download", request, figure, format).into_py(py),
        }
    }
}

/// Starts the server on `address` and `port` (0: any free port), trying
/// `port_retries` other ports nearby while they are in use (matplotlib's
/// `random_ports`), serving `script` as the client script; `None` when
/// every port tried is in use.
fn bind(
    address: String,
    port: u16,
    port_retries: u32,
    script: String,
) -> PyResult<Option<WebRelay>> {
    let options = WebOptions {
        port_retries,
        ..WebOptions::new(address, port)
    };
    match WebRelay::bind(&options, script) {
        Ok(relay) => Ok(Some(relay)),
        Err(WebError::NoFreePort { .. }) => Ok(None),
        Err(err) => Err(PyErr::new(ExceptionKind::OSError, err.to_string())),
    }
}

/// The port the server listens on.
fn port(relay: Native<WebRelay>) -> PyResult<u16> {
    Ok(relay.borrow().local_addr().port())
}

/// Serves pages for the figures `figures`: `(number, dark)` pairs, `dark`
/// for a figure drawn on a dark background.
fn set_figures(relay: Native<WebRelay>, figures: Vec<(u32, bool)>) -> PyResult<()> {
    relay.borrow().set_figures(&figures);
    Ok(())
}

/// What clients sent within `timeout` seconds (forever with `None`), as
/// event tuples.
fn poll(relay: Native<WebRelay>, timeout: Option<f64>) -> PyResult<Vec<RelayEvent>> {
    let timeout = timeout
        .map(|seconds| {
            Duration::try_from_secs_f64(seconds)
                .map_err(|_| PyErr::value_error(format!("{seconds} is not a duration")))
        })
        .transpose()?;
    Ok(relay.borrow_mut().poll(timeout))
}

/// Sends the text message `text` to the client of `connection`; whether
/// it is connected.
fn send_text(relay: Native<WebRelay>, connection: u64, text: String) -> PyResult<bool> {
    Ok(relay.borrow_mut().send_text(connection, &text))
}

/// Sends the binary message `data` to the client of `connection`;
/// whether it is connected.
fn send_binary(relay: Native<WebRelay>, connection: u64, data: Bytes) -> PyResult<bool> {
    Ok(relay.borrow_mut().send_binary(connection, &data.0))
}

/// Closes the WebSocket `connection`.
fn close(relay: Native<WebRelay>, connection: u64) -> PyResult<()> {
    relay.borrow_mut().close(connection);
    Ok(())
}

/// Answers the download `request` with the bytes of the file, or with
/// "not found" (`None`).
fn answer_download(relay: Native<WebRelay>, request: u64, file: Option<Bytes>) -> PyResult<()> {
    relay
        .borrow_mut()
        .answer_download(request, file.map(|file| file.0));
    Ok(())
}

/// The files of matplotlib's browser client: `(path, bytes)` with paths
/// in its `web_backend` directory (`"js/mpl.js"`).
fn client_files() -> PyResult<Vec<(&'static str, Bytes)>> {
    Ok(pltrs::web::client_files()
        .map(|(path, bytes)| (path, Bytes(bytes.to_vec())))
        .collect())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("bind", bind)?;
    module.add("port", port)?;
    module.add("set_figures", set_figures)?;
    module.add("poll", poll)?;
    module.add("send_text", send_text)?;
    module.add("send_binary", send_binary)?;
    module.add("close", close)?;
    module.add("answer_download", answer_download)?;
    module.add("client_files", client_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_port_in_use_without_retries_binds_nothing() {
        let relay = bind("127.0.0.1".into(), 0, 0, String::new())
            .unwrap()
            .expect("a free port");
        let taken = relay.local_addr().port();
        assert!(
            bind("127.0.0.1".into(), taken, 0, String::new())
                .unwrap()
                .is_none()
        );
        assert!(bind("not an address".into(), 80, 0, String::new()).is_err());
    }

    #[test]
    fn the_client_files_include_the_notebooks_script() {
        let files = client_files().unwrap();
        assert!(files.iter().any(|(path, _)| *path == "js/nbagg_mpl.js"));
    }
}
