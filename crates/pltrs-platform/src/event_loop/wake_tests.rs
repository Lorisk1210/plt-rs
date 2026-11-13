//! Tests of a loop blocked without a deadline: what wakes it, for every
//! backend (the headless one here, the others in their live tests).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::EventLoop;
use crate::event::Event;
use crate::wake_pipe::WakePipe;

/// Longer than any wake-up takes: a test failing waits this long, not
/// forever.
const GUARD: Duration = Duration::from_secs(10);

/// Pumps `event_loop` with no deadline but [`GUARD`], its waits through a
/// hook that tells when it blocks, and has `wake` run on another thread
/// once it does (and a little later, so that it is well inside the
/// wait): the pump must return with [`Event::WakeUp`] soon after, having
/// waited through the hook.
pub(crate) fn assert_woken_while_blocked(
    event_loop: &mut EventLoop,
    wake: impl FnOnce() + Send + 'static,
) {
    let (blocked, blocks) = mpsc::channel();
    let waits = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&waits);
    event_loop.set_wait_hook(Some(Box::new(move |wait| {
        counted.fetch_add(1, Ordering::SeqCst);
        let _ = blocked.send(());
        wait();
    })));
    let waker = thread::spawn(move || {
        blocks.recv_timeout(GUARD).expect("the loop blocks");
        thread::sleep(Duration::from_millis(50));
        wake();
    });
    let start = Instant::now();
    let mut woken = false;
    while !woken && start.elapsed() < GUARD {
        event_loop
            .pump_events(Some(GUARD), |_, event| woken |= event == Event::WakeUp)
            .unwrap();
    }
    waker.join().unwrap();
    event_loop.set_wait_hook(None);
    let took = start.elapsed();
    assert!(woken, "not woken within {GUARD:?}");
    assert!(took < GUARD / 2, "woken after {took:?}");
    assert!(
        waits.load(Ordering::SeqCst) >= 1,
        "the wait ran through the hook"
    );
}

/// [`assert_woken_while_blocked`] for a write to a [`WakePipe`] waking
/// the loop through its proxy, as a signal does in pltrs (Python).
pub(crate) fn assert_woken_by_a_wake_pipe(event_loop: &mut EventLoop) {
    let proxy = event_loop.proxy();
    let pipe = Arc::new(
        WakePipe::new(move || {
            let _ = proxy.wake_up();
        })
        .unwrap(),
    );
    let writer = Arc::clone(&pipe);
    assert_woken_while_blocked(event_loop, move || writer.write(&[2]).unwrap());
    assert_eq!(pipe.take_received(), [2]);
}

#[test]
fn a_proxy_wakes_a_blocked_headless_loop() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let proxy = event_loop.proxy();
    assert_woken_while_blocked(&mut event_loop, move || proxy.wake_up().unwrap());
}

#[test]
fn a_wake_pipe_wakes_a_blocked_headless_loop() {
    let (mut event_loop, _handle) = EventLoop::headless();
    assert_woken_by_a_wake_pipe(&mut event_loop);
}

#[test]
fn waits_without_a_hook_run_as_before() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let start = Instant::now();
    event_loop
        .pump_events(Some(Duration::from_millis(30)), |_, _| {})
        .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(30));
}
