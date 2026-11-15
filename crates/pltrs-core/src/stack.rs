//! Room on the thread's stack for work that goes deep: work whose depth
//! depends on its input (a formula nesting fractions in fractions) or that
//! runs at the bottom of a long chain of calls (a text's first shaping, at
//! the end of a figure's layout). Where less of the stack is left than
//! such work needs, [`with_room`] runs it on a thread of its own, with a
//! stack of the size it needs, while the caller waits.
//!
//! Only code that knows the bounds of a thread's stack can tell how much
//! is left: pltrs (Python) tells each thread's ([`set_end`]) when it first
//! calls into pltrs on it, since Python's threads may have small stacks
//! (musl's 128 KiB, a size the program set). On a thread whose end is not
//! known, the work runs where it is called, as any other code does.

use std::cell::Cell;
use std::sync::{Mutex, PoisonError};

thread_local! {
    /// The lowest address the calling thread's stack may reach, if known.
    static END: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Tells where the calling thread's stack ends (its lowest address), or
/// that it is not known (`None`).
pub fn set_end(end: Option<usize>) {
    END.set(end);
}

/// How much of the calling thread's stack is left below the caller, if
/// its end is known ([`set_end`]).
#[must_use]
pub fn left() -> Option<usize> {
    let marker = 0_u8;
    let here = std::ptr::from_ref(&marker).addr();
    // An address below the end is on a stack other than the thread's own.
    END.get().and_then(|end| here.checked_sub(end))
}

/// Runs `work`, which needs `need` bytes of stack: here when that much is
/// left (or the stack's end is not known), else on a thread of its own
/// with a stack of `need` bytes, waiting for it. A panic of `work` goes on
/// unwinding from here, as it would have.
///
/// `work` runs on another thread only where the stack is short, so it must
/// not depend on the thread it runs on: it must not read or leave values
/// in thread-locals the caller reads, nor need a lock the caller holds.
pub fn with_room<R: Send>(need: usize, work: impl FnOnce() -> R + Send) -> R {
    if left().is_none_or(|left| left >= need) {
        return work();
    }
    // Kept where the caller can take it back if no thread can be started.
    let slot = Mutex::new(Some(work));
    let take = || {
        slot.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .expect("the work runs once")
    };
    std::thread::scope(|scope| {
        let started = std::thread::Builder::new()
            .name("pltrs-deep".to_owned())
            .stack_size(need)
            .spawn_scoped(scope, || take()());
        match started {
            Ok(thread) => thread
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
            // Without a thread (the process may start no more), the work
            // runs here, as it did before there were threads for it.
            Err(_) => take()(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name of the thread `with_room(need, ..)` runs its work on, with
    /// the calling thread's stack ending `left` bytes below the caller.
    fn runs_on(left: usize, need: usize) -> Option<String> {
        let marker = 0_u8;
        set_end(Some(std::ptr::from_ref(&marker).addr() - left));
        let name = with_room(need, || std::thread::current().name().map(str::to_owned));
        set_end(None);
        name
    }

    #[test]
    fn work_runs_here_while_the_stack_has_room() {
        let here = std::thread::current().name().map(str::to_owned);
        assert_eq!(runs_on(1 << 20, 64 << 10), here);
        assert_eq!(left(), None, "an unknown end");
        assert_eq!(with_room(usize::MAX, || 7), 7, "runs here without an end");
    }

    #[test]
    fn work_runs_on_a_stack_of_its_own_where_the_stack_is_short() {
        assert_eq!(runs_on(16 << 10, 64 << 10).as_deref(), Some("pltrs-deep"));
    }

    #[test]
    fn the_work_gets_the_stack_it_needs() {
        /// Recurses `depth` times with a kilobyte per frame.
        fn deep(depth: usize) -> usize {
            let frame = std::hint::black_box([1_u8; 1024]);
            if depth == 0 {
                return usize::from(frame[0]);
            }
            deep(depth - 1) + usize::from(frame[1])
        }
        let marker = 0_u8;
        set_end(Some(std::ptr::from_ref(&marker).addr() - (8 << 10)));
        // About 600 KiB, on what is left of a stack of 8 KiB.
        assert_eq!(with_room(4 << 20, || deep(500)), 501);
        set_end(None);
    }

    #[test]
    fn a_panic_of_the_work_unwinds_from_the_caller() {
        let marker = 0_u8;
        set_end(Some(std::ptr::from_ref(&marker).addr() - (16 << 10)));
        let caught = std::panic::catch_unwind(|| {
            with_room(64 << 10, || std::panic::panic_any::<u32>(42));
        });
        set_end(None);
        let payload = caught.expect_err("the panic reaches the caller");
        assert_eq!(payload.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    fn a_stack_other_than_the_threads_has_no_room_known() {
        set_end(Some(usize::MAX));
        assert_eq!(left(), None);
        set_end(None);
    }
}
