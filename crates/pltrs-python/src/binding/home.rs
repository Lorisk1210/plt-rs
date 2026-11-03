//! Calls from other threads on values tied to the main thread: run there
//! while it runs an event loop.
//!
//! Every thread holding the GIL may use pltrs objects ([`threads`]), but
//! for those of values tied to the thread that made them
//! ([`wrap_thread_bound`]): pltrs (Rust)'s windows, which the main thread
//! makes and runs (on macOS no other thread may). A worker that draws a
//! figure shown in one of them still asks its window to show the new
//! pixels, as matplotlib's GUI toolkits let any thread ask a widget to
//! repaint. So the main thread (the thread that imported pltrs) serves
//! other threads while it runs the windows' event loop
//! (`backends/_pltrs_loop.py`): a native function called on another
//! thread with an argument tied to the main thread is then queued to it,
//! the calling thread waits without the GIL, and the main thread runs the
//! call at its next step and hands back the result or the exception. When
//! the main thread stops serving it runs what is still queued, so no
//! caller is left waiting. A call made while the main thread does not
//! serve waits up to a second for it to start (as tkinter waits for its
//! main loop), since the main thread may be about to; past that (it may
//! be blocked waiting for the caller), the call runs where it is made,
//! where the value refuses it.
//!
//! [`threads`]: super::threads
//! [`wrap_thread_bound`]: super::native::wrap_thread_bound

use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::thread::ThreadId;
use std::time::Duration;

use super::error::PyResult;
use super::gil::{GilScope, Python, entry, without_gil};
use super::module::Module;
use super::native::owner_of;
use super::object::{Bound, Owned};
use crate::ffi;

/// Python's main thread, if the extension was imported on it.
static MAIN: OnceLock<ThreadId> = OnceLock::new();

/// The main thread's service: how many event loops it runs, one inside
/// the other, and the calls queued meanwhile.
static SERVICE: Mutex<Service> = Mutex::new(Service {
    loops: 0,
    queue: Vec::new(),
});

/// See [`SERVICE`].
struct Service {
    loops: usize,
    queue: Vec<Job>,
}

/// Signalled when the main thread starts serving.
static SERVING: Condvar = Condvar::new();

/// How long a call waits for the main thread to start serving.
const WAIT_FOR_SERVICE: Duration = Duration::from_secs(1);

/// What a native function call runs: the native function's capsule and
/// its arguments, as `dispatch` receives them.
pub(super) type NativeCall =
    unsafe fn(*mut ffi::PyObject, &[*mut ffi::PyObject]) -> *mut ffi::PyObject;

/// A call queued to the main thread.
struct Job {
    call: NativeCall,
    capsule: Owned,
    args: Vec<Owned>,
    reply: Arc<Reply>,
}

/// Where the main thread puts a queued call's outcome.
#[derive(Default)]
struct Reply {
    outcome: Mutex<Option<PyResult<Owned>>>,
    ready: Condvar,
}

impl Reply {
    /// Hands the outcome to the waiting thread.
    fn finish(&self, outcome: PyResult<Owned>) {
        *self.outcome.lock().unwrap_or_else(PoisonError::into_inner) = Some(outcome);
        self.ready.notify_all();
    }

