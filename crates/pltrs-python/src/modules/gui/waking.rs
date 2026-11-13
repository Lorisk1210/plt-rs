//! What wakes the windows' event loop, which blocks until something
//! happens (`backends/_pltrs_loop.py`): input and timers wake it by
//! themselves; a call another thread queues for it
//! ([`home`](crate::binding)), [`wake`] (an idle draw asked for on another
//! thread, a loop asked to stop) and signals ([`signal_fd`]) wake it
//! through the host's proxy. While it waits, it does without the GIL.

use std::sync::{Arc, Mutex, PoisonError};

use pltrs::gui::WindowHost;
use pltrs_platform::WakePipe;

use crate::binding::prelude::*;
use crate::binding::{Bytes, releasing_gil, set_loop_waker, wake_loop};

/// The pipe Python's signal handler writes to while the loop runs
/// (`signal.set_wakeup_fd`), made when first asked for; it wakes the loop
/// of whichever host is current.
static SIGNALS: Mutex<Option<Arc<WakePipe>>> = Mutex::new(None);

/// Has `host`'s waits release the GIL, and [`wake_loop`] (a queued call,
/// a signal) wake `host`: for every host made, the latest being the one
/// the program uses.
pub(super) fn install(host: &mut WindowHost) {
    host.set_wait_hook(Some(releasing_gil()));
    let proxy = host.proxy();
    set_loop_waker(Box::new(move || {
        // A host already dropped has no loop to wake.
        let _ = proxy.wake_up();
    }));
}

/// The pipe of [`SIGNALS`], made if need be.
fn signals() -> PyResult<Arc<WakePipe>> {
    let mut signals = SIGNALS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(pipe) = signals.as_ref() {
        return Ok(Arc::clone(pipe));
    }
    let pipe = Arc::new(
        WakePipe::new(wake_loop).map_err(|error| PyErr::runtime_error(error.to_string()))?,
    );
    *signals = Some(Arc::clone(&pipe));
    Ok(pipe)
}

/// `_gui.wake()`: wakes the windows' event loop, from any thread (not
/// queued to the loop's thread, as calls with a host are).
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers"
)]
fn wake() -> PyResult<()> {
    wake_loop();
    Ok(())
}

/// `_gui.signal_fd()`: the descriptor (a socket handle on Windows) for
/// `signal.set_wakeup_fd`, non-blocking, whose writes wake the loop.
fn signal_fd() -> PyResult<i64> {
    let pipe = signals()?;
    #[cfg(unix)]
    let fd = i64::from(pipe.writer_fd());
    #[cfg(windows)]
    let fd = i64::try_from(pipe.writer_socket())
        .map_err(|_| PyErr::runtime_error("the wake pipe's socket handle is out of range"))?;
    Ok(fd)
}

/// `_gui.take_signals()`: the bytes written to [`signal_fd`] since last
/// asked (the signals' numbers), taken.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers"
)]
fn take_signals() -> PyResult<Bytes> {
    let signals = SIGNALS.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(Bytes(
        signals
            .as_ref()
            .map_or_else(Vec::new, |pipe| pipe.take_received()),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("wake", wake)?;
    module.add("signal_fd", signal_fd)?;
    module.add("take_signals", take_signals)
}
