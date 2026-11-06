//! Live tests of connecting: a program opening connections in a row,
//! as matplotlib's display checks do, finds the server every time,
//! though the server resets whenever its last client leaves and drops
//! the connections it accepted meanwhile.

use std::sync::PoisonError;
use std::time::Duration;

use super::super::auth::Credentials;
use super::super::connection::Connection;
use super::super::transport::Stream;
use super::{ANCHOR, Live, connect_patiently, live};
use crate::backend::BackendKind;
use crate::error::Error;
use crate::event_loop::EventLoop;

/// Runs `f` with no connection of the test run open, so that the server
/// resets whenever the last connection `f` opens closes (unless another
/// program is connected to it).
fn without_anchor<R>(live: &Live, f: impl FnOnce() -> R) -> R {
    drop(ANCHOR.lock().unwrap_or_else(PoisonError::into_inner).take());
    let result = f();
    *ANCHOR.lock().unwrap_or_else(PoisonError::into_inner) = Some(connect_patiently(&live.name));
    result
}

/// A connection accepted while the server's only client leaves: the
/// stream `open` gives once the server had time to accept it, after
/// closing `only_client`, with the credentials it is opened with (a
/// desktop's server, such as Xwayland, wants them).
fn opened_while_resetting(
    live: &Live,
    only_client: Connection,
) -> Result<(Stream, Option<Credentials>), Error> {
    let opened = Stream::connect(&live.name)?;
    std::thread::sleep(Duration::from_millis(50));
    drop(only_client);
    Ok(opened)
}

#[test]
fn the_server_drops_connections_it_accepted_while_resetting() {
    let Some(live) = live() else { return };
    let handshake = without_anchor(&live, || {
        let only_client = Connection::connect(&live.name).unwrap();
        let (stream, credentials) = opened_while_resetting(&live, only_client).unwrap();
        Connection::handshake(stream, credentials.as_ref())
    });
    // Unless another program kept the server from resetting.
    if let Err(error) = handshake {
        assert!(
            matches!(error, Error::ConnectionClosed | Error::Io { .. }),
            "{error:?}"
        );
    }
}

#[test]
fn connections_dropped_by_a_reset_are_made_again() {
    let Some(live) = live() else { return };
    let connected = without_anchor(&live, || {
        let mut only_client = Some(Connection::connect(&live.name).unwrap());
        Connection::connect_through(|| match only_client.take() {
            Some(only_client) => opened_while_resetting(&live, only_client),
            None => Stream::connect(&live.name),
        })
    });
    assert!(connected.is_ok(), "{connected:?}");
}

/// Each connection closes, which resets the server when it was its only
/// client, before the next one is made after a pause: the pauses sweep
/// the time a server takes to notice a closed connection.
#[test]
fn connections_in_a_row_all_succeed() {
    let Some(live) = live() else { return };
    let failures: Vec<_> = without_anchor(&live, || {
        (0..300_u32)
            .filter_map(|attempt| {
                std::thread::sleep(Duration::from_micros(u64::from(attempt % 30) * 20));
                EventLoop::with_backend(BackendKind::X11)
                    .err()
                    .map(|error| (attempt, error))
            })
            .collect()
    });
    assert!(
        failures.is_empty(),
        "{} failed: {failures:?}",
        failures.len()
    );
}
