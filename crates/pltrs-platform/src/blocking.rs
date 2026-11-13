//! The event loop's blocking waits, and the hook an embedder runs them
//! through ([`EventLoop::set_wait_hook`](crate::EventLoop::set_wait_hook)).
//!
//! A backend waits for input in one call (a read of the display server's
//! socket, `MsgWaitForMultipleObjectsEx`, `nextEventMatchingMask:…`) and
//! handles what arrived after it, so the loop can be idle without waking:
//! it waits until input comes, a timer is due or a
//! [`EventLoopProxy`](crate::EventLoopProxy) wakes it. Only that call runs
//! through the hook. It touches nothing but the backend's own connection
//! and runs none of the embedder's code (no handler, no decoration
//! painter), so an embedder may let its other threads work meanwhile:
//! pltrs (Python) releases the GIL there.

/// Runs one blocking wait of the event loop, the function it is given:
/// once, on the calling thread.
pub type WaitHook = Box<RunWait>;

/// The function of a [`WaitHook`].
type RunWait = dyn Fn(&mut dyn FnMut());

/// How a backend runs its blocking wait: through the embedder's
/// [`WaitHook`], if it gave one.
#[derive(Clone, Copy)]
pub(crate) struct Blocking<'a>(Option<&'a RunWait>);

impl<'a> Blocking<'a> {
    /// Waits run as they are: no hook (the X11 and Wayland connections'
    /// waits for replies, and tests).
    #[cfg(any(unix, test))]
    pub(crate) const DIRECT: Blocking<'static> = Blocking(None);

    /// Waits run through `hook`, if any.
    pub(crate) fn through(hook: Option<&'a WaitHook>) -> Self {
        Self(hook.map(|hook| &**hook))
    }

    /// Runs `wait` through the hook and returns its result; runs it here
    /// when a hook did not.
    pub(crate) fn run<R>(self, wait: impl FnOnce() -> R) -> R {
        let Some(hook) = self.0 else {
            return wait();
        };
        let mut wait = Some(wait);
        let mut result = None;
        hook(&mut || {
            if let Some(wait) = wait.take() {
                result = Some(wait());
            }
        });
        match result {
            Some(result) => result,
            None => (wait
                .take()
                .expect("a wait not run by the hook is still here"))(),
        }
    }
}

impl std::fmt::Debug for Blocking<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Blocking")
            .field(&if self.0.is_some() { "hooked" } else { "direct" })
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn direct_waits_run_as_they_are() {
        assert_eq!(Blocking::DIRECT.run(|| 7), 7);
    }

    #[test]
    fn hooked_waits_run_inside_the_hook_once() {
        let depth = Rc::new(Cell::new(0));
        let inside = Rc::clone(&depth);
        let hook: WaitHook = Box::new(move |wait| {
            inside.set(inside.get() + 1);
            wait();
            wait();
            inside.set(inside.get() - 1);
        });
        let runs = Cell::new(0);
        let seen = Blocking::through(Some(&hook)).run(|| {
            runs.set(runs.get() + 1);
            depth.get()
        });
        assert_eq!((seen, runs.get(), depth.get()), (1, 1, 0));
    }

    #[test]
    fn a_hook_that_skips_the_wait_still_has_it_run() {
        let hook: WaitHook = Box::new(|_wait| {});
        assert_eq!(Blocking::through(Some(&hook)).run(|| "ran"), "ran");
    }
}