    /// Waits for the outcome.
    fn wait(&self) -> PyResult<Owned> {
        let outcome = self.outcome.lock().unwrap_or_else(PoisonError::into_inner);
        let mut outcome = self
            .ready
            .wait_while(outcome, |outcome| outcome.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        outcome
            .take()
            .expect("the outcome is there once the wait ends")
    }
}

/// Records the main thread, when the extension is imported on it.
pub(super) fn init(py: Python<'_>) -> PyResult<()> {
    // SAFETY: the GIL is held; a new reference or NULL with an exception.
    let threading =
        unsafe { Bound::from_owned_ptr(py, ffi::PyImport_ImportModule(c"threading".as_ptr())) }?;
    let current = threading.call_method("current_thread", &[])?;
    let main = threading.call_method("main_thread", &[])?;
    if current.as_ptr() == main.as_ptr() {
        let _ = MAIN.set(std::thread::current().id());
    }
    Ok(())
}

/// Whether this is the main thread.
fn on_main() -> bool {
    MAIN.get() == Some(&std::thread::current().id())
}

/// Whether a native call with `args` made on this thread may have to run
/// on the main thread: this is another thread and an argument is tied to
/// the main thread.
///
/// # Safety
///
/// The GIL is held and `args` are live Python objects.
pub(super) unsafe fn belongs_to_main(args: &[*mut ffi::PyObject]) -> bool {
    let Some(&main) = MAIN.get() else {
        return false;
    };
    // The arguments first: few values are tied to their thread, and
    // reading them costs less than finding which thread this is.
    args.iter()
        // SAFETY: (caller) live objects, with the GIL held.
        .any(|&arg| unsafe { owner_of(arg) } == Some(main))
        && std::thread::current().id() != main
}

/// Runs `call(capsule, args)` on the main thread once it serves other
/// threads (waiting up to [`WAIT_FOR_SERVICE`] for it to start), waiting
/// for the result without the GIL: a new reference, or NULL with the
/// exception raised; `None` when the main thread does not serve.
///
/// # Safety
///
/// CPython called the native function on this thread, so it holds the
/// GIL; `capsule` and `args` are live.
pub(super) unsafe fn call_on_main(
    call: NativeCall,
    capsule: *mut ffi::PyObject,
    args: &[*mut ffi::PyObject],
) -> Option<*mut ffi::PyObject> {
    let reply = Arc::new(Reply::default());
    let job = {
        // SAFETY: (caller) the GIL is held, until the scope is dropped
        // before waiting.
        let scope = unsafe { GilScope::enter() };
        let py = scope.python();
        // SAFETY: (caller) live objects, which become strong references
        // the queued call keeps.
        unsafe {
            Job {
                call,
                capsule: Bound::from_borrowed_ptr(py, capsule).unbind(),
                args: args
                    .iter()
                    .map(|&arg| Bound::from_borrowed_ptr(py, arg).unbind())
                    .collect(),
                reply: Arc::clone(&reply),
            }
        }
    };
    let queue_and_wait = || {
        let service = SERVICE.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut service, _) = SERVING
            .wait_timeout_while(service, WAIT_FOR_SERVICE, |service| service.loops == 0)
            .unwrap_or_else(PoisonError::into_inner);
        if service.loops == 0 {
            // Dropped where the GIL is not held, the job's references
            // wait for the next entry from Python.
            return None;
        }
        service.queue.push(job);
        drop(service);
        Some(reply.wait())
    };
    // SAFETY: (caller) the GIL is held; the wait touches no Python object
    // and no pltrs (Rust) value, only the queue and the reply.
    let outcome = unsafe { without_gil(queue_and_wait) }?;
    // SAFETY: (caller) the GIL is held again; the outcome becomes the
    // native call's result.
    Some(unsafe { entry(|py| outcome.map(|object| object.bind(py))) })
}

/// Runs the queued calls, on the main thread with the GIL held, and hands
/// each its outcome.
fn run(py: Python<'_>, jobs: Vec<Job>) {
    for job in jobs {
        let args: Vec<*mut ffi::PyObject> = job.args.iter().map(Owned::as_ptr).collect();
        // SAFETY: the GIL is held (`py`); the job keeps the capsule and the
        // arguments alive, and the call is the one `dispatch` would have
        // made on this thread; it returns a new reference, or NULL with an
        // exception set, which `from_owned_ptr` takes.
        let outcome = unsafe { Bound::from_owned_ptr(py, (job.call)(job.capsule.as_ptr(), &args)) };
        job.reply.finish(outcome.map(Bound::unbind));
    }
}

/// The calls queued so far, taken off the queue.
fn take_queue(service: &mut Service) -> Vec<Job> {
    std::mem::take(&mut service.queue)
}

/// `_native.serve_threads(serving)`: the main thread starts (`True`) or
/// stops serving other threads, around an event loop; stopping runs what
/// is still queued. Nothing happens on other threads, which nothing is
/// queued to.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers"
)]
fn serve_threads(serving: bool) -> PyResult<()> {
    if !on_main() {
        return Ok(());
    }
    let left = {
        let mut service = SERVICE.lock().unwrap_or_else(PoisonError::into_inner);
        if serving {
            service.loops += 1;
            SERVING.notify_all();
            Vec::new()
        } else {
            service.loops = service.loops.saturating_sub(1);
            if service.loops == 0 {
                take_queue(&mut service)
            } else {
                Vec::new()
            }
        }
    };
    Python::with_gil(|py| run(py, left));
    Ok(())
}

/// `_native.run_queued_calls()`: the main thread runs the calls other
/// threads queued, a step of its event loop (nothing on other threads).
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers"
)]
fn run_queued_calls() -> PyResult<()> {
    if on_main() {
        let jobs = take_queue(&mut SERVICE.lock().unwrap_or_else(PoisonError::into_inner));
        Python::with_gil(|py| run(py, jobs));
    }
    Ok(())
}

/// Adds `serve_threads` and `run_queued_calls` to the extension module.
pub(crate) fn add_to(module: &Module<'_>) -> PyResult<()> {
    module.add("serve_threads", serve_threads)?;
    module.add("run_queued_calls", run_queued_calls)
}
