# pltrs: design

pltrs (Python) is pltrs with matplotlib's syntax: a Python package over
pltrs (Rust), the crate. Code written for matplotlib runs with one
change, the import:

```python
import pltrs.pyplot as plt          # import matplotlib.pyplot as plt
from pltrs import colors            # from matplotlib import colors
from pltrs.mpl_toolkits.mplot3d import Axes3D
```

The measure is matplotlib 3.11's own test suite, run unchanged against
pltrs: the share of the tests that pass under real matplotlib here which
also pass under pltrs (`cargo xtask mpl-suite`). It is at 100%, and must
stay there.

This document is for the agents who widen pltrs. It records the decisions
that bind them, and where to put new code.

## Layout

```
crates/pltrs-python/            the native extension (a cdylib, no crates)
  build.rs                      finds a libpython for the unit tests only
  src/lib.rs                    PyInit__native
  src/ffi/                      the CPython C API, declarations only
  src/binding/                  the safe binding core (all other unsafe code)
    gil.rs                      Python<'py> token, GIL count, entry points
    object.rs                   Bound<'py>, Owned, calls, attributes
    error.rs                    PyErr, PyResult, ExceptionKind, warnings
    convert.rs                  FromPy / IntoPy for Python's own types
    containers.rs               tuples, lists, dicts
    buffer.rs                   NdArray: numbers through the buffer protocol
    native.rs                   the native object; native_value!, artist_handle!
    function.rs                 native functions from plain Rust functions
    module.rs                   the extension module and its submodules
    callback.rs                 Callback: Python called from pltrs (Rust)
    reentrant.rs                changes made while their figure is in use, applied after
    deferred.rs                 work Python leaves to the next native call (line data)
    own_state.rs                artists whose objects are told before a native call reaches them
    after_call.rs               work left to the end of each native call (queued warnings)
    registry.rs                 Python objects registered for native code
    threads.rs                  every thread holding the GIL uses every object
    home.rs                     other threads' calls on windows, run by the main thread
    blocking.rs                 the windows' event loop waiting without the GIL
    stack.rs, stack/            the stack guard: RecursionError before a thread's stack ends
    fault.rs                    the address of a fatal fault, on stderr (PLTRS_REPORT_FAULTS)
    watched.rs                  package attributes a program may replace, read by native code
    allocator.rs                the allocator on Linux (huge pages for large blocks)
    testing.rs                  the embedded interpreter of the unit tests
    init.rs                     the module definition
  src/modules/                  one file per matplotlib module
    mod.rs                      registration list (shared, see below)
    archive.rs                  archives of figures and values (pickling)
    artist.rs axes.rs colors.rs figure.rs lines.rs rcsetup.rs ticker.rs ...
python/
  pltrs/                        the package; mirrors matplotlib's module tree
    __init__.py                 rcParams, RcParams, rc, rc_context, rcdefaults, use
    _native.abi3.so             built by `cargo xtask python` (gitignored)
    _api/                       matplotlib's _api, kept as it is
    _handles.py                 one Python object per pltrs (Rust) artist
    _pickling.py                how figures and native objects pickle
    _pylab_helpers.py           pyplot's figure registry (Gcf)
    artist.py figure.py axes/ axis.py lines.py text.py patches.py
    collections.py container.py legend.py colors.py ticker.py units.py
    backend_bases.py cbook.py ft2font.py pyplot.py pylab.py style/
    backends/                   the backends: registry, canvases, GUI toolkits,
                                pltrs' own windows (backend_pltrs, _pltrs_loop)
    testing/                    setup, decorators, compare, conftest, exceptions
    mpl_toolkits/               the toolkits (imported as pltrs.mpl_toolkits.*)
  mpl_suite/                    the suite runner's Python side (not in pltrs)
    mpl_suite_report.py         pytest plugin recording outcomes
    pltrs_alias.py              sitecustomize: `pltrs` aliases what loads as `matplotlib`
    import_gaps.py              `--imports`: every missing import of each test file
  tests/                        `cargo xtask python test`: the smoke tests (`test_*.py`),
                                the fuzzer (`fuzz/`) and the GUI comparison (`gui_events/`)
  backend/pltrs_backend/        the PEP 517 build backend (standard library only)
  alias/                        the `matplotlib` distribution: matplotlib's names for pltrs
                                (and matplotlib's type stubs, `alias/matplotlib/*.pyi`)
  ecosystem/                    `cargo xtask ecosystem`: the libraries importing matplotlib
                                on pltrs (pandas, seaborn, xarray, networkx, scikit-learn,
                                statsmodels, geopandas, plotnine, astropy, cartopy, nilearn,
                                shap, ...), `fetch.py`, which downloads their sdists, and
                                `plugin/`, the suites' pytest plugins
  tools/toolkits.sh             GTK 3 and 4, PyGObject, pycairo, cairocffi, wxPython and
                                Qt's X libraries without root
  requirements-pltrs.txt        the pltrs venv (pinned)
  requirements-reference.txt    what the reference venv gets on top
  requirements-suite.txt        what both get for the suite (pandas, tornado, Qt bindings)
  requirements-ecosystem.txt    the ecosystem venvs (the libraries, what their tests use, pytest)
  requirements-ecosystem-reference.txt  what the ecosystem's reference venv adds (matplotlib)
  requirements-ecosystem-<library>.txt  a library's own ecosystem venvs, where its pins clash
  requirements-notebooks.txt    the notebooks' venvs (Jupyter, IPython, matplotlib-inline, ipympl)
  requirements-notebooks-reference.txt  what the notebooks' reference venv adds (matplotlib)
pyproject.toml                  `pip install .`: the backend and the distributions
spec/mpl-suite/
  reference.json                outcomes under real matplotlib (committed)
  pltrs.json                    outcomes under pltrs (committed)
spec/ecosystem/
  reference.json, pltrs.json    the same for the libraries' suites (`cargo xtask ecosystem suite`)
spec/notebooks/
  *.ipynb, execute.py           `cargo xtask notebooks`: notebooks run in Jupyter, and their driver
  reference.json, pltrs.json    the outcomes of their comparison and of the libraries' tests
```

## The native extension

### The C API: abi3, limited API 3.12

There is no PyO3 (no crates at all): `crates/pltrs-python/src/ffi` declares
by hand the part of CPython's limited API the crate uses. The extension is
built once and loads in Python 3.12 and every later CPython
(`_native.abi3.so`). The floor is 3.12, not 3.11, because the limited API
gained there what a clean binding needs: `PyObject_Vectorcall`,
`PyErr_GetRaisedException`/`PyErr_SetRaisedException`, and type data
reached through `PyObject_GetTypeData` with a negative `basicsize`, so
that no CPython struct size is assumed. The only layouts used are those
the limited API exposes: the object header (`ob_type`), `Py_buffer`, and
the structs describing types, methods and modules. Reference counts go
through `Py_IncRef`/`Py_DecRef`. Whether every imported symbol is in the
3.12 limited API can be checked by compiling references to them against
CPython's headers with `Py_LIMITED_API=0x030c0000` (gcc exists here).

The extension links no libpython: the interpreter importing it provides
the symbols. The Rust unit tests embed an interpreter instead: `build.rs`
finds a shared libpython 3.14+ (`PLTRS_PYTHON`, else `.venv-pltrs`, else
`.venv`, else `python3`) and links it only into the test binary, which
starts it with the PEP 741 `PyInitConfig` API. Without one the crate's test
build is empty (with a cargo warning), so `cargo test --workspace` still
works on a machine without Python.

On x86_64 Linux with glibc (the manylinux x86_64 wheel, and builds from
the sdist there) the linker lays the extension's functions out in the order
of `crates/pltrs-python/symbol-order.txt`: the code every program runs
first, so that a process maps about 5 MiB of the library's code rather
than 11 (see `build.rs`). Musl, aarch64, macOS and Windows builds keep
the compiler's layout: the order's names are those of the host's build.
`crates/pltrs-python/tools/symbol_order.py`
writes the file from what the benchmark scripts run; a function it does
not name (new code, or code whose name changed with the code around it)
is laid out after the others, among code no program runs. One change of
native code can cost a small figure half a MiB that way, eighty commits
cost it 3 MiB: the file is written again before a release and an
official benchmark (`RELEASING.md`), and `python/tests/test_startup_work.py`
fails when the code a small program holds has grown by a quarter.

### `unsafe`

The crate lifts the workspace's ban on `unsafe`, as only it and
`pltrs-platform` may. `ffi` holds the declarations; `binding` the wrappers,
with a `// SAFETY:` comment on every block (`clippy::undocumented_unsafe_blocks`
is on). `modules` and everything above them are safe Rust: they never see
a raw pointer.

### The binding core (`binding`)

* **`Python<'py>`**: a zero-sized, non-`Send` token proving that the
  thread holds the GIL. Every entry from Python (`gil::entry`: native
  functions, the native type's slots, module init) opens a `GilScope`,
  which counts the thread as holding the GIL and hands out the token.
  Rust code running on its own takes it with `Python::with_gil`
  (`PyGILState_Ensure`, skipped when the count is already positive).
* **`Bound<'py>`** is a strong reference usable while the GIL is held;
  **`Owned`** is the strong reference Rust state keeps (a callback in a
  pltrs (Rust) closure): `Send + Sync`, used through `bind(py)`. An `Owned`
  dropped where the thread is not known to hold the GIL is not released
  there; it waits in a pool released at the next entry from Python.
* **`PyErr` / `PyResult<T>`**: a native function returns `PyResult<T>`;
  `Err` is raised when it returns. `ExceptionKind` names the built-in
  classes Rust raises. A Rust panic never unwinds into C: `entry` turns it
  into `RuntimeError("pltrs internal error (Rust panic): ...")`.
* **`FromPy` / `IntoPy`**: conversions with Python's own rules and
  messages. `bool` is any object's truth value; integers take
  `__index__`; floats take what `float()` takes without parsing; `String`
  only `str`; `Vec<T>` any iterable but a string; tuples iterables of
  their length; `BTreeMap<String, V>` anything with `items()`; `PathBuf`
  `os.PathLike`; `Bytes` `bytes`. `Vec<T>` returns a `list`.
* **`NdArray<T>`** (`T`: `f64`, `i64`, `u8`, `bool`) is how numbers cross
  in bulk: read through the buffer protocol from any exporter (numpy
  arrays of any numeric dtype, byte order and strides, `array.array`,
  `memoryview`), or from nested sequences; returned as a numpy array that
  owns a copy (`numpy.empty` of the shape, filled through its buffer).
  `FloatArray` reads an argument as `numpy.asarray(arg, float)` does
  (numpy arrays and flat tuples or lists of numbers directly, anything
  else through numpy). No numpy C API.
* **Deferred work** (`binding::install_deferred`): a dict of work Python
  leaves to the next native call, handed (taken out of the dict at once)
  to the function installed with it before any native call runs.
  `Line2D.set_data` converts the data at once and leaves the numbers
  there, as matplotlib leaves them to the next draw.
* **The native object** (`_native.Native`) holds any Rust value for
  Python. Python cannot make or subclass it. See below.
* **Native functions** are plain Rust functions or closures, registered
  with `Module::add(name, function)`; see "Adding a function".
* **`Callback`**: a Python callable pltrs (Rust) calls. See "Callbacks".
* **The registry** (`_native.register(name, object)`, `binding::registered_error`):
  Python objects the package defines that native code needs, such as an
  exception class to raise (`ImageComparisonFailure`). The native module
  never imports the package by name, so it works under any name (see the
  suite's aliasing).

### The native object

A native object holds one `'static` Rust value in a `RefCell`, for
artist handles the artist's `RawHandle`, and for the few values tied to
their thread the thread that made them (see "Threads").

* `artist_handle!(Line2D)` (in the module file that owns the type) lets
  native functions take and return `Line2D` directly. Its native object
  also serves the functions common to all artists, which take a
  `RawHandle` (`_native.artist`: visibility, z-order, alpha, label,
  `remove`, and `id`, the artist's id in its figure). The other way
  round, a function taking a `Line2D` accepts the native object of any
  handle of a line (the untyped `RawHandle` of a list of children or of
  an adoption, say) and raises `TypeError` for an artist of another
  kind: every type given to `artist_handle!` implements pltrs (Rust)'s
  `TypedHandle`, which recovers it from an untyped handle.
* `native_value!(RcParams)` does the same for values that are not
  artists (a clone comes out). A function that must change the value in
  place takes `Native<T>` instead and uses `borrow_mut()`.

### Threads

As in matplotlib, any Python thread holding the GIL may use any pltrs
object: build a figure in a worker, draw it in a thread pool, save it
from a web server's request thread, or keep using it after the thread
that made it ended. matplotlib documents that it is not made for
concurrent use; neither is pltrs, but nothing ties an object to the
thread that made it. The model is in `binding/threads.rs`; in short:

* **pltrs (Rust) is single-threaded code**: handles share their figure
  through `Rc`s and `RefCell`s, so they are not `Send`. What makes them
  sound to share is the GIL: native objects' values and everything they
  reach are only touched by native calls and the native type's slots,
  which hold it, and the GIL's release and acquisition order every touch.
  The binding never releases the GIL while pltrs (Rust) works (only
  waits release it, touching nothing: `home`'s wait for the main thread,
  and the windows' event loop's wait for something to happen, through
  `binding::blocking`), and
  Python code that releases it in the middle of a native call (a
  callback) runs no Rust code of that thread until it takes it back.
  abi3 builds exist only for CPython with a GIL.
* **The state matplotlib keeps in process-wide variables** (rcParams, the
  registries of colormaps, colour sequences and projections, the style
  library, the default legend handlers, the warning and log handlers, the
  program's text measurer and painters, the font map replacement, the
  watched transform trees) is per thread in pltrs (Rust), so that Rust
  programs and pltrs (Rust)'s parallel tests keep their own: each such
  variable is a `pltrs_core::thread_state!`, which a thread reaches in a
  `pltrs_core::state::State` instead of its own while it works with one.
  Every Python thread works with the interpreter's `State` while it holds
  a `GilScope` (a native call, a slot, `Python::with_gil`): rcParams are
  global, and `rc_context` in one thread is seen by the others, as in
  matplotlib. What stays per thread is what belongs to one call and its
  callbacks: the missing glyphs of a layout, the measuring modes of a
  backend's draw, the exception frames of callbacks, the renderer stacks
  of `_text_metrics`, `_python_drawing` and `_transform_trees`
  (`threading.local`).
* **Thread-local storage never holds what another thread can reach**: it
  is dropped without the GIL when its thread ends. The one per-thread
  value kept across native calls, the metrics cache `text._measuring_with`
  installs, is parked in the shared state when the thread leaves its
  outermost scope and taken back at the next.
* **Shared state is never borrowed across a call into Python**, since
  Python may switch threads there (in a callback, or in any Python code a
  garbage collection runs): what a change replaces (a handler, a
  registration, a Python function) is dropped after the borrow ends, and
  no lock is held while Python runs. A native call that does call back
  into Python keeps its figure borrowed meanwhile, so a thread's outermost
  native call on an artist whose figure another thread's call is using
  waits for it (without the GIL, up to five seconds) instead of finding it
  borrowed: a worker changing a figure while the main thread's draw runs
  a `FuncFormatter` changes it once the draw is done.
* **Values tied to their thread**: pltrs (Rust)'s windows (`_native.gui`'s
  host: macOS runs windows on its main thread only, Windows delivers their
  messages to the thread that made them) are made with
  `wrap_thread_bound`. Their objects refuse other threads
  (`RuntimeError: this pltrs WindowHost belongs to another thread`), and
  one deallocated on another thread leaks its value. Calls on them from
  other threads are queued to the main thread while it runs the windows'
  loop (`binding/home.rs`, `backends/_pltrs_loop.py` brackets its loop
  with `_native.serve_threads(True)` / `(False)` and runs
  `_native.run_queued_calls()` at each step), the caller waiting without
  the GIL; a call made while the main thread does not serve waits up to a
  second for it to start (as tkinter waits for its main loop), then runs
  where it is made and is refused, so a main thread blocked on the caller
  gets the error, not a deadlock. So a worker drawing a figure shown in a
  pltrs window has the window show it, as Qt lets any thread ask a widget
  to repaint.

The tests: `python/tests/test_threads.py` (workers, thread pools rendering
the bytes one thread renders, `rc_context` and warnings across threads, a
seeded run of threads using pltrs at once with Python switching as often
as it can), `test_backend_pltrs.py::test_interactive_thread_safety`, and
matplotlib's own `test_interactive_thread_safety` (Qt, Tk, GTK, wx), which
the suite runs with the toolkits and a display (see "Running the suite").

### The stack guard

A program that recurses through drawing (a formatter saving its own
figure, an artist's `draw` drawing its Axes) runs native frames between
CPython's own checks, so a thread's native stack could end before
CPython's limit does, and the process would die. `binding/stack.rs`
refuses a native call made inside another one (`function::call_nested`,
the one place every such round passes) with `RecursionError` once less
than its reserve, 64 KiB, is left of the thread's stack. The bounds of
each thread's stack are read once (Linux: `pthread_getattr_np`, the main
thread's limit from `/proc`; macOS: `pthread_get_stackaddr_np`; Windows:
`GetCurrentThreadStackLimits`); the check is one comparison.

The reserve is what the deepest native stretch below a nested call needs,
measured, not guessed: STACK1's probe (the stack pointer at every nested
call, the stack painted below the outermost call and scanned afterwards)
over saves in every format of figures with subfigures, constrained layout,
mathtext, legends and colorbars, recursing through formatters, locators
and artists' `draw`. A round deepens by at most 44 KiB between two
nested calls, and a draw goes on at most 47 KiB below its last nested
call in an optimised glibc build, 48 KiB in the musl wheel and 56 KiB in
the unoptimised development build. Big values held by value in a frame
cost every thread: the save paths of `Figure::to_format` are not inlined
into one frame, and the DEFLATE encoder keeps its trees on the heap, which
brought a draw's first callback from 108 KiB of stack to 42 KiB. With
that, a thread of musl's default 128 KiB (CPython 3.9 to 3.13 in the
`musllinux` images, uv's musl builds) draws with the program's callbacks
as matplotlib does there. What still does not fit is a draw inside a draw
on such a thread (a formatter saving its figure: about 90 KiB deep, its
nested calls with less than the reserve left); there the error starts
with Python's own "maximum recursion depth exceeded", as code matching on
it expects, and goes on to name the thread's stack size, what pltrs needs
and `threading.stack_size`. pltrs does not change `threading.stack_size`
itself: a library has no business setting it for the whole process, and
Alpine's own Python, Docker's `python:alpine` and the `musllinux` images'
CPython 3.14 already give threads 1 to 2 MiB.

The reserve must hold everything a call does below the check that let it
in, including what it goes on doing after a call it makes is refused (a
draw measures its tick labels after their formatter failed). Two kinds of
work at the bottom of a draw go deeper than any measurement of warm draws
shows, and the fuzzer met both: shaping a line of text the engine has not
shaped (the font chain made, faces parsed, a cold one read from its file:
12 KiB in the development build) and typesetting a formula, whose depth
grows with its nesting (about 7 KiB a level of fractions; pltrs' parser
takes 100 levels, matplotlib's 20 but not 30). Both are pure Rust, so
`pltrs_text::deep` runs them where the stack has room for them: on a
thread of its own, with a stack of the size they need, where less is left
(`pltrs_core::stack::with_room`; the binding tells pltrs (Rust) where each
thread's stack ends). A formula's need is counted from its text: a level
opens at `{`, `\`, `^` or `_`, and no opening costs more than 8 KiB. The
font lookups and mathtext messages the work notes there are handed back to
the caller's thread; it reads the font database without writing it and
takes no lock its callers hold. Without them below it, a nested draw goes
at most 47 KiB below the check that let it in (development build; 46 KiB
optimised: an SVG save under constrained layout drawing its tick labels),
61 KiB before the frames a draw runs below were made lean: a document's or
a canvas' renderer, the figure state a layout works on and the mixed-mode
renderer are on the heap, and the work beside the chain (the axis label
and offset text placed, the ticks drawn) has frames of its own.
`test_stack1.py` measures that stretch on saves in four formats, with the
deep work apart (`PLTRS_DEEP_WORK_APART`, which runs it on a thread of its
own whatever is left), and fails where it leaves less than 12 KiB of the
reserve of the system it runs on (`_native._stack_room()` tells where the
stack is and the reserve).

The calls native code makes into Python are checked too, against 48 KiB.
CPython 3.14 checks the stack before every Python call and refuses once
less than 32 KiB is left (twice `_PyOS_STACK_MARGIN_BYTES`), with its own
`RecursionError: Stack overflow (used 478 kB)`. A native call let in with
64 KiB left went some 40 KiB deeper before it called the program's
formatter, so without that check whether a recursion ended with
matplotlib's message or CPython's depended on where in a round the stack
ran out: on the build. With it, CPython's check is never the first to
refuse a round (the Python code a callback runs before it reaches pltrs
again takes about 4 KiB of the 16 left above CPython's margin). CPython
3.12 and 3.13 do not check the stack; there the same window was a
segfault. Tests: `python/tests/test_stack1.py` (every recursion on every
stack, formulas as deep as matplotlib takes them, small threads drawing,
the stretch below an admitted call).

## The Python package

### Where logic lives

pltrs (Rust) does the work: validation, layout, drawing, saving, colour and
tick computation. Python holds what is inherently Python: matplotlib's
signatures (positional forms, keywords, defaults, aliases), `set(**kw)`
dispatch, pyplot's registry of Python figure objects, the `_api`
decorators, `cbook` helpers that work on Python objects, the pytest
plumbing of `testing`. When a Python method grows logic beyond turning
its signature into native arguments, the logic belongs in Rust.

### The look

pltrs (Python) imported as `pltrs` draws in pltrs' modern look by
default, as pltrs (Rust) does: the `pltrs-modern` style sheet, Inter,
constrained layout (see `pltrs::look`). Imported as `matplotlib` (the
alias, or matplotlib's suite loading the package under that name), it
draws in matplotlib's look, in which the output is matplotlib's. The
name of the first import decides, once per process: the extension
loads once, as `pltrs._native` or as `matplotlib._native`, and
`modules::register` reads that name, puts matplotlib's appearance
under the environment's choices for `matplotlib._native`
(`pltrs::look::set_default`), and starts the parameters from the look
in force. The toolkits and `pylab` import `pltrs` by name, which the
alias then serves from the package already loaded, so they cannot
decide it. `PLTRS_LOOK`, `PLTRS_FIGURE_LOOK` and `PLTRS_CHROME_LOOK`
choose over the name, as in pltrs (Rust); code that needs a look reads
the look in force, never the variables. `rcParams` then
holds the look's parameters (`rcParamsOrig` keeps them), while
`rcParamsDefault`, `rcdefaults()` and `style.use("default")` are
matplotlib's defaults, as `pltrs::rc::reset` is. What compares with
matplotlib runs in matplotlib's look: `cargo xtask mpl-suite` sets
`PLTRS_LOOK=matplotlib` for both runs, and so do the benchmarks
(`xtask/src/bench`).

### How a class wraps a native handle

An artist object holds `self._native`, the native object of its pltrs (Rust)
artist, and `self._figure`, its (root) `Figure`. Methods call the native
functions of their module:

```python
class Line2D(Artist):
    def get_linewidth(self):
        return _lines.get_linewidth(self._native)
```

Native functions returning an artist return a new native object each
time, but matplotlib code expects the same Python object back
(`ax.lines[0] is line`, attributes kept, subclasses kept). So each root
`Figure` keeps `self._artists`, its registry `{artist id: object}`, and
handles become objects through `_handles.wrap(native, cls, figure)`:

```python
lines = _handles.wrap_all(_axes.get_lines(self._native), Line2D, self.figure)
```

`wrap` returns the object made before, or a new `cls` around the handle
(`cls._from_native`). The caller always knows the figure (the object the
method was called on) and the class (what the method returns). Every
artist object refers to its figure and the figure to its artists, so an
artist's object lives as long as its figure, and Python's garbage
collector sees the whole cycle (all references are Python's).

### Artists made before they are added

matplotlib code makes artists with their constructors and adds them
later: `line = Line2D(x, y)`, then `ax.add_line(line)` (or `add_patch`,
`add_collection`, `add_image`, `add_artist`, `fig.add_artist`). pltrs
(Rust) supports this for every artist that owns no other (lines, patches,
collections, texts, images, tables, custom artists): a *loose* artist
(`Line2D::loose`, `Patch::loose`, `Collection::loose`, `Text::loose`,
`Table::loose`, `Custom::loose`) lives in a holder of its own, a figure
state that is no figure, so that its handle works as any handle does and
its properties can be read and set before it is added. `Axes::adopt_artist(artist,
AddAs::Line)` (matplotlib's `add_line`; `AddAs` names the `add_*`
method, which decides the data limits and the `_child<n>` label) and
`Figure::adopt_artist(artist, clip)` move it into the figure under a new
id, give it the default transform (`transData`, `transSubfigure`) unless
it has one, and return the handle there; an artist removed from the same
figure comes back under its own id.

In Python, a constructor makes the native loose artist with the `new`
functions of its module (`_lines.new(x, y, kwargs)`, `_text.new(x, y,
text)`, `_annotation.new(text, ...)`, `_patches.new_rectangle(...)` and a `new_<shape>` per patch
class, `_collections.new_line_collection(...)` and a `new_<kind>` per
collection class, `_table.new(ax, loc, bbox)` for `Table(ax)`; `Artist()`
itself makes a plain artist that draws nothing, `_artist.new()`) and
calls `Artist.__init__`, which sets
`_figure` to None. Its getters and setters are then those of an added
artist. The `add_*` methods (`axes/_base.py`, `Figure.add_artist`) call
`_native.artist.adopt_into_axes` / `adopt_into_figure` (through
`Artist._add_to_axes` / `_add_to_figure`, which a class overrides when it
joins differently), and `_handles.adopt` gives the object the returned
handle and registers it in the figure. The returned handle is untyped (a
`RawHandle` native object), which the `artist_handle!` conversions accept
(see "The native object"). The loose handle is not used
again: its holder is empty. An artist's `axes` is the one pltrs (Rust)
has it in. `artist.set_figure(fig)` alone moves a loose artist into the
figure as a removed artist (`Figure::take_artist`), which the figure does
not draw but measures (`RawHandle::measured_extent`) and draws where the
program asks (`RawHandle::draw_onto`, a `RendererAgg` of no figure
drawing onto pixels of its own), as matplotlib's do.

An artist's `figure` is the (sub)figure it is on, a `(Sub)Figure`'s its
root, as in matplotlib; `_figure` is always the root, which keeps the
registry (`_handles` reaches it through `figure._figure`).

Artists that own others are made in their figure instead, as removed
artists, since matplotlib's constructors take their parent (`Axes(fig,
rect)`, `Legend(ax, ...)`, `SubFigure(fig, spec)`, and `Quiver(ax, ...)`,
`Barbs(ax, ...)` and `QuiverKey(Q, ...)`, which pltrs (Rust) lays out
for their Axes: `Axes::removed_quiver`): `fig.add_axes(ax)`,
`ax.add_collection(q)` then add them back. A quiver, barbs or a key is
decoupled until then (`Artist._decouple`, see "Artists removed, cleared
and added though they are no artists"), as matplotlib's constructors
give them no figure.

An `InsetIndicator` is a plain loose artist with a loose `Rectangle`
until an axes adds it: `Axes::adopt_indicator` then makes the rectangle
its frame and gives it its four `ConnectionPatch`es (`_native.insets`).

Offset boxes are the exception: their constructors take no parent, so
an `AnchoredOffsetbox` or an `AnnotationBbox` is a plain loose artist
until it is added (every `OffsetBox` is a loose
`pltrs::offsetbox::ProgramBoxArtist` then, `_native.offsetbox.new_box`),
and the artists of its tree (the texts of text areas, the patches and
lines of drawing areas, the image of an `OffsetImage`, the frames of
padded and anchored boxes, made with the box as matplotlib makes them)
are loose artists. Adding the box describes its tree to pltrs (Rust)
(`OffsetBox._native_tree`, `_native.offsetbox`), whose
`BoxItem::from_loose` moves each artist out of its holder into the box;
the box takes the common properties of the plain artist, and the box that
added the tree, its owner (`_offsetbox_trees.TreeOwner`), gives every
Python artist of the tree its handle in the figure. Each box of the tree
knows its place in it (`_node`: the owner's handle and its index in tree
order, a box before the boxes inside it), its figure, Axes (an
`OffsetBox` gives its children its Axes, an `AnnotationBbox` none) and
staleness being the owner's (`_tree_owner`). Through its place, every box
of the package's has matplotlib's protocol: `get_bbox` is the extent pltrs
(Rust) lays it out with (`pltrs::offsetbox::node_extent`), `get_offset`
where its parent placed it at the last draw (`node_origin`), which a
hidden box keeps, `get_window_extent` the two together, and `draw` draws
its artists where pltrs (Rust) placed them (a container through
matplotlib's own `OffsetBox.draw`), so that matplotlib's algorithms
written in Python work over them. A text area places its text as
matplotlib's does, through `offset_transform + _baseline_transform`
(`Placement::Text` gives the text that translation at each layout, its
position kept). The transforms matplotlib's draw sets on the leaves (the
`offset_transform` of text areas, drawing areas and aux-transform boxes,
a text area's `_baseline_transform`, a drawing area's `dpi_transform`)
read where the last layout put the box (`offsetbox._FollowsLayout`). An
aux-transform box outside any tree gives its children its transform and
measures them as matplotlib's `get_bbox` does; they read back the
transform pltrs (Rust) places them with once it lays the box out.

matplotlib reads a box's settings and children again whenever it draws
it, so the owner hands pltrs (Rust) its tree again whenever the
description changed (`TreeOwner._sync_tree`,
`pltrs::offsetbox::rebuild_tree`, which keeps the artists the tree held,
takes the loose ones and lets go of those it no longer holds): when a
setting of a box is set (packers' `pad`, `sep`, `width`, `height`,
`align` and `mode`, a padded box's `pad`, a drawing area's size and
clipping, an image's zoom, a text area's `multilinebaseline`, a box's
visibility: `_offsetbox_trees.TreeSetting`, `OffsetBox._changed`), before
Python measures a box of the tree, and before every layout of a draw
(owners follow their figure's layout hook, `axes._program_locators`),
which sees the changes Python cannot (a list of children changed in
place). An anchored box hands its anchoring settings again the same way.

A box of a class of the program's that sizes and draws itself (an
`OffsetBox` subclass describing no tree, or a subclass of the package's
boxes overriding `draw`, `get_bbox`, `get_offset` or
`_get_bbox_and_child_offsets`: `OffsetBox._sizes_itself`) is a leaf of
the tree, `("program", visible, box)`, whose painter is the box (see
"Drawing defined in Python"). Once the figure is laid out, pltrs (Rust)
asks the painter for the box's extent (`ArtistPainter::box_extent`: its
`get_bbox(renderer)`, through `_python_drawing.measure_box`) and lays the
boxes holding such leaves out again; the box's `draw` then runs as a
painted artist's, after its `set_offset` was given where its parent placed
it (`OffsetBox._place_as_laid_out`), as matplotlib's parents do. Such a box
added on its own is a painted artist. It draws its children itself, as
matplotlib's do: each box of the package's among them becomes a box placed
at its offset that the program's box owns
(`ProgramOffsetBox::add_box`, `OffsetBox._join_children`), laid out with
the figure but drawn only where the program's `draw` draws it, after its
`set_offset` moved it; a box of the program's among them hands its
children on; a subclass of one of the package's leaves (a `DrawingArea`
overriding `draw`) gets the box its class describes so, which the
package's methods it calls (`super().draw(renderer)`) measure, place and
draw. Such boxes are measured once per layout, where matplotlib's parents
ask a child's `get_bbox` again in each of their `get_bbox`, `get_offset`
and `draw`.

A table's cells are no artists of their own in pltrs (Rust): they are
part of the table's native state, and a Python `Cell` is the cell at its
`(row, column)` key in its table. A table places its cells when it is
drawn or measured, as matplotlib's `_update_positions` does: until then
a cell measured or drawn alone is where it was made, at the origin with
its own size (`TableArtist::place_as_is`, `Table::place_cells`). A cell
made by its constructor (`Cell(xy, width, height)`) lives in a loose
table without an axes, until `table[row, col] = cell` copies it into a
table and makes the object that table's cell.

### Artists removed, cleared and added though they are no artists

matplotlib's `remove` and `clear` decouple an artist from its figure and
Axes (its `figure` and `axes` read None) and leave it its transform and
clip box, so that it keeps measuring where it was. pltrs (Rust) keeps a
removed artist apart in its old figure (`DetachedArtists`, whose axes
still give their trees' leaves), and the Python object keeps its root
figure (`_figure`, through which it reaches its registry and its trees)
while `Artist._decouple` marks it decoupled: `get_figure()` and `axes`
read None, and `set_figure` and the `add_*` methods take it again. What
matplotlib's last draw computed and the artist keeps (an annotation's
transform, a quiver's) is computed as it is decoupled
(`_keep_drawn_state`), and a child Axes keeps the place its host gave it
(`_leave_host`). The removal methods fail as matplotlib's do: removing an
artist again raises the `ValueError` of the list it was taken out of
(`_remove_again`; an Axes' legend's removal does not fail), an Axes'
removal is its figure's `delaxes`, a `KeyError` once it is out, and a
child Axes' its host's list of child Axes (`_check_removable`), and the
parts an artist made of itself (an Axes' spines and titles, a legend's
texts, handles and frame: `RawHandle::is_part`) have none,
`NotImplementedError`. An object `Axes.add_artist` is given that is no
artist (a container, a colorbar) stays in the Axes as matplotlib's does
once its `set_figure` fails (`axes/_foreign_children`): the Axes lists it
among its children, and every draw of the figure fails on it in the draw
hook, as matplotlib's `Axes.draw` asks its children whether they are
animated. A collection of another figure that `add_collection` is given
is listed and drawn there before the `RuntimeError` of its `set_figure`,
as matplotlib's is: a stand-in artist of the Axes, painted through
Python, draws it with its own `draw` and the renderer of the Axes'
figure, through the transforms of the collection's own figure.

The same stand-ins list what matplotlib lists in a second place, which
pltrs (Rust), holding each artist in one place of one figure, cannot
(`_foreign_children.held_elsewhere`, `relist`): `add_artist` given an
artist of another figure lists it before the `RuntimeError`, one removed
from another figure moves into the Axes' figure (which it then reads as
its own, `_listed_in`), and a part of the Axes (a spine, a title), an
artist of the figure itself, a box of a tree of offset boxes or an Axes
are listed once more and drawn twice; `Figure.add_artist` given a child
of one of its Axes lists it among its artists too (`relist_in_figure`).
The stand-in's removal method takes such an artist out again
(`_listing`, which `remove` consults before refusing a part). A part
taken out so is decoupled while its owner still draws it: its owner then
fails to draw where matplotlib's part reads its missing Axes or figure
first (a linear spine its Axes' view limits, an axis the view interval
or a 3-D Axes' scaled limits, a visible title its figure's resolution),
kept as a problem of the owner (`_unusable`), since a hidden part is not
drawn at all. The other add methods give the artist the figure first,
failing on one of another figure before anything else (`add_held`).
`artist.axes = None` on an artist of an Axes reads None from then on
(`_axes_unset`), and `_recouple` undoes all of these as an add method
takes the artist back.

A stand-in draws through the artist's own `draw`, which draws nothing
for a *mirror* (a tick's label or line, an axis label, an offset text:
pltrs (Rust) draws those with their axis). matplotlib's artist draws what
it was last given wherever it is drawn, so the stand-in asks the part
for an artist of its own made like it for that draw
(`Mirroring.alone`: the mirror's text or data and properties, the
transform its axis gives it, its figure): an Axes listing the tick label
of another figure's axis shows it where that axis' transforms put it.

### Removed and cleared parts measured where they were

matplotlib's `remove`, `delaxes` and `clear` only take an artist out of
lists: a removed Axes keeps its position, limits, tick helpers and
children, a removed subfigure its box, a title or a tick that `clear`
replaced its text and place, and all of them still answer
`get_window_extent` and `get_tightbbox`. pltrs (Rust) keeps removed
artists apart from the figure's store, where the code laying out and
measuring artists does not look them up, so the design is:

* **A removed tree stands in while it is measured**
  (`figure::standing_in`, `RawHandle::standing_in`): the artist removed
  itself and all it owns move into the store for the length of a measure,
  listed by no artist, so the figure neither draws them nor hands them to
  a layout engine, while `FigureState::prepare` autoscales, places and
  ticks a removed Axes as any other and every lookup by id finds its
  parts. The measuring entry points take it: `measured_extent`,
  `extent_as_is`, `Axes::tight_bbox`, `Axis::tight_bbox`, `Axes::ticks`,
  `Figure::tight_extent`. A new measure of a removed artist takes it too;
  nothing else needs a patch per kind of artist.
* **What is removed remembers where it was**: an Axes or a subfigure
  removed from a figure frame keeps that frame as its `home`, in whose
  fractions it stays placed (`FigureState::frame_bbox` and the leaves of
  the transform trees follow it), and is still of that (sub)figure
  (`Figure::parent_of`). The Python object of a removed subfigure stays
  the figure of the artists removed with it.
* **`clear` makes new titles** (`FigureState::renew_titles`): the old
  title artists stay parts of the Axes, with their text and the place the
  last layout gave them, in `AxesState::former_titles`; the Axes no
  longer draws them. The object a program holds keeps reading its own.
* **What the program makes keeps what it was made with until placed**:
  a spine made with `Spine(...)` or `linear_spine` has `transData` and no
  axis until something asks its position (`Spine::is_unplaced`; an Axes
  never drawn places the spine that takes a side's place, as it makes
  its ticks), and the children of a clipped `DrawingArea` keep the
  outline of the area as it was when they were first drawn
  (`offsetbox::apply_placements`).
* **Ticks dropped by `clear` and `reset_ticks` retire**
  (`Tick._retire`): ticks are mirrors of what pltrs (Rust) draws, so a
  dropped tick stops mirroring, and its parts become artists of their
  own with the text and location of the last draw, placed by the
  transforms the axis gave them. Those are read inside
  `spines._transforms_read()`: a new tick asking the Axes for them sets
  the position of a spine a clear left unset, which makes the ticks anew
  (`Spine._ensure_position_is_set`), and a tick that goes asks nothing.

### Axes sharing an axis across figures

matplotlib has one `Grouper` per axis name for the Axes of all figures,
so `ax2.sharex(ax1)` and `plt.subplot(..., sharex=ax_of_another_figure)`
join Axes of two figures. pltrs (Rust) keeps one state per figure, with
the groups of its own Axes; what joins them across figures are *links*
(`axes::share_across`): each of the two Axes records the other, in its
own figure, held weakly. The group of an Axes is its figure's group and,
through the links, the groups of its peers (`FigureState::peers`). The
few places that keep a group in step reach the peers after the Axes of
their own figure, borrowing each other figure for the change (one that
code further up the stack uses is passed over):

* limits (`set_axis_view`): the peers take them without emitting, their
  limit callbacks run after those of the figure that changed, and their
  figures are stale (matplotlib's `draw_idle`);
* autoscaling: an axis is stale when a peer's is, and autoscales over
  the data and sticky edges of all (`shared_data_across`);
* scales, tick helpers and whatever else `update_shared_axis` shares;
* removal and `fig.clear()`: the Axes leaves, and the others stay
  together (`leave_peers`); a cleared Axes takes its peer's axis again.

Each figure is laid out and drawn by itself. The links are not archived
(a figure's archive holds one figure): an Axes pickles the Axes of other
figures it shares with, which brings their figures into the pickle as
matplotlib's does, and joins them again when read back
(`axes/_sharing.py`, `Axes::join_across`). The Python helpers of a
shared axis (`locator`, `formatter` objects) are one object for all the
figures sharing it (`_tick_helpers._held_by_peers`). Locators shared
across figures count the ticks that fit along the axis of their own
figure, not along the axis of the figure they were set in.

### Picking

`Figure.pick` runs on button presses and scrolls, as in matplotlib
(`backend_bases._pick_handler`). pltrs (Rust) keeps the pickers it decides
with (`True` and a tolerance, `Picker`), orders the pickable artists as
matplotlib's recursion over the children does (`Figure::pick_order`) and
hit-tests them (`contains`, with the tolerance a class gives,
`_pick_radius`). A callable picker stays with the Python artist, which
names itself among the artists to order; the pick then calls it.

### Transforms

`transforms` is matplotlib's Python transform tree, computing with
`_native.transforms`. The trees of figures and Axes are matplotlib's too
(`_transform_trees`): `fig.transFigure` is a `BboxTransformTo` of
`fig.bbox`, a `TransformedBbox` of `fig.bbox_inches` through
`fig.dpi_scale_trans`; a subfigure's `transSubfigure` maps its
`bbox_relative`; an Axes' `transData` is `transScale + (transLimits +
transAxes)` over its `_position`, `_viewLim` and scales, `bbox` the
`TransformedBbox` of `_position` through its figure's `transSubfigure`,
built by the class' `_set_lim_and_transforms` (polar Axes:
`projections/_polar_trees.py`; geographic Axes: `GeoAxes`). Each object
is made once, the first time one of them is read (`TreeAttribute`), and
kept, so `ax.transData is ax.transData` and `line.get_transform() is
ax.transData`, and the trees compose, invert, freeze, invalidate and
pickle (they are made anew when read back) as matplotlib's do.

What pltrs (Rust) draws agrees with those trees where they are not
finite. A view its scale maps onto one point (a logit axis with limits
beyond 1, a function scale whose inverse is NaN over the margin) gives
`transLimits` an infinite scale, and matplotlib's matrix products leave
no entry of `transData` finite: nothing placed in data coordinates is
drawn, neither axis shows ticks, and the Axes keeps its patch, its
title and what is placed in axes fractions. pltrs (Rust) builds the same
matrices (`ProjectionView::rectilinear`, `bbox_transform_from`). The
tick labels matplotlib then measures at NaN put the union of their boxes
at NaN, and with it the axis label and the offset text beyond them
(`AxisTexts::unplaced_labels`, `measured_at_nan`). An image's box is
then at NaN too (the corners bounded as `TransformedBbox` bounds them),
which intersects no clip box, so that no image is made, clipped or not.
Contour labels on a
line that is at NaN on the canvas fail as matplotlib's (`math.floor` of
the NaN numpy's `interp` gives: `numeric::interp` searches as numpy
does, `ContourError::LabelCutAtNan`).

The leaves of the trees are pltrs (Rust)'s values
(`Figure::tree_leaves`): an Axes' position after the aspect, its
original position, its view and data limits and scales, a subfigure's
box, the figure's size and resolution, and what a projection's trees rest
on besides (polar Axes' theta offset, direction, radial origin and label
position; geographic Axes' longitude cap). They are read when the trees
are built. A figure whose trees are built is watched
(`Figure::watch_changes`): it records itself among the changed figures
whenever a handle borrows its state for change, and at the end
of every native call (an after-call hook, `sync_watched` in
`modules/transforms/leaves.rs`) the figures recorded
(`take_changed_figures`) are read again and the leaves that changed are
handed to `_transform_trees._sync`, which sets them as matplotlib sets
its own, invalidating what is built on them; a call that changed no
watched figure costs the hook next to nothing. A draw's Python code reads
those of the laid-out figure (`_transform_trees.sync`).
A leaf the program sets (`ax.viewLim.set(...)`, `ax.dataLim`,
`fig.bbox_inches`) is handed to pltrs (Rust) (`_LeafWatcher`).

The trees are the state as it is, as matplotlib's: before a draw,
`ax.transData` still maps the view limits autoscaling has not computed
yet (`(0, 1)` for a fresh axes), the position an aspect has not adjusted
yet, and the positions a layout engine has not set yet. What matplotlib
brings up to date brings pltrs (Rust)'s state up to date at the same
moments: reading the limits (`viewLim`) autoscales, `get_position()`
applies the aspect (`Axes::apply_aspect`, which leaves the layout engine
to the next draw), contour sets and stream plots autoscale when made (in
pltrs (Rust), `Axes::contour`, which asks for the view again as
matplotlib's `Axes.contour` does: labelling the set places its labels in
that view and applies nothing that is pending), and
a draw lays the whole figure out. `canvas.inaxes` and
`Axes.contains_point` test the axes where it is now, as matplotlib's
patch test does. A removed axes keeps its transforms, at its last
position in the figure.

Window extents are computed as matplotlib computes them, from the trees
as they are: a line's vertices through its transform padded by its
markers, a patch's path through its transform, a collection's data
limits in display coordinates (`get_datalim` with the identity: null for
a scatter's offsets in data), an image's extent through its transform, a
text where its transform puts it (`RawHandle::extent_as_is`, the figure
not laid out), `Artist`'s zero box. Legends, tables and annotation boxes,
which matplotlib lays out when they are measured, give where pltrs (Rust)
places them in the figure as it is (`_extent_as_is`); anchored offset
boxes and the parts of axis artists, whose matplotlib extents come from
their layout, give where pltrs (Rust) lays them out (`_laid_out_extent`).

State matplotlib only updates when it draws an artist (the transform of
an annotation, the path of a spine or an arc, the transform of a quiver,
the size transforms of a collection, the corners of table cells)
follows pltrs (Rust)'s count of the draws of each artist
(`RawHandle::draw_count`, `_native.artist.draw_count`): the Python side
keeps what it computed with the count it computed it at, and computes
it again once the count moved on.

`_live_transforms` is the one bridge between the two trees, both ways:

* **To pltrs (Rust).** A transform crosses as the nested tuples of
  `_native.transforms.native` (`to_native`): composites, blends,
  `ScaledTranslation`s and affine maps around the nodes of the trees that
  stand for a transform pltrs (Rust) has (`ax.transData`, `ax.transAxes`,
  `ax.get_xaxis_transform()`, `fig.transFigure`, `fig.dpi_scale_trans`),
  which carry it as `_live` and cross as it, resolved by pltrs (Rust) in
  the figure's layout whenever it draws; a box of an Axes or figure
  crosses as the transform spanning it (`_spanning`). A transform of the
  program's crosses too: an affine one as its matrix, any other as
  `("python", t)`, a pltrs (Rust) map calling its `transform` (and its
  inverse's) back. Everything that gives pltrs (Rust) a transform goes
  through it: `Artist.set_transform`, the `transform` keyword of the line
  properties, `Collection.set_offset_transform`, and the `transform=`
  keywords of plotting functions (`scatter`, `quiver`, `barbs`,
  `streamplot`), which pass `given_to_native(t)`, None when absent so
  that the native function picks its default.
* **Back to Python.** A native transform comes back from `describe` as
  the nodes of the trees of the Axes and figures it names
  (`from_native`): `ax.transData`, a blend of an axes' data and fractions
  as its `get_xaxis_transform()`, a composite as a composite; a transform
  pltrs (Rust) has that no node stands for (a projection's map) as a
  `LiveTransform`. `of_artist(artist, native)` gives an artist's transform
  or offset transform (`IdentityTransform` for none), the same object
  while pltrs (Rust)'s stays the same (`kept`), so that
  `Artist.get_transform` and `Collection.get_offset_transform` return the
  object they were given, or matplotlib's for the one pltrs (Rust) has.
  A clip box pltrs (Rust) gives is the unit box through the patch of its
  Axes or figure, as matplotlib's `set_clip_path(self.patch)` makes it:
  for an Axes whose patch is no `Rectangle` (polar, geographic, a
  program's circle) no box but the patch's `TransformedPatchPath` as the
  clip path, which a clip path set replaces and a clip box set keeps (the
  same for the axis and the gridlines of its ticks,
  `_artist_clipping.patch_clip_path`), and `add_artist` gives what pltrs (Rust) adopts its own way (offset
  boxes, inset indicators, quiver keys, tables) matplotlib's transform
  and clip box (`take_parent_defaults`). The parts of an axis (mirrors)
  answer with matplotlib's transforms of them
  (`get_xaxis_text1_transform`...).

A non-affine transform of the program's crosses wrapped in
`_live_transforms._ProgramMap`: pltrs (Rust) maps points through its
`transform`, and whole paths through a matplotlib `TransformedPath`,
which keeps the path the transform last gave until the transform is
invalidated, so that a draw after an affine change (`transAxes`
invalidated) does not call the program's transform again, as in
matplotlib. The transform is asked for its inverse when a point first
goes back through it (`_InverseMap`), not when it crosses: matplotlib
inverts nothing when an artist is given a transform, and programs count
the calls of their `inverted`.

Python callables that map values are called with arrays, as matplotlib
calls them: the transforms of the program's, the functions of function
scales and secondary axes (`FuncTransform`), the transforms of scales of
the program's and the `(trans, inv_trans)` functions of curvilinear
grids. pltrs (Rust) gathers the values a path, a set of ticks or a limit
needs and calls them once for all of them (`AxisMap::forward_all`,
`PointMap::forward_all`, `AuxTransform::transform_points`;
`modules/transforms/arrays.rs`), a masked answer reading as NaN; where
matplotlib calls `Transform.transform` (autoscaling's margins), a function
of a one-dimensional transform gets the values as a column, shaped
`(N, 1)` (`AxisMap::forward_column`, `scale::ArrayLayout`). A secondary
axis hands its forward function its host's limits as one array wherever
matplotlib's `_set_lims` runs (`tick_sites`, `axes_tight_bbox_at`); an
axis
keeps its view limits mapped through its scale while they and the scale
stay (`AxisState::scaled_ends`), as matplotlib's `TransformedBbox` of
them does. Formatters keep matplotlib's per-tick `__call__`, through
`format_ticks`.

An `Axes` subclass the program registers with `register_projection`
(the SkewT-logP example's `SkewXAxes`) is a projection pltrs (Rust)
does not know (`projections/_program.py`): its Axes is a rectilinear
pltrs (Rust) axes that `set_up` builds as matplotlib's `__init__` does
(its trees with the class' `_set_lim_and_transforms`, from the boxes
matplotlib builds them from: a new view, no data, and the unit box for an
Axes of a grid, whose cell replaces it after; then `_gen_axes_spines` and
`_init_axis`, from the mixin
`axes/_projection.py`), then hands over to a pltrs (Rust) `Projection`
asking the Axes through an adapter (`_native.projections`,
`Axes::install_projection`): the data transform is `transData` up to its
final `transAxes`, the gridlines' and ticks' transforms those of
`get_xaxis_transform` where the class gives its own, the tick interval
that of an overridden `Axis.get_view_interval`, the sides a tick draws
those an overridden `Tick.draw` leaves visible on a tick made alone, and
the interval of a spine that of an overridden `Spine._adjust_location`,
and the data ratio that of an overridden `get_data_ratio`, asked wherever
matplotlib applies the aspect (`Projection::data_ratio`: where a draw of
the figure applies it, `_get_draw_artists`, and where the axes is drawn).
Every clear (and the making of the Axes by `add_subplot`, as matplotlib's
`__init__` clears it) gives the Axes the patch its class'
`_gen_axes_patch` makes (`_program.cleared`): unless it is the unit
square, its outline is the background's shape and clips what the Axes
clips (`Axes::set_patch_outline`; a `Rectangle` clips as its box, as
matplotlib clips to one), images within the Axes box as matplotlib's
`get_clip_box() or self.axes.bbox`.
pltrs (Rust) asks while it lays the figure out or draws it, when the
figure cannot be asked about itself, so each hook passes the view limits
it works with, which the Axes' `_viewLim` is set to meanwhile; the
class' transforms are matplotlib's Python classes and compute as
matplotlib's do. The Axes' `transData` and axis transforms reach pltrs
(Rust) as the Axes' own (`_live`). A transform not ending with
`transAxes` crosses followed by its inverse.

A subclass of `PolarAxes` the program registers (the radar chart
example's `RadarAxes`) goes the same way, with a polar pltrs (Rust) Axes:
`PolarAxes.__init__` makes it (`Figure::removed_axes_with_projection`)
and runs the class' construction as matplotlib's does (its `__init__`,
`_gen_axes_spines`, `_init_axis`, two clears), then `set_up` hands it
over (`ProjectionState::ProgramPolar`). pltrs (Rust) keeps its polar
settings, ticks and labels, and asks the adapter only for the data
transform (made with the class' `PolarTransform`, whose
`transform_path_non_affine` draws every path, gridlines with their
interpolation steps) and the spines the class made; every clear gives it
the outline of the patch `_gen_axes_patch` makes (`_program.cleared`,
`PolarFrame::Path`) unless it is a `Wedge`, which pltrs (Rust) fits to the
view as matplotlib's `draw` does (the `Wedge` the patch mirror is reads
its centre, angles, radius and width as the last draw fitted them,
`PolarAxes._fit_patch`). A `Spine(axes, spine_type, path)` is
the spine of its side, or the outer one, drawn as the path through its
transform (`SpinePatch::Path`). While pltrs (Rust) asks, `get_rorigin`
and `get_rsign` answer from the trees, as matplotlib's do, and the
non-affine part computes in the view it was evaluated at.

Transforms a subclass computes are evaluated at draw time, as
matplotlib reads an artist's `get_transform()` in its `draw`: an artist
whose class takes `get_transform` from outside the package is kept for
its figure (`_computed_transforms`, from `_handles.adopt`); the figure's
draw hook (`Figure::set_draw_hook`, set through
`_native.transforms.set_draw_hook`), which pltrs (Rust) runs once the
figure is laid out and before its artists are drawn, sets each such
artist's native transform from its `get_transform()`.

### Values as matplotlib types them

pltrs (Rust) holds numbers as `f64` and colours as its `Color`, while
matplotlib answers with the objects it keeps, very often the object the
program gave (`line.set_zorder(5)` reads back the int `5`,
`line.set_color([0.5] * 3)` the list). `_values` makes the answers
matplotlib's: a setter `remember`s the value given, and its getter returns
it (`given`) while pltrs (Rust) still holds what it converts to; limits are
numpy floats (`float64s`), as matplotlib reads them off its boxes. Colours
pltrs (Rust) took from the property cycle read back as the cycle holds them
(triples, for `tab10`), and `to_rgba` types alpha as matplotlib does (the
int `1` for a colour of three components). `cargo xtask diff` compares
pltrs' answers with matplotlib's (`spec/diff`).

matplotlib's setters keep many values unchecked and fail only when they
compute with them. A value pltrs (Rust) cannot hold is kept on the Python
side (`_values.keep` for what reads back, `_unusable.keep` for what then
fails), and the artist is drawn and measured through Python, where the
same computation as matplotlib's raises the same error. Patches keep their
geometry this way (`_kept_geometry`): a field that is no number, or a
geometry whose arithmetic fails or warns (an infinite angle), has the
patch recompute it as matplotlib does (`Ellipse._recompute_transform`,
`Annulus._recompute_path`, ...) wherever matplotlib's draws and measures
do; spines keep their bounds, centre and size, and their Axes fails to
draw while a spine its axes measure beside their labels cannot be laid
out (`Spine._locate`). Arithmetic matplotlib does on Python's or numpy's
scalars before it hands numbers on (`Affine2D.scale`, `Path.arc`'s angles,
`Collection._bcast_lwls`) is done the same way, so that it raises and
warns as it does.

Values matplotlib keeps unchecked and computes with only where it draws or
measures (a text's position, an annotation's point and coordinate systems,
a figure's resolution) are kept by the artist when pltrs (Rust) cannot hold
them (`_unusable`), and fail there with matplotlib's errors, in the order
matplotlib's draw computes with them (an artist's `_kept_order`): a text's
coordinates convert with its Axes' units as `Text.draw` converts them
(`Text._set_text_coordinate`), an annotation places its point as
`_check_xy` and `update_positions` do (`_annotation_kept`).

A plotting method whose matplotlib code is a computation in numpy over
artists (`hexbin`, `pie`) runs that code over the package's artists for
the arguments pltrs (Rust) does not take as they are, and after pltrs
(Rust) refuses arguments (`axes/_plotting/_hexbin_reference.py`,
`_pie_reference.py`): it raises matplotlib's errors, or makes what
matplotlib makes of them. Others check what matplotlib's first steps
check, in its order, before pltrs (Rust) computes (`hist2d` bins unusual
arguments with numpy's `histogram2d`; `bxp` merges its properties;
`stackplot` checks its first layer as `fill_between`).

### Keyword arguments and aliases

* Aliases are declared on the class, as in matplotlib:
  `@_api.define_aliases({"linewidth": ["lw"]})`. `cbook.normalize_kwargs`
  resolves them.
* `Artist.set(**kwargs)` normalizes and calls `set_<name>(value)` for each
  key, raising matplotlib's `AttributeError` for unknown ones; subclasses
  override setters as in matplotlib.
* Plotting methods pass their artist properties to the native function in
  one dict, after normalizing aliases, because pltrs (Rust) applies them when it
  makes the artist (a given colour, for instance, does not advance the
  property cycle). The module file converts the dict into the pltrs (Rust)
  property set (`lines::LineKwargs` builds `LineProps`), raising
  `AttributeError("Line2D.set() got an unexpected keyword argument 'x'")`
  for keys it does not know: extend that table as properties are added.
* Python passes native functions exactly their arguments, positional,
  with `None` for what is absent. Defaults and keyword handling stay in
  the Python signature.
* A keyword pltrs accepts but does not implement yet raises
  `NotImplementedError("pltrs: savefig(bbox_inches=...)")`, never is
  silently ignored: the suite's blocker list then names it.

### Arrays

Python normalizes array-likes with numpy where matplotlib does (masked
arrays, units and lists all go through `np.asarray` in the Python
method), and passes arrays to native functions, which take `NdArray`.
Results that matplotlib returns as arrays come back as `NdArray` (numpy
arrays); lists come back as `Vec`.

pltrs (Rust) keeps copies of what it is given, where matplotlib keeps
the program's arrays and reads them again when it draws: see the next
section.

### State programs change in place

matplotlib's artists are Python objects whose state is plain attributes,
and many getters return the attribute itself: `collection.get_paths()` is
the list of `Path`s the draw reads, `scatter.get_offsets()` the array it
places the markers by, `line.get_xydata()` the array of the line's path.
Programs change those objects in place and expect the artist to follow;
the half violin alone (`body.get_paths()[0].vertices[:, 0] = ...`) is in
countless scripts. matplotlib also keeps some of what it is *given*
without copying (a polygon's `xy`, a `PathCollection`'s list of paths,
the array given to `set_sizes`, a `PathPatch`'s path). pltrs (Rust) holds
that state itself. What both do, object by object, is measured:
`python/tests/mutability` changes in place every object reached from
every public getter and attribute of every kind of artist, and every
object given to one, and compares the read back, the next draw, the data
limits, a measure and a pickle with matplotlib's
(`python/tests/mutability/reference.json`, `python -m mutability`). Each
row is asked twice, of a scene already drawn and of one changed before
its first draw (matplotlib computes some state at that draw), and the
rows of some kinds are asked again with a setter called between the
change and the next read (`setters.py`, one call of the public API for
every native function that is passed by, see the costs below).

One mechanism answers it, `_own_state`, in two halves.

**The object handed out or taken in is the artist's own.** A class names
each such piece of state once, as a `State`: how to read what pltrs
(Rust) holds as the object matplotlib's attribute would be (`read`), how
to hand pltrs (Rust) what such an object holds (`take`, through the
native setter of the property and nothing else: the object was changed,
not set), and what an object holds (`key`: the bytes and shape of an
array, the contents of a list or a `Path`). The states of a class live in
a module of their own, imported when a getter first needs them
(`_collection_state`, `_line_state`, `_patch_state`). An artist keeps a
*record* per state in its `__dict__`, made the first time the object is
handed out (`hand_out`, which the getter calls) or taken in (`keep`,
which a setter calls where matplotlib keeps what it is given): the
object, what it held when pltrs (Rust) last took it, and what pltrs
(Rust) held then.

* *Identity.* `hand_out` reads pltrs (Rust)'s state and gives the
  recorded object while that state is what it was; a new object, recorded
  in its place, once pltrs (Rust) changed it by another way (a setter of
  another property, a draw), as matplotlib's code then binds a new one. A
  setter that binds a new object in matplotlib forgets the record
  (`drop`: after `set_offsets(new)` the old array is nobody's) or records
  the new object. A state matplotlib's draw binds anew although nothing
  changed (the colours of a collection that maps values, the shape
  transforms of one with sizes) has a `version`, the artist's count of
  draws. Where matplotlib returns a new object at each call, the getter
  has no state and does too.
* *Views share.* What matplotlib keeps as one buffer is one buffer:
  a line's `_xy`, its views `_x` and `_y` and the vertices of its path
  (without steps), a polygon's `xy` and its path's vertices, the edge
  colours that are the face colours.
* *Read-only stays read-only.* The paths matplotlib shares between
  artists (the unit circle of circles and ellipses, a regular polygon's,
  the codes of a marker's or a patch's path transformed) are handed out
  read-only, and refuse a change as matplotlib's do.

**pltrs (Rust) draws from it.** `sync(artist)` hands over each recorded
object whose content is no longer what pltrs (Rust) last took. It runs
before pltrs (Rust) reads the artist, and nowhere else:

* before any native call that takes the artist's native object as an
  argument: the binding lets a native object ask to be told
  (`binding::own_state`, `_native.artist.tell_before_calls`), and an
  artist with a record asks. That is every getter, measure, hit test and
  copy of the artist itself, with no list of them to keep. Only a
  thread's outermost native call tells, and the calls made while an
  object is told tell nobody;
* before pltrs (Rust) reads the artists of a figure together, which no
  argument names: the figure's layout hook (see "Grids, layout engines,
  legends") tells them before every layout of a draw or a print
  (`sync_figure`), and the package tells them before the data limits are
  recomputed (`relim`), before a figure or an artist is pickled, and
  wherever else it has pltrs (Rust) measure or search a figure's artists
  (`tell_figure`).

What is handed over makes nothing stale (`pltrs::figure::unseen`), as a
change in place makes no artist of matplotlib's stale, and moves no data
limit: pltrs (Rust) recomputes limits where matplotlib does (`relim`,
adding the artist), from the state as it then is. A `fill_between`
collection measures the points it fills whatever its polygons are, as
matplotlib's keeps their box.

**What it costs.** An artist without a record pays nothing: its native
object asks for nothing, its figure has no hook for it, and its setters
test one key of its `__dict__`. What every native call pays is the
binding's check that nothing asks: a few loads and the size of one dict
(a native call from Python takes 349 ns in all, measured as 200,000
`line.get_alpha()`; 357 ns once another artist of the process has a
record). An artist with records pays the comparison of each recorded
object per native call *that reads it* and per draw of its figure, and
one copy of an object the program did change. That is per call and not
per draw so that `contains`, a measure and a getter see the change at
once, as matplotlib's read the attribute. Three rules keep it small:

* *Native functions that read none of this state tell nobody.* The
  binding lists them in one place (`binding::own_state::PASSING_BY`,
  35 functions, each with a line saying what it reads or writes:
  the plain fields of every artist, the widths, dash patterns,
  antialiasing, caps, joins, hatch, URLs, sizes and offsets of a
  collection, the style properties of a line and of a patch). The
  hand-over waits for the next call that reads the artist, so a setter
  and a change in place are two writes in either order, as in
  matplotlib. A function is listed only once its Rust code was read; one
  that is not listed tells, which is slower and never wrong. One that
  writes recorded state is called by setters that forget the record
  (`drop`), or the old object would be handed over on top of the new
  value. The sweep makes rows of the list (below), and
  `test_mut1.py` fails on a listed function without rows. The collection
  setters of face and edge colours and of alpha are not listed: they
  read the other side's colours natively.
* *An array nothing but its record holds is not compared.* Once a
  hand-over has compared an array at a moment when its reference count
  is the record's alone (`_own_state._only_recorded`), no code can
  change it, and it is left alone until a getter hands it out again
  (`_Record.settled`). The count is compared with one measured at import
  on an array of the module's own, in the same call shape
  (`_count_unheld`), not with a constant: the interpreter's own
  references differ by version. Only arrays that own their numbers
  qualify (a view holds its base, a buffer its exporter), with no weak
  reference to them (one reaches the array without holding it); for a
  masked array, which a scatter's offsets are, the numbers and the mask
  must be nobody else's either. Lists, paths and views are always
  compared: their content is reached another way. Where the interpreter
  runs without its GIL, counts tell nothing and the rule is off, here
  and for the lists of rcParams. What is not seen: a write through an
  address kept without any reference, which matplotlib's artist would
  show, as it keeps the array alive too.
* *Offsets and sizes are compared with pltrs (Rust)'s own.* A state may
  say how to compare an array with what pltrs (Rust) holds, both read
  where they are (`State.same`; `_native.collections.holds_offsets`,
  `holds_sizes`, through the buffer protocol): no copy of the array is
  made and none is kept beside the record. "pltrs (Rust) holds something
  else" then has to mean "the program changed the array", so only states
  that nothing writes natively but their own hand-over and setters that
  forget the record are compared so: the offsets (masked or not) and the
  sizes of 2-D collections. A line's numbers are not: pltrs (Rust)
  converts them again when an axis changes units. An object pltrs (Rust)
  does not hold bit for bit after it took it (another dtype) falls back
  to the comparison with what it held.

Measured on the optimised extension (2026-11-16, CPU time, the least of
nine runs, on a machine under load), a native call that reads the artist
takes 0.1 to 0.2 µs with nothing kept, 1.0 µs for a line of two points
whose array is held, 1.8 µs for a scatter of 1,000 points whose offsets
are held and 40 µs for one of 100,000 (the comparison with a kept copy
took 66 µs on the same array in the same run, and kept 1.8 MB beside
it); 0.9 µs for that scatter once nothing holds the offsets; 52 to 59 µs
for a line of 100,000 points, which is compared with what it held. A
style setter on any of them takes what it takes on an artist that keeps
nothing (1.4 to 1.9 µs). The benchmark
`calls/scatter_1e5_read_then_1000_calls` (offsets read once and held,
then 1,000 `set_linewidth`) pins the case the first rule answers: built
and drawn it takes 52 ms of CPU time with the read and 48 ms without,
where matplotlib takes 92 ms (before the rule: 130 µs per
`set_linewidth`, 178 ms in the bench's run against matplotlib's 80).
`test_mut1.py` holds the 1,000 calls to no comparison at all, which is
what keeps the benchmark where it is. 1,000 `set_edgecolor` on that
scatter still cost about 40 µs each: the colour setters tell. Records
hold their objects as the artist would; pltrs (Rust)
holds only the artist's weak callable. Records are left out of pickles,
which hold what pltrs (Rust) holds after a last `sync`. An assignment to
a recorded path (``path.vertices = vertices``) says that it changed, and
hands it over at once (`path_assigned`): code of the program's that pltrs
(Rust) calls inside a draw moves paths so, past every point where the
artists are told.

**What is no live object.** State pltrs (Rust) computes at each read and
matplotlib stores is kept from its first hand-out on, by the same
record: the path of a wedge, an annulus or an arrow (pltrs (Rust) draws
the object's content as the outline in place of the shape's,
`Patch::set_program_path`, until a setter of the geometry makes the path
anew), the colours of a collection as its last draw mapped them. A
collection that maps values keeps the edge and hatch colours a program
changed in the array only until its next draw sets them again, as
matplotlib's does (`_collection_state.shown_until_drawn`). The path of a
line drawn in steps is made of the line's numbers at every draw: it can
be read and changed, and the change is not drawn.

**What the package's own code reads** is kept by nobody. A getter about
to make an artist's first record of a state looks at what the program
called (`_own_state._read_by_package`: the outermost function of the
package in the call). A member of the artist itself
(``scatter.get_offsets()``, ``line._x``) or a getter of another object
handing the artist's state on (``shadow.get_path()``) is the program
asking, and makes the record. Anything else is the package working:
`boxplot` reading the lines it made, a legend handler reading the
collection it stands for, `getp`, a draw; it gets a new object and no
record, so that only state a program (or a library) asked for is compared
afterwards. Over matplotlib's gallery the package then makes records only
where it keeps an object it is given by reference (a `PathPatch`'s path,
`markevery`). The reads on the plain path of every figure go around the
getters altogether, `colorizer._array_of(mappable)` for matplotlib's
`get_array()` (every colorbar asks for it) and `image._extent_now()` for
`get_extent()`, and pay no look at the caller.

**Transforms** are objects of the program's with matplotlib's own way of
saying they changed: a transform an artist was given (`set_transform`,
`set_offset_transform`, a clip path's, the `transform=` of `plot` and
`scatter`) that holds an `Affine2D` gets a parent node
(`_live_transforms._GivenWatcher`), and a change of the `Affine2D`
through its methods (`t.clear().rotate_deg(angle)`, the animation idiom)
hands pltrs (Rust) the transform again at once, so that nothing is
compared at a draw.

**The lists of rcParams** are no artist's: `rcParams["font.sans-serif"]`
is the parameter itself in matplotlib, and programs insert into it. An
`RcParams` keeps a list it hands out by its key (`__getitem__`) with a
copy of what it held and the rc text pltrs (Rust) then held; read again
it is the same list while pltrs (Rust) holds that text, and what the
program changed in it is taken first. Any native call may read the
current parameters, so the binding compares the kept lists with their
copies before each outermost native call (`binding::own_state`'s
`install_lists`, one C comparison per list, 12 ns) and has the changed
ones taken. Two rules keep that off the plain path, where 30 kept lists
cost the gate scenes 13 to 35% (`dict(rcParams)` held, measured):

* *A list is kept only while a program holds it.* Once nothing but the
  record references it (`sys.getrefcount`), nothing can change it: it is
  forgotten at the next read of any parameter, with what was changed in
  it. ``rcParams["font.sans-serif"].insert(0, name)`` keeps a list for
  one statement.
* *A walk over all the parameters keeps none.* Reads by item that follow
  the order of the last iteration (``dict(rcParams)``, ``items()``,
  `__repr__`, a comprehension over the keys) are a copy of them all: the
  lists are the copy's own, where matplotlib's copy shares them with the
  parameters. `copy()` is a new `RcParams` with lists of its own too.

The package's own code reads list parameters with `_get`, which keeps
nothing. So a program that changes no list in place has none compared
(`python/tests/test_mut1.py` checks it), and one that holds a few (a
style context of seaborn's holds two for its block) pays 12 ns per list
and native call. A list that was the parameter once and is no more (the
parameter was set by another way since) is nobody's: nothing takes it,
and the end of an `rc_context` does not put it back; the lists that were
the parameters when the context began are put back as the very objects,
with what was changed in them meanwhile, as matplotlib's are.

**Lists of artists** are one list each, as matplotlib's attributes are:
`Figure.texts`, `artists`, `legends` and `images`, `Axes._children` and
`child_axes` hand out the list handed out before
(`_own_state.same_list`), of a class whose changes in place add and
remove artists (`_figure_artists.DrawnList` and its subclasses) and which
brings itself up to what pltrs (Rust) lists whenever it is used. Nothing
is compared at a draw and nothing is kept for a figure whose lists no
program read; the package's own code reads the lists through
`_texts_now()`, `_children_now()` and the like, which hand nothing out.
Where an attribute is also one a program assigns (`Figure.images`:
`_values.reassignable`), the two compose: the getter is the list's, and
assigning it back is the no-op it should be. An axis' `majorTicks` and
`minorTicks` are its own lists too, filled up when the axis makes ticks
or is drawn and left as the program left them in between.

**What matplotlib makes once and draws nothing from** is kept from its
first read on, with no record (`_own_state.kept_once_read`): a contour
set's `levels`, `layers`, `cvalues` and its lists of labels (filled anew,
the same lists still, when `clabel` adds labels), a legend's `texts` and
`legend_handles`. A program changing them in place reads the change
back, in a pickle too.

**State that is no artist's record** is asked for by the figure where it
tells its artists (`_own_state.follow`: before a draw's layout, before
`relim`, a measure and a pickle), with no native call asking:

* the limit boxes of an Axes whose transform trees are built: the
  watcher of each box (`_transform_trees._LeafWatcher`) compares the
  array of its points with what it last took, so that
  `ax.viewLim.get_points()[1, 1] = top` and `ax.viewLim.intervalx[1] =
  right` are the limits pltrs (Rust) draws; the Axes also looks before
  its limits are read (`get_xlim`). pltrs writes its own limits into the
  same array, so the array a program holds stays the box's;
* the limit boxes of a 3-D Axes (`_limit_boxes`), likewise;
* a `FixedLocator` that keeps an array the program has (the array given
  to it or to `set_xticks`, or its `locs` once read): installed again
  when the array changed;
* a colormap whose lookup table a program read (`cmap._lut`, a property
  over the same storage that tells a read from outside the package):
  every live figure asks its watch (`colors._LutWatch`), and a table that
  changed is handed over again by the colorizers drawing with it, as
  `Colormap.set_bad` has them do.

**More states of the kind above:** the mapped array by its private name
(`_A` is a property handing out what `get_array()` does; the package
keeps the array as `_mapped`, which its own code reads), the vertices of
a quad mesh (`get_coordinates()`; pltrs (Rust) takes them back with
`Collection::set_mesh_coordinates`), a 3-D Axes' box aspect, the data of
3-D lines and scatters, a quiver's `U` and `V`, the clip box an artist
was given when it is a `Bbox` of the program's.

**The first draw of a collection sets its colours again** in matplotlib
(its first `update_scalarmappable` finds its flags changed and resolves
the colours from what the setters were given), so a change made in the
arrays `get_facecolor()` or `get_edgecolor()` hand out before that draw
is read back until then and never drawn; after it the arrays are the
collection's own. pltrs does the same (`_collection_state`'s
`shown_until_drawn`), as it does at every draw for a collection that
maps values.

**A draw style set anew** makes matplotlib compute a line's numbers
again at its next use (``_invalidx``): the x numbers from the data the
line was given, the y numbers as they are, in a new array. `set_drawstyle`
does that at once (`Line2D._recache_x`): what a program changed in the y
numbers of the array it has, and in the x data it gave, is drawn; what it
changed in the x numbers is not; the array is the line's no more.
(matplotlib's private `_x`, `_y`, `_xy` and `_path` still are the old
objects until that next use; pltrs' are new at once.)

**Before the first draw** matplotlib has computed less than after it, and
some of what it then computes it reads from objects a program gave or
may have changed. Two of them are records that live until the artist's
first draw and are dropped at the next hand-over after it
(`State(until_drawn=True)`): the path an artist is clipped by, given
with its transform (matplotlib transforms it when the artist is first
drawn), and the array `Collection.set_facecolor` was given (its first
draw resolves the colours again). The sweep's rows changed before the
first draw still differ from matplotlib's where pltrs took the content
when the artist was made: the `colors` of a `ListedColormap` (its table
is made at first use; following it would compare every program colormap
at every draw), the unit data of an `axline` (matplotlib's first draw
makes the line's path of them, which shortens the line: an accident);
matrices and boxes changed in their arrays before any cache of
matplotlib's exists (`dpi_scale_trans`, which would take a figure scale
of two factors; an Axes' `bbox`, which would take an Axes drawn where
its position is not); and the array of a `NonUniformImage`, which pltrs
draws changed and matplotlib does not. They are listed in
`expected.json` with the keys `first_draw`, `first_kept` and
`first_value`.

**The 3-D artists** (`mpl_toolkits/mplot3d/art3d.py`) keep the third
dimension in attributes of their own in matplotlib, and project them at
every draw. pltrs (Rust) keeps and projects the geometry; the classes
answer for matplotlib's attributes through two descriptors
(`mplot3d/_properties.py`), whichever way an artist was made (the class
itself, a 2-D artist converted, a method of `Axes3D`):

* `Field`, an attribute of the geometry: `_segments3d`, `_segment3d` and
  `_code3d`, `_faces` with `_invalid_vertices` and `_codes3d`,
  `_3dverts_codes`, a text's `_dir_vec` (`_verts3d` and `_offsets3d` were
  records already). An artist the program made keeps the object it was
  given; one a 3-D Axes made reads what pltrs (Rust) holds
  (`_native.mplot3d.kept`) into the container matplotlib's method builds
  (`mplot3d/_fields3d.py`, loaded at the first such read: a list of
  arrays for `quiver` and `plot_wireframe`, one array for `errorbar`,
  lists of tuples for `stem`). Either becomes a record at the first
  read, so it is the same object at every read and is handed back when a
  program changed it in place; assigned, the attribute keeps what was
  assigned and hands it over at once.
* `Left`, what a projection leaves behind: `_vzs`, `_z_markers_idx`,
  `_offset_zordered`, `_data_scale`, `_facecolors2d`, `_edgecolors2d`
  (and `_path2d` of a patch). It is computed with matplotlib's formulas
  from the fields and the projection matrix of the last draw
  (`Axes3D.M`) when first read after a projection, and is one object
  until the next (a projection is told by the draws of the artist's
  Axes, which projects its visible collections and patches before it
  draws any, an animated one too, and by the projections the program
  asked for). Assigned, it reads back until
  the next projection, as matplotlib overwrites it then.

The plain values are properties: `_axlim_clip` on every class,
`_depthshade` and `_depthshade_minalpha`, `_closed`, `_zsortfunc` (one
of matplotlib's three functions; pltrs (Rust) sorts by no other, and
assigning one says so: nothing in the census or the corpora does).

*Subclasses of the program's.* matplotlib's `Axes3D.draw` asks every
visible collection and patch for its depth (`do_3d_projection`) before
it draws them. A class of the program's that overrides the method on a
class of `art3d` has it called before each draw
(`mplot3d/_program_projection.py`, behind `takes_from_program`): it
reads and changes the fields, calls the method it overrides, which has
pltrs (Rust) project the artist, and the depth it returns orders the
artist (`_native.mplot3d.set_program_depth`). A 3-D patch that another
patch class draws (qiskit's `Arrow3D` is a `Patch3D` and a
`FancyArrowPatch`, drawn by the latter's `draw` with its ends where its
`_segment3d` projects) is not lifted into pltrs (Rust)'s 3-D geometry at
all, which would replace the arrow by the outline of its two points: it
keeps `_segment3d` in Python and `Patch3D.do_3d_projection` projects it
as matplotlib's code does (`art3d._draws_itself`).

A 2-D collection converted (`line_collection_2d_to_3d`,
`poly_collection_2d_to_3d`, `patch_collection_2d_to_3d`) after a 3-D
Axes added it is taken out and added again (`art3d._lift_added`): pltrs
(Rust) lifts a collection as the Axes takes it. It is then the last of
the Axes' children, where matplotlib leaves it in its place.

`Axes3D.relim` is matplotlib's 2-D one, inherited: it leaves in
`dataLim`, a box the 3-D axes draw nothing from, the extent of the lines
as last drawn, which is their projection, and nothing of the collections
and patches.

**Values handed back.** Where matplotlib stores what a setter is given
and the getter returns it, the object is the same: `get_markevery()`
answers with the tuple, slice or float given, also through `plot`; after
`update_from` two artists answer with one clip box (the second holds the
first weakly and asks it); `tick_params(reset=True)` makes new `Tick`
objects. Strings and floats that pltrs (Rust) holds (a label, a text, a
line width) come back equal and not identical: keeping them in Python
would put a write on the plain path of every artist for a difference
only `is` shows.

**What stays a copy, and why.** An axis' `get_view_interval()` and
`get_data_interval()` are views of the Axes' boxes in matplotlib; handing
out the boxes' own arrays would build the Axes' transform trees at every
tick computation. `Figure.bbox_inches` changed in its array: matplotlib's
draw does not follow it either (its canvas keeps the old size), only
`get_size_inches()` does. Positions a patch was given as a list or an
array (a circle's centre, an arrow patch's ends) and a `LineCollection`'s
segments given as arrays: keeping every such artist as a record would
cost each program that makes patches from the rows of an array a
comparison per patch and draw. The alpha column of the face colours of a
collection that has an alpha of its own: pltrs (Rust) resolves the
colours with that alpha at each draw, where matplotlib's array keeps
what the program wrote.

This mechanism retires three partial ones: `_program_paths` (copies of
the arrays of the paths a collection was given, compared at each draw),
`_owned_paths` (paths telling their collections and patches when a
vertex array was assigned, not changed) and `_changing_arrays` (an
`ndarray` subclass telling its owner of item assignments, which the
program could see). `_own_state.held_path` keeps what `_owned_paths` did
besides: finding the path an artist keeps by its content, for transforms
of the program's that cache what they compute by path.

### Units

Plotting methods convert values in units (dates, categories, the
quantities of a converter in `units.registry`) before they reach pltrs
(Rust), which only ever sees numbers from them: matplotlib's pipeline is
in Python (`axes/_units.py`: `_process_unit_info`, `_convert_dx`,
`_unit_change_handler`; `Artist.convert_xunits`; `Axis.update_units`,
`convert_units`, `set_units`). The converters pltrs (Rust) has, those of
dates and category names, keep their state in the native axis (its
converter, unit and category names): `Axis.update_units` and
`convert_units` hand their values to it (`axis_values`), and it installs
their tick helpers. Dates cross as date numbers with the zone of the first
(`dates._axis_zone`), which becomes the unit of an axis without one, as
matplotlib's `default_units` makes it; Python keeps the `tzinfo` itself as
`Axis.units`, and `set_units` with a zone hands it over too. Date locators
place ticks on the wall clock of the dates they are given
(`tick_values(vmin, vmax)` passes their zones natively, `_zoned`), not
necessarily the locator's. Any other converter is the program's: its axis keeps
it in Python, `_update_axisinfo` installs its `AxisInfo` natively
(`_tick_helpers.use_axis_info`, `Axis::take_axis_info`), and lines made
from its values keep them (`_xorig`) to convert them again when the unit
changes. An artist made outside any axes with values in units converts
them when it joins one (`_convert_units_for`). As matplotlib's, an
Axes follows a change of its axes' units (recomputing its data limits and
autoscaling) only until it is cleared: its first clear after it is made
disconnects the handler with the axis' callbacks (`Axis::follows_units`),
and a clear keeps the data limits for the next data to replace.

Patches, texts and annotations a program makes with values in units keep
them as given, as matplotlib's do, which convert them each time they draw
or measure (`_unit_values`). Such an artist hands pltrs (Rust) their
conversion when it joins an Axes (its `_convert_units_for`), and then
follows the units of the Axes' axes: an axis whose units change
(`set_units`, `set_converter`) has it convert them again before the
`'units'` callbacks run, so that the Axes' handler recomputes the data
limits from the new numbers, also on Axes whose handler is gone
(matplotlib's patches convert at every draw). A patch keeps every field of
its geometry while any is in units (`keep_geometry`), since matplotlib
converts them together (a rectangle's far edge as ``x + width``), and its
getters answer the values given while pltrs (Rust) holds their conversion
(`_values.InUnits`). A conversion a setter cannot do yet (a new left edge
in dates beside a width still in numbers) waits for the draw or measure,
and fails there as matplotlib's does. The fields matplotlib computes with
as they are, without an Axes' units (a wedge's or a regular polygon's
centre, a fancy box), take what its arithmetic ends in, Python's `float`
of the object (`_kept_geometry.floated`). Nothing of this is paid by an
artist given no values in units: the constructors and setters on the
plain path tell plain values by a set of their types, without a call.

Category data a program gives `set_units` (a `UnitData`) replaces the
names pltrs (Rust) placed (`set_categories`), on the axes sharing the axis
too, and becomes their units object; names the program adds to it in
place (``units.update([...])``) are handed over where pltrs converts or
reads categories next. Masked dates convert as matplotlib's `date2num`
converts them, with its numbers under the mask (seconds since 1970, which
`bar` and `stairs` use, as matplotlib's do), and the conversion keeps the
mask.

A zone crosses as pltrs (Rust) models it where it can (`_date_locators._native_tz`:
the names of `zoneinfo`, pytz and dateutil's files, the fixed offsets that
name themselves as `datetime.timezone` does); any other `tzinfo`
(`dateutil.tz.tzlocal()`, `tzstr`, a class of the program's) crosses as a
`_program_zones.ProgramZone`, which pltrs (Rust) asks what it needs
(`pltrs::dates::ZoneRules`, `TimeZone::Program`) through the zone's own
methods, as matplotlib's `date2num` and `num2date` do (`astimezone` to and
from UTC, `tzname`), the offsets of a locator's ticks in one call and the
names only where a format writes them, so the zone is never asked more
often than matplotlib asks it. Zones compare by identity, as Python's
aware `datetime`s compare theirs. A name pltrs (Rust) finds no zone of (a
POSIX `TZ` string in :rc:`timezone`) is read by dateutil's `gettz`, as
matplotlib reads it (`pltrs::dates::set_zone_finder`).

### Colour mapping

Colormaps and norms are matplotlib's Python classes (`colors`, with the
registries of `cm` built from pltrs (Rust)'s colormap definitions), and a
mappable keeps its Python `Colorizer`. What pltrs (Rust) draws through
them gets a copy: a native function taking a `pltrs::colors::Colormap`
accepts any Python colormap (the colormap of its lookup table), and one
taking a `Box<dyn Norm>` accepts a norm of matplotlib's classes (the
pltrs (Rust) norm of that kind, with its limits; `modules/colors/mapping.rs`).
Any other norm (a subclass of the user's) maps data through the Python
norm itself: pltrs (Rust) calls the callables the package's registered
`native_norm` gives for it (`modules/colors/python_norm.rs`).
A mappable pltrs (Rust) makes (an image) is wrapped, attached to its
colorizer with `_ScalarMappable._attach_colorizer`, and hands the native
side a new copy from `changed()`. matplotlib's draw maps an image's or a
collection's data through the norm object, whose `__call__` gives it the
limits it lacks; pltrs (Rust)'s draw scales its copy, so the mappable
gives the norm object the limits that draw scaled it to when it is next
read (`ColorizingArtist._scale_as_drawn`, from each class's
`_drawn_clim`: an image those of the data it resampled, a collection
those of its array; a 3-D collection counts its axes' draws), as long as
the norm is the one handed over and the mappable was drawn since.

A collection whose colorizer maps several variates (a `BivarColormap` or
`MultivarColormap` with a `MultiNorm`, the values a structured array or a
tuple of arrays; `pcolormesh` of a structured array, as matplotlib 3.11)
gets its colours from the Python colorizer, matplotlib's `to_rgba` over
its norms (matplotlib's, the program's) and colormap, whenever its array,
mapping or alpha changes (`Collection._map_variates`): pltrs (Rust) puts
them on the sides its mapping colours (`Collection::set_program_colors`).
The plotting methods of one variate refuse such a colormap as
matplotlib's do.

matplotlib's collection maps its values when it is drawn
(`update_scalarmappable`), and its getters report the colours of the last
mapping until the next. pltrs (Rust) maps when asked, so a collection
keeps the mapping it last mapped with once its values, norm or colormap
change (`CollectionArtist::mapping_shown`, the old mapping moved there,
not copied, when `set_array` replaces it), for `get_facecolor` and
`get_edgecolor` until the next draw.

### Callbacks and subclassing

Python code runs inside pltrs (Rust) through `Callback`: the native function
takes the callable, and the pltrs (Rust) closure or trait object holds it
(`Callback` is `Send`, as pltrs (Rust) requires of what runs during layout and
drawing). `axis/python_helpers.rs` shows the pattern: the Python formatter
of an axis becomes a pltrs (Rust) formatter calling its methods as
matplotlib's `Axis` does (`format_ticks`, then `get_offset`, through the
adapter `_tick_helpers` makes), handing it pltrs (Rust)'s description of
the axis being drawn, which its `_axis_info()` and its `Axis` answer from
meanwhile.

Python subclasses that override drawing or computation (a `Formatter`,
`Locator`, `Transform`, `Artist` subclass) follow the same route: the
native side is given the Python object as a callable or as an object
whose methods a Rust adapter calls through `Callback`. A Rust value that
must refer back to Python objects that can reach it (an event callback
holding its figure) must hold them weakly, or the cycle runs through Rust
where Python's garbage collector cannot see it.

pltrs (Rust)'s callback traits cannot fail the Python way, so when the
callable raises, the exception is kept in the frame of the native call
that is running (`binding/callback.rs`), the callback returns a fallback
(`None` from `call`), later callbacks of that call are skipped, and the
native function raises the kept exception when pltrs (Rust) returns. Each
native call has its own frame, so a native function a callback calls
cannot pick up its caller's exception. A callback outside any native call
turns its exception into a `RuntimeWarning`. Where matplotlib's Python code catches
one that pltrs (Rust) runs the program's code for, pltrs (Rust) asks
the binding through `pltrs::layout::failures` (installed by
`modules/figure_layout.rs`), which takes an exception of the class asked
out of the frame (`binding::forgive`): a layout engine's pass stops where
the program's code failed, as matplotlib's does; `Figure.draw` passes
over a `ValueError` of that pass and draws without the rest of the
layout; and measuring an axis for a layout measures it again in full
after a `TypeError` (matplotlib's `_get_tightbbox_for_layout_only`).

### What keeps a figure alive

A figure goes when the program forgets it, as in matplotlib: a web
server drawing per request, a notebook kernel or a window redrawn for
hours must not grow. Two rules keep it so.

* **pltrs (Rust) holds no Python object that reaches back to its
  figure.** Python's collector sees the references Python objects hold,
  not those pltrs (Rust) holds (native objects take no part in garbage
  collection), so a cycle through native state is never collected. An
  object of the program's pltrs (Rust) calls is held by the Python object
  of its figure, and pltrs (Rust) holds it weakly: scales, transforms
  artists are given and path effects through `_program_classes.held` (the
  root figure keeps them in ``_held_for_native``, as pltrs (Rust) shares
  a scale among the axes sharing it; an artist in no figure yet keeps its
  own; a `Held` proxy's pickle holds the object itself), tick helpers
  through their adapters (the axis holds them), agg filters through
  `_artist_effects.held_filter` (the artist holds them), and the same for
  those a plotting method's keyword gives (`_live_transforms.given_to_native`
  and the lines' `_native_kwargs` hold a transform for the root figure of
  the Axes, a line takes its path effects through its setter). A norm or a
  colormap mapping in Python is held by the colorizer it was given to, and
  weakly by pltrs (Rust) (`colors._native_norm` and `_native_colormap`; a
  norm read back from an archive, which nothing else holds, strongly).
  pltrs (Rust)'s memo of the answers of the program's maps holds the maps
  weakly (`transforms::program_calls`). What the package makes for an
  artist and keeps (`_live_transforms.kept`: a clip path, a transform) is
  kept on the artist, not in a table keyed by it weakly: what it refers to
  reaches the artist again (a clip path its Axes and figure), which a
  weak key never lets go. Caches keyed by a class of the program's
  hold the class weakly (`_program_classes.class_cache`), and nothing the
  package shares between figures (a class attribute, a module's table)
  may refer to one figure's objects (`Rectangle.get_path`'s shared unit
  rectangle refers to its rectangle weakly). The artist made gives back
  the object given (``line.get_transform() is t``).
* **A removed artist lives only while something reaches it.** pltrs
  (Rust) counts the handles of each artist (`FigureCell`'s census);
  before a draw a figure lets go of the removed artists no handle reaches
  (`FigureCell::release_unheld`). The figure's registry of Python objects
  (`_handles.Registry`) holds the objects of artists in the figure, and
  those of removed ones weakly (swept there when it has doubled since
  its last sweep, if they were out of the figure at the sweep before too,
  as an artist made for the figure is until it is added; at once by
  `Figure.clear`), the object of a removed artist holding those of the
  removed artists it owns; a tick helper registry
  hands the helpers of a removed Axes to the Axes.

`_native.diagnostics.live_counts()` gives pltrs (Rust)'s counts of figure
states, native objects and removed artists kept (`pltrs::diagnostics`).
The leak harness, `python -m leaks` (from `python/tests`), makes, draws,
saves and drops a few hundred scenes under pltrs and under matplotlib and
reports what stays alive or grows under pltrs only, the scenes of
figures that some forty libraries make (`leaks/libraries.py`, in the
venvs of `cargo xtask ecosystem setup`) among them; ``--serve N
[lib_seaborn]`` runs a request loop; ``python -m
leaks.holders SCENE`` shows what holds a leaking scene's objects (on the
Python side: the objects whose references Python's collector cannot
account for, and the chain from a module to them). Its short mode runs
in `cargo xtask python test` (`leaks/test_leaks.py`, against matplotlib's
outcomes recorded in `leaks/reference.json` by ``python -m leaks
--record``).

### Private names other libraries use

Libraries read, write and call matplotlib's private names
(``ax._get_lines``, ``text._color``, ``fig._suptitle = t``). Which ones
is a census, `spec/diff/private_census.json` (`spec/diff/private_census.py`
reads the libraries and corpora installed here with ``ast``), and
`cargo xtask diff --private` measures pltrs against it. The means, by
family:

* **Plain fields** (a private attribute a public getter of its class
  returns as it is: ``Text._color``, ``Line2D._drawstyle``): the table
  `FIELDS` in `_artist_private.py`, written by
  `spec/diff/private_fields.py` from matplotlib's source. A read of one
  misses the instance and goes to `PrivateAttributesMixin.__getattr__`,
  which calls pltrs's own getter of the class defining it (a program's
  override left out; a class of the package whose own getter reads
  the field keeps none, as matplotlib's `NonUniformImage` its
  ``_filterrad``). Fields libraries assign whose setter exists, and which
  pltrs's own code never assigns, get an assignable attribute on the
  class (`__init_subclass__`): read through the getter, assigned through
  the setter, so that the next draw, getter and pickle see it. A new
  field joins by regenerating the table; one state pltrs (Rust) owns
  that no plain getter returns gets a property of its own over the
  native getter and setter, in the class or a mixin of private state.
* **Assignable attributes** (public or private, an attribute a program
  assigns and reads back) have one means, `_values.assignable`:
  ``assignable(getter, setter)`` where the assignment has an effect
  matplotlib's later code sees (a draw, a measure, a pickle), and
  ``@assignable`` on a getter alone (store and read back, `_values.take`
  for the method that reads it once) where matplotlib's code never reads
  it again, or only while the object is made.
* **Private helpers** matplotlib's code runs over public state (``_get_lines(ax,
  x, y)``, ``_set_gc_clip``, ``_set_artist_props``) are ported as they are,
  in a lazily imported reference module when they are long; forwards
  keep matplotlib's signature (``__wrapped__`` set to a function taking
  its parameters, as `axis.py`'s `_AXIS_METHODS`).
* A private name pltrs's own code uses as state of its own must be
  renamed before it becomes matplotlib's (spines keep `_kept_path`
  behind matplotlib's ``_path``); a property never goes where pltrs's
  code assigns the name.

### Axes' axes: tick helpers, ticks and texts

An `Axis` (`ax.xaxis`) reaches its pltrs (Rust) axis through its axes and
its name; pltrs (Rust) keeps everything about it. Two mechanisms give it
matplotlib's objects:

* **Tick helpers** (`_tick_helpers`, `_native.axis` `helpers`): Python
  refers to the locators and formatters pltrs (Rust) uses through tokens
  whose identity stays unique while held; each root figure keeps
  `{identity: (token, helper object, axis)}` (`figure._tick_helpers`), so
  `ax.xaxis.get_major_locator()` is the same object each time, and the
  same for the axes sharing the axis. A locator of `ticker` (or of polar
  axes) set from Python is installed natively from its spec and installed
  again in place when changing one of its attributes changes its spec
  (`TickHelper.__setattr__`); any other helper, formatters included (they
  keep what they computed for the ticks drawn last), is installed through
  an adapter pltrs (Rust) calls. A helper pltrs (Rust) installed itself is
  described by its spec (`ticker/describe.rs`) and becomes the object of
  matplotlib's class; a formatter of those is installed through an
  adapter in place of pltrs (Rust)'s the first time one of its attributes
  changes its spec (`ticklabel_format`, `set_useOffset`), since
  matplotlib's formatters change in place.
* **What ticking leaves in helpers** (`_tick_state`, `_native.axis`
  `labelled`, `labelling`): matplotlib's `_update_ticks` leaves what the
  helpers computed in their attributes (a formatter's `_locs`, `offset`,
  order of magnitude and `_format`, the `_freq` of an `AutoDateLocator`,
  the `ndivs` an `AutoMinorLocator` settles). pltrs (Rust) numbers each set
  of ticks the first time one of matplotlib's tick updates places it
  (drawing or measuring the axis, reading its tick labels, axisartist's
  `get_tick_iterators`; `TickSet::labelled`) and keeps the locations and
  the description of the axis it labelled; the attributes such an update
  sets are `_tick_state.ticked` descriptors, which first make the object of
  a helper pltrs (Rust) ticks with natively do quietly what matplotlib's
  update did with it (`format_ticks(locs)`, a locator's call), once per
  set of ticks.
* **The axis as an artist**: an axis set animated is left out of its
  Axes' draws but printing, and one given an image filter is drawn
  through it, as matplotlib's `Axes.draw` and `allow_rasterization` treat
  an axis (`Axis::set_animated`, `Axis::set_agg_filter`).
* **Mirrors** (`_axis_artists`, `_native.axis` `mirrors`): the ticks
  (`Tick` with `label1`, `label2`, `tick1line`, `tick2line`, `gridline`),
  the axis label and the offset text are loose `Text` and `Line2D`
  artists that a native `sync` fills from the axis when they are handed
  out and that every setter pushes back (`push`): what one tick has of its
  own is what differs from the style of its level, kept as pltrs (Rust)'s
  per-tick properties (`Axis::set_tick_label_props`). The path effects a
  tick label or the axis label was given go apart, only once the mirror
  was given some (`push_tick_label_effects`, `push_axis_label_effects`):
  the text is then laid out and drawn through them in place of
  `path.effects`, as matplotlib's `Text` keeps the effects it was given
  (`axis_layout::rc_with_effects`).

The axes of a 3-D axes (`axis3d.Axis`) are `XAxis` objects, as in
matplotlib: `_native.axis` routes the functions of the tick helpers, the
mirrors, the tick parameters, the label pad and the tight box to the
3-D axis an `Axes3D` native object and a name (`"z"` too) designate
(`axis/owner.rs`, `axis/deep.rs`), and refuses the others
(`NotImplementedError`) rather than act on the projection plane the
object otherwise converts to. A 3-D axis draws every tick with the style
of its level but for the label pad of each major tick, and moves its
texts at every draw, so its mirrors are filled with where the last
layout placed them, positions set on them are not read back (matplotlib's
draw overwrites them too; reading the tick labels after a draw moves
their labels back to their locations along the axis, as matplotlib's
`_update_ticks` does), and what a single tick would keep of its own
is refused. A 3-D axis is ticked once per layout; the warnings
matplotlib's tick updates issue (numpy's about the tolerance of the minor
ticks of a view outside the scale's domain) are issued at matplotlib's
sites instead: each draw of the gridlines and of the ticks of an edge, each
placement of a title (the y axis), each tight box, each read of the tick
labels or the minor locations (`mplot3d/tick_sites.rs`). Values in units plotted on 3-D axes pass as they are
(`Axis._native_units`): pltrs (Rust) converts them, and axes sharing an
axis share its units but not its converter, as 2-D ones do
(`Axis3D::own_converter`).

matplotlib's `Tick` objects sit at 0 without a label until
`Axis._update_ticks` places them, which drawing or measuring a shown
axis and reading its tick labels do; the ticks of a hidden axis stay
there. pltrs (Rust) records whether an axis' ticks were ever placed
(`Axis::ticks_placed`; `_native.axis.place_ticks` for what Python runs),
and the mirrors of ticks never placed sit at 0 without labels. The
lines of the mirrors hold the data matplotlib's ticks hold
(`Tick._place_lines`): what their class makes them with, then `(loc,)`
along the axis once a tick update placed them, while their native lines
keep what pltrs (Rust) draws. The gids
of an axis, of its texts, of its ticks and of their parts (mirrors push
theirs) are the axis' too (`AxisGroup`), and it draws matplotlib's SVG
groups with them: `matplotlib.axis`, then `xtick`/`ytick` holding a
`line2d` per visible line and a `text` per non-empty label, then the
label and the offset text; a 3-D axes draws its background and the
`pane3d`, `grid3d` and `axis3d` groups of its axes before its own group.

An axis whose locator, formatter or scale is Python's (a class of the
program's, a `FuncFormatter`, a function scale, a secondary axis, the
colorbar of a norm of the program's) is ticked by the program, as
matplotlib's `Axis` ticks it, wherever matplotlib does: pltrs (Rust)
models matplotlib's call sites of `Axis._update_ticks` (`Axis.draw` and
`get_tightbbox`, the label placement's siblings and spine, the title
placement, `Axes.get_tightbbox`'s spines, the layout engines' passes,
the draw before a print; `axes/tick_sites.rs`) and calls the figure's
`TickUpdater` there (`axes/tick_updates.rs`), which `_tick_updates`
installs once a figure has such an axis: it runs matplotlib's
`_update_ticks` with the program's objects, so that the program's code
is called as often, in the same order and with the same arguments.
pltrs' own helpers are computed natively where pltrs needs them, as
before; the ticks the program gave stand while asked for at the same
geometry. An axis whose class of the program's answers
`get_view_interval` itself (skewt's) is ticked by the program too
(`AxisState::program_view`): the package's locators and `ScalarFormatter`
ask the axis for its view interval where matplotlib's do, and autoscaling
asks it whether it is inverted (`TickUpdater::inverted`), as
`set_xbound` does.

A scale of the program's (a function scale, a scale class of the
program's, a secondary axis' functions) is called where matplotlib's
artists map through it, at the same sites (`axes/transform_sites.rs`,
walked with the tick updates): every tick label measured
(`_get_ticklabel_bboxes`) and drawn, every tick line and gridline drawn
(its path, then its points), every spine measured and drawn, the view
limits mapped once while they stay (`TransformedBbox(viewLim,
transScale)`, `AxesState::scale_box`), and a line's path kept while its
data and transform stay, mapped anew at every draw for a line drawing a
slice of its points (`lines/program_path.rs`, matplotlib's
`TransformedPath`), and a mesh's coordinates mapped at every draw
(`QuadMesh.draw`). Each call hands the arrays matplotlib hands: the
coordinates of the points a blend of `transData` and `transAxes` maps,
a row where drawing maps them through the scale's `transform_non_affine`,
a column where matplotlib calls `transform` (autoscaling, the tick
updates, the function of a `functionlog` scale;
`_scale_calls.ScaleCalls`), and a masked row for a mesh's coordinates
(`ArrayLayout::MaskedRow`). pltrs
lays out and draws the same parts its own way meanwhile, needing the
same values in other batches: that work runs quietly
(`transforms/program_calls.rs`), the program's maps answering it from
what they answered since the layout began, so that the program sees only
matplotlib's calls. A curvilinear grid is computed where matplotlib's
grid helper computes it, when a floating axis is made
(`AxisArtist._init_line`, `_native.axisartist.init_line`, with the figure
free), and again only for other limits.

The transforms of a projection of the program's are called at the same
sites (`axes/projection_sites.rs`): each tick label measured and drawn
maps its point through the label transform of its side (the adapter's
`tick_label` gives it evaluated, and pltrs maps each tick's point
through it), each gridline and tick mark its path, then its points. The
maps of the plane the adapter evaluates are kept per transform and view
(`_Adapter._kept`), so that the program's answers are kept by the Python
object they call (`PointMap::program_key`, `_answers_alike`) and pltrs'
own quiet work answers from them.

A norm of the program's (`PythonNorm`) is called as matplotlib's
colorbar and mappables call it: a collection autoscales it once, when it
is made (`_scale_norm`), and normalises its array, as a masked array of
the collection's shape, at every draw (`update_scalarmappable`,
`Mapped::normalize_drawn`; walked as a site of the axes' draw,
`program_collection_drawn`, which the paint then uses); a colorbar
autoscales it to its mappable's values in each `_process_values`,
normalises its boundaries in `_mesh` and `_do_extends`, the values of
its extensions alone (`Norm::normalize_scalar`), and its boundaries
again wherever matplotlib's `_ColorbarAxesLocator` runs (`colorbar_located`,
where the aspect is applied: `_get_draw_artists`, `Axes.draw`, tight
boxes). A norm made from a scale (`norm._scale`, a `FuncNorm`'s
functions) scales the colorbar's long axis with that scale, which calls
the program's functions; another norm, with a function scale through the
norm itself. The arrays are those matplotlib hands over
(`Norm::normalize_shaped`, `ArrayForm`): plain for boundaries and axis
values, masked for a mappable's array and a mesh's coordinates.

Axes sharing an axis hold the same `Ticker` objects, those of the first
Axes of the chain they share from (`Axis._take_shared_tickers`), as
matplotlib's `sharex` hands them over; what they read through them is
the helpers pltrs (Rust) shares among them, bound to the axis they were
set on last (`_tick_helpers._locator_axis`).

While pltrs (Rust) lays a figure out it works on the figure's state taken
out of its cell (`FigureCell::work_on`, `figure/lend.rs`), and lends it
back to the cell for each call of the program's tick updates
(`FigureState::lend`): the program finds the figure free, as the layout
left it, reads it (limits, ticks, positions) and changes it at once,
and the rest of the layout sees the change, as in matplotlib. Only the
thread laying the figure out finds it free meanwhile.

While pltrs (Rust) paints a figure (draws its artists), the figure's state
stays in its cell, lent to the painting thread (`FigureCell::paint`,
`figure/painting.rs`), and the drawing borrows it only for the moments it
reads it: a `DrawContext` borrows it when first asked about it and lets it
go whenever it hands its renderer out or is told to (`DrawContext::free`),
which takes it mutably, so that nothing read from the figure is held
meanwhile. Nor is the artist being drawn borrowed from the figure: the
figure keeps its artists shared (`artist/kept.rs`), the draw holds the one
it draws, and a change made to an artist a draw holds is made to a copy,
which the figure keeps from then on, while the draw finishes that artist
as it found it (so every artist is `Clone`: `ArtistClone`, and every
projection, `ProjectionClone`). So the program's code a draw runs where
it calls a renderer (a path effect, a renderer of the program's drawing
an artist, the TeX painter) or after letting the figure go (a replaced
`Text.draw`, a `draw` override recorded while the figure draws) finds the
figure free: it changes it at once, reads its changes back, may make
changes returning what they changed (`set_title`), and the rest of the
draw sees them, as in matplotlib. A figure being painted counts as laid
out: an extent measured meanwhile is that of its layout
(`FigureCell::is_painted`).

The program's code that maps values for a draw runs with the figure free
too, at each call site: what the call needs is gathered (a transform
resolved, a text's wrapping box, style and resolution, a box image's
canvas), the figure let go, the call made, and the draw goes on with its
result. A transform knows whether applying it runs the program's code
(`ResolvedTransform::calls_program`, from `AxisMap::calls_program` and
`PointMap::calls_program`), and `DrawContext::free_for` lets the figure
go before such a transform maps a line, its markers, a gridline, a tick
mark, a spine or a text's position; the texts of an axis (tick labels,
the axis label, the offset text, and the spines they clear) are laid out
through a `FigureReader`, a draw's context letting the figure go around
the transforms placing them, a projection of the program's placing a
tick label, and the text measurer of the program's
(`text::has_line_measurer`), a layout's context holding it throughout. An
image lets the figure go before it maps its data through its norm and
transform (on a copy of its mapping, put back after: a norm of the
program's may change the image it maps, which copies it), a box image
before a function of the program's gives its box. Autoscaling lends the
figure to the program's locator and scale while a layout works on it
(`FigureState::lend`), as matplotlib's `autoscale_view` runs them in the
draw. An axis maps values through its scale apart from the figure
(`Axis::scaled`, which the tick updates' transforms call). Draws without
code of the program's pay nothing: a transform calling none of it keeps
the figure borrowed.

The program's code that makes or maps a patch's outline (a transform or
a box style of the program's) runs with the figure free: a draw lets it
go first (`PatchParts::program_outline`, `PatchShape::MAPS_OUTLINE_ALONE`),
and the data limits of an artist added to an axes are measured apart from
the figure (matplotlib's `_update_patch_limits` makes the path first,
calling a box style even for a patch in axes fractions). A projection's
data ratio is asked with the figure lent (`FigureState::data_ratio_of`,
`Axes::apply_aspect` working on the figure apart), and so are the spans of
its spines (its `Spine._adjust_location`) once a layout settled the view,
and again once the figure is laid out for a draw, asked anew while the
program moves the axes answering (`FigureState::span_program_spines`):
the layout and the draw use what it answered for that view
(`AxesState::spine_interval`). A tight box maps the
positions of the texts it measures through a transform of the program's
with the figure lent, then measures them quietly
(`program_texts_measured`).

Other code pltrs (Rust) runs while it lays a figure out or draws it with
the figure held finds the figure in use: taken out while it is laid out,
borrowed for reading while it is drawn. That is the code a transform's
resolution runs (the hooks of a projection of the program's other than
its data ratio and its spines' spans: its transforms evaluated, its
`get_view_interval`, its `Tick.draw` probe), the grid finders of
axisartist where an axis artist or its gridlines draw (but
`AxisArtist._init_line`), and texts measured through transforms of the
program's outside tight boxes. matplotlib's may still change it, so a
native call made from such code (any native call made while another
native call of its thread runs) runs inside `pltrs::figure::reenter`,
which stops it where it reaches the figure in use, instead of letting the
`RefCell` panic (`binding/function.rs`, `binding/reentrant.rs`):

* a native function returning nothing (a change: `set_color`, `set_text`,
  `set_visible`...) is queued with its arguments and called again as soon
  as the figure is free: when a draw has drawn the figure's artists, before
  the figure counts as drawn and its draw event is sent
  (`pltrs::figure::set_draw_end_handler`), else when the thread's outermost
  native call ends. Its exception is raised from the native call drawing
  (or ending). Each change is made as at the tick of the thread's draws it
  was asked for at (`pltrs::figure::replaying_changes_from`): an artist
  changed while it was drawn ends fresh, as the end of matplotlib's draw
  of it leaves it, one changed after it was drawn stale. The same call
  queued again (a path effect run for every marker) takes its last place;
  changes queuing others are made in rounds, which end when a round queues
  exactly itself again (a `relim` running a transform that calls `relim`)
  and raise `RecursionError` after 64;
* any other native function raises `RuntimeError` saying the figure is in
  use: a read while the figure is laid out (a formatter calling
  `ax.get_xlim()`), or a change returning what it changed (`set_title`
  returns the title) while it is drawn or laid out.

Reads while the figure is drawn work, so reading a view limit borrows the
figure for change only when autoscaling is pending
(`FigureState::view_is_stale`). Event callbacks (`draw_event`, limit
changes), `draw` overrides and the program's axes locators run with the
figure free, as in matplotlib.

A draw started from a callback during a draw: a callback that
matplotlib's draw calls (a formatter, a locator, a transform, a norm, a
path effect, an artist's `draw`, an `xlim_changed` callback of limits the
draw autoscales: librosa's `waveshow` redraws its canvas from there) may
draw the canvas (`canvas.draw()`, `draw_idle()`, which the Agg canvas runs
at once, `fig.draw(canvas.get_renderer())`). matplotlib then paints the
whole figure, as the callback finds it, into the buffer of the draw under
way (a PNG save draws into the canvas' renderer too), clearing it first,
and the draw under way paints what it had left over that: the rest of
the Axes being drawn and every artist after it come out twice, and a
state halfway through autoscaling (y limits still 0 to 1) shows through.
pltrs (Rust) calls most of these callbacks while it lays the figure out,
before it paints anything, and paints the figure once, as its final state
says: the nested draw runs on the figure lent to it and paints the
canvas' own buffer, not the save's. Matching matplotlib would take a
paint target shared with nested draws, an outer paint resuming at the
point of matplotlib's draw each kind of callback is called at, and nested
layouts of the states halfway; it was decided against (task ROBUST3): the
doubled image is a glitch no program asks for. What the callback reads,
the state the draw leaves and every later draw are matplotlib's (the
`reentry` generator's `xlim_changed-stale` kind and `--draw_idle-at-save`
programs; the image difference is in `spec/diff/allowlist.txt`). A
`fig.draw(renderer)` from code pltrs runs while it paints (a path effect,
an artist's `draw`) paints into that renderer, as matplotlib's does.

A draw stops where the program's code it runs before painting fails, as
matplotlib's draw stops at an exception: a layout hook or draw hook of the
package's that raises (`DrawHook::stopping`) leaves the figure laid out
but its artists undrawn.

matplotlib accepts data its 3-D artists cannot project (a line through
points of two coordinates, segments of 2-D points, positions whose
coordinates differ in length) and 2-D collections and patches in a 3-D
axes, which lack the `do_3d_projection` its draw asks them for, and fails
only when it draws them. The package runs matplotlib's own computations
of such data when it is given them (`mplot3d/_draw_problems.py`, ported
from `art3d` and `proj3d`) and records the exception they raise with
pltrs (Rust) (`Axes3D::set_draw_failure`, `pltrs::mplot3d::DrawFailure`),
keeping the data as given: the axes' draw reports it where matplotlib's
meets it (the collections and patches projected after the background,
the lines and texts drawn among the other artists), leaves the artist
unprojected and undrawn, and projects no line of a draw ending at a
collection; `do_3d_projection` raises it, and runs the same computations
through the projection of the last draw (`M`), failing as matplotlib's
does before the first draw.

A `Line3D` holds two kinds of data, as matplotlib's: its 3-D data, and
the 2-D data of `Line2D`, which each draw replaces with the projection.
2-D data the program sets (`set_xdata`, `set_data`) are what the line is
measured by until the next draw (`get_path`, `get_bbox`, extents):
`Line3D._numbers` hands them to pltrs (Rust) while they are newer than
the last draw (`_data_set_at`). matplotlib's draws convert no 2-D data,
so data that converts to no numbers leaves the projection in place.

An artist a figure holds without matplotlib's giving it the figure (a
child an `AuxTransformBox` takes once it is in a figure, whose
`add_artist` sets no figure) is *figureless* (`ArtistBase::figureless`):
its `get_figure()` is None, and measuring or drawing a text among them
raises matplotlib's `AttributeError` (`OffsetBoxError::NoFigure`).

The parasites of an axes_grid1 host whose view limits follow the host's
(`twin`, `get_aux_axes(viewlim_mode=...)`) take them when the host's
limits change, as matplotlib's `_sync_lims` does on the host's
`xlim_changed` and `ylim_changed` (`FigureState::sync_parasites`); between
two changes a parasite keeps limits of its own. A parasite without an aux
transform has a box of its own, as matplotlib's (whose transforms are its
own; one with an aux transform takes its host's `transAxes`): its own
position, the host's box from the host's draw (`HostAxes.draw` gives it
`apply_aspect(rect)`) until `apply_aspect()` brings it back, as every
measure of its tight box does (`Parasite::active`); its ticks, tight box and
transforms between draws are those of that box. The positions given
(`set_position`, `set_subplotspec`) are kept by the Python object for its
getters as given.

A draw matplotlib makes before printing (with a layout engine, or to cut
the figure to its tight box) runs no renderer: what a print renderer notes
while drawing (the PostScript backend's partially transparent colours,
logged as drawn, so also when an artist fails later) is not noted when
that earlier draw fails first (`Figure::render_vector_noting`).

Some parts matplotlib keeps as artists of their own are drawn by pltrs
(Rust) as parts of their artist: a text's box (`Text.get_bbox_patch()`)
and the arrow of an annotation (`Annotation.arrow_patch`) or of an
annotation box (`AnnotationBbox.arrow_patch`, matplotlib's own patch
until the box joins a figure, then the mirror). Those are mirrors,
as the ticks of an axis are (`_text_box`, `_annotation_arrow`): loose
`FancyBboxPatch` and `FancyArrowPatch` objects filled from the part's look
(`_native.text.bbox_patch`, `_native.annotation.arrow_patch`,
`_native.offsetbox.annotation_arrow_patch`) whose setters give the part
their changes (`restyle_bbox`, `restyle_arrow`); the arrow's
`get_window_extent` is where the annotation draws it, and
`Text.get_window_extent(annotation)` the text alone. The box's patch
moves where each draw of the text placed it (its path and transform
follow the text's count of draws), as matplotlib's draw places it.

Mirrors are objects of matplotlib's very classes (`type(ax.set_xlabel("x"))
is Text`), not subclasses: `Text`, `Line2D`, `FancyBboxPatch`,
`FancyArrowPatch` and `FontProperties` are made `_mirrors.mirrorable` (or
`pushing`), so that every setter of an instance attached to a part (its
`_mirror`, a `_mirrors.Mirroring`) pushes the instance back into the part,
and the extent and figure are asked of the part. A text's `FontProperties`
is such a mirror of the text's font, the same object while the font stays.

`ContourLabeler.clabel` and `add_label_near` place every label through
the Python method `_split_path_and_get_label_rotation`, as matplotlib's
`labels` loop does: pltrs (Rust) places the labels and calls it (through
the callable the Python side hands `_native.contour.clabel`) with each
line, vertex index, position, width and spacing; the method returns the
rotation and the cut line pltrs (Rust) computes (`_native.contour.split_path`).

### Staleness

pltrs (Rust) keeps whether each artist changed since it was last drawn
(`ArtistHandle::stale`): drawing an artist makes it fresh, and any change
through its handle, or adding or removing one of its children, makes it
stale again together with the artists containing it and the figure,
unless it is animated (`figure/stale.rs`). `Artist.stale` reads and sets
it, so matplotlib's propagation through `stale_callback` needs no Python.
A `draw` matplotlib leaves stale (an annotation whose point is clipped,
an inset indicator) is one whose artist answers `false` to
`Artist::freshened_by_draw`: the draw is counted, the artist stays stale.

What matplotlib draws as a part of another artist and pltrs (Rust) draws
with it (an `Axis` with its Axes, a table's cells with their table, the
boxes inside an offset box with the box added) reads as fresh once the
artist drawing it drew since it went stale (`_drawn_with.DrawnWith`,
through `_native.artist.draw_count` of its `_drawer()`).

A program's own `stale_callback` (on any artist, the figure included)
sees the changes pltrs (Rust) makes, which set no `stale` in Python: the
root figure of the artist it is put on reports (`Figure::report_stale`,
`_artist_staleness.watch`) the artists its changes mark stale, the
innermost of each change, or the figure itself when it went stale with
none (a scale, the subplot parameters, its size). At the end of the
thread's outermost native call the binding hands them to
`_artist_staleness.stale_reported`, which sets each stale as matplotlib's
setters do, running the chain of callbacks up to the program's. What
the program sets stale itself, and what a mirror hands its part
(`_mirrors.push`: matplotlib's part has no callback of its own), is not
reported (`_artist.unreported`). Every change that stales the figure calls its
callback at least once; how often matplotlib calls it (its `set_title`
11 times, each draw dozens of times as ticks update) is not followed.
Setters matplotlib leaves fresh (`gid`, `url`, `picker`, `rasterized`,
`in_layout`, `navigate`, a text's `wrap` and `bbox`, a collection's line
and cap styles) leave pltrs (Rust)'s artists fresh too.

### Warnings

pltrs (Rust) issues its warnings through `pltrs::warnings` (data cast to
a narrower type to be drawn, say). The extension sends them to
`_api.warn_external`, which `_api` registers, so they are matplotlib's
`UserWarning`s; one that the warning filters turn into an exception
raises from the native call that issued it, as a callback's exception
does. Where matplotlib's numpy arithmetic warns (`divide by zero
encountered in log10`, isclose's invalid tolerance), pltrs (Rust) does
the arithmetic itself and issues the same warning in the same case with
`warnings::warn_runtime`, which the extension issues as numpy does, a
`RuntimeWarning`.

### Warnings issued while pltrs (Rust) works

Some of matplotlib's warnings come from deep inside a draw or a layout,
where pltrs (Rust) cannot call Python: "Glyph 128579 (\N{UPSIDE-DOWN
FACE}) missing from font(s) DejaVu Sans." is issued for every line with a
character no font has that a renderer measures without finding it in its
cache of line metrics. pltrs (Rust) queues such events on the thread
(`pltrs_text::take_missing_glyphs`, noted where pltrs (Rust) measures a
line its cache does not hold: `pltrs::text` `font_notes`), and every
native call ends,
when it succeeds, with the functions `modules::register` installed
(`binding::after_call`), which drain the queues and issue the warnings
through the package's own functions (registered with `_native.register`,
here `_text_helpers.warn_on_missing_glyph`, which calls
`_api.warn_external`). A warning turned into an error by the warning
filters is raised in place of the call's result.

The font manager's log messages about the fonts of a text (a family no
font has, a face of another weight than asked for) come the same way:
pltrs (Rust) records each lookup of a text's fonts matplotlib would log
about (`pltrs_text::take_font_lookups`) at the moments matplotlib's
renderers look them up (`_find_fonts_by_props`): when a text is laid out
(`Text._get_layout` takes the height of its font), when one of its lines
is measured uncached, and when one is drawn (`draw_text`'s
`_prepare_font`); tick labels are laid out once more where an axis places
its label (`_update_label_position`). The after-call hook hands each
lookup to `_text_helpers.report_font_lookup`, which looks the fonts up
again with the program's `fontManager` (`_find_fonts_by_props` or
`findfont`), logging as matplotlib's does: once per font properties for a
substituted weight (`findfont` caches), at every lookup for a family not
found. Two more kinds of lookup go the same way. The texts a PostScript or
PDF renderer measures and draws in AFM fonts (`ps.useafm`,
`pdf.use14corefonts`) are looked up among those fonts
(`LookupKind::Afm`; `findfont(fontext="afm", directory=...)` as
`_get_font_afm` does, and for the PDF core fonts `_find_fonts_by_props`
too, as `fontName` does), every one, since what the font manager says of
an AFM font (`Failed to find font weight normal for Helvetica, now using
medium`) is its to find out. And while the deprecated `text.kerning_factor`
is set, every lookup is handed over (`pltrs_text::keep_every_lookup`, set
with the rcParams), and the reporter opens the fonts found as
matplotlib's `get_font` does: `FT2Font` then warns of the deprecated
`_kerning_factor` for each font `_get_font` has not cached, the Last
Resort font behind it included.

### Warnings of matplotlib's transform graph

matplotlib's transforms compute lazily, with numpy: a node makes its
matrix when it is first asked after a leaf changed and keeps it until the
next change (`BboxTransformFrom.get_matrix`, a blend's `get_affine`), or
makes it at every call (`CompositeGenericTransform.get_affine`, which
`transData` is: two `np.dot` each time), or is itself made anew each time
it is asked for (a rectangle's `get_patch_transform`: a `BboxTransformTo`
and an `Affine2D` translated in place). Where a leaf is no ordinary
number (view limits without width, an infinite rectangle, a tick pad of
`inf`), that arithmetic warns (`invalid value encountered in dot`, `...
in scalar add`, `divide by zero encountered in scalar divide`), once for
each node computed. How many warnings a statement gives, and of which
operation, follows from which nodes matplotlib computes there: its
caching is part of its behaviour.

pltrs (Rust) draws from its own state and computes none of these nodes.
The package keeps matplotlib's graph over pltrs (Rust)'s leaves (see
Transforms above), with matplotlib's invalidation, so the warnings come
from **evaluating that graph where matplotlib evaluates its own, with the
calls matplotlib's code makes there, in its order** (`_transform_replay`):

* **The arithmetic warns as numpy's.** `Affine2D.translate`, `scale` and
  `rotate` are matplotlib's numpy code. The matrices `_native.transforms`
  makes for the nodes (`compose` for `np.dot`, `transform_from`,
  `transform_to`, `transform_between`) issue numpy's warnings for the
  floating-point exceptions the same operations raise
  (`pltrs::numeric::numpy_arith`): a product's as OpenBLAS raises them
  (its fused sums hide `0 × inf` behind a sum that is NaN already), none
  where numpy's products raise none (`warnings::PRODUCTS_WARN`).
* **The nodes cache as matplotlib's.** `CompositeGenericTransform` keeps
  its composed affine part for speed, but only while both factors are
  tame (`_native.transforms.tame`): a product that may warn is made at
  every call, as matplotlib makes every product.
* **The leaves invalidate as matplotlib's.** A leaf pltrs (Rust) hands
  over invalidates what is built on it when its value changed (NaN
  counting as itself), and the view limits whenever they were set, to the
  same values or not (`AxesLeaves::view_sets` counts the sets), as
  matplotlib's `viewLim.intervalx = ...` does in `set_xlim` and
  `set_view_interval`. Handing leaves over evaluates nothing: a node
  computed there would be found computed, and silent, where matplotlib's
  draw first asks it.
* **The evaluations are those of matplotlib's code at each site.**
  `add_patch` and `relim` run `_update_patch_limits`' search of the
  patch's transform for `transData` (`patch_limits_updated`; plotting
  methods whose patches pltrs (Rust) adds say so,
  `_transform_trees.patches_added`). A
  draw hands what pltrs (Rust) draws, as it reaches it, to the figure's
  evaluation hook (`Figure::set_evaluation_hook`,
  `pltrs::transforms::Evaluation`): an Axes starting its draw (a polar
  one fits its wedge through `transWedge`), its background, each artist
  (`Patch.draw` makes the transform and takes its affine part,
  `Line2D.draw` takes it once for the line and once for the markers
  through the line's `TransformedPath`, `Text.draw` maps its position,
  `Collection.draw` freezes its transform and hands the offset transform
  over as a matrix; each reads its clip path where a renderer would,
  logging one that is not finite), and each axis (`Axis.draw` places the
  ticks, measures the labels of those it draws, which pltrs (Rust) names
  with their parts and pads, draws each, measures the labels again and
  the spine to place its label). An artist the draw never reaches (one
  after the draw's first failure, a hidden one, any of a figure whose
  layout already met the problem matplotlib's draw raises first)
  evaluates nothing, and one a draw method of the program's draws twice
  evaluates twice.
* **Only the graph is evaluated, on no data.** Paths and points go
  through the transforms empty: the non-affine parts then compute
  nothing, and what they say of the data (the logarithm of a negative
  coordinate) stays with pltrs (Rust), which maps the data and issues
  those warnings itself. Polar Axes are the exception: the positions of
  their ticks go through for real, since what numpy makes of them depends
  on the Axes' own settings (labels at an infinite angle), of which pltrs
  (Rust) says nothing.
* **One voice.** Where a patch recomputes its geometry ahead of
  matplotlib's computation with it (`_unusable.check_ahead`, in its draw
  and in `add_patch`) and that computation is replayed, the recomputation
  is silent: the replay says what the arithmetic warns of.

**Nothing is replayed while every leaf is tame**: a finite number of
ordinary size, a view with some width (`pltrs::transforms::is_tame`,
`is_tame_span`). A draw looks at the figure's leaves once
(`FigureState::graph_is_wild`: sizes, positions, view limits through
their scales, tick pads, a polar projection's settings) and at each
patch's own transform (`Artist::has_wild_transform`), and only a figure
with a wild leaf, or a wild patch, reaches the hook; `add_patch` and
`relim` ask `_native.transforms.replays` first (`_transform_trees.patch_added`
and its kin, which load `_transform_replay` when it is first needed, so
that a program without such a figure never imports it). A tame graph raises no
exception whatever is computed, so skipping it loses no warning; it
leaves the nodes uncomputed, which is where matplotlib's are once a leaf
turns wild, since that change invalidates every node built on the leaf.

**Measuring is replayed like drawing.** matplotlib's
`Axes.get_tightbbox` evaluates the graph as it measures: each axis
(`Axis.get_tightbbox`: the ticks placed, the axis label placed past their
labels, the labels and the axis label measured), the titles' placement
(`_update_title_position` measures the x axis of each Axes the titles must
clear that has its decorations on top, then for each title with a text the
y axis of each: in every draw too), then each artist that takes part in
layout and is not clipped to the Axes (a spine's path, a text's position,
an annotation's point, an image's extent, a legend's search for the best
location). pltrs (Rust) hands these over where it takes the tight box of
an Axes, the figure lent to the hook meanwhile
(`FigureState::measuring_evaluated`, from `axes_tight_bbox_at` and
`tight_bbox_of_axis`): the axes it measures as `Evaluation::Axis {
measured: true }`, those of the titles, then `Evaluation::Measured`,
whose artists the package finds itself (`get_default_bbox_extra_artists`).
So every caller evaluates as matplotlib's does, as often: `get_tightbbox`
of an Axes, an axis or a figure, a tight print (matplotlib draws, measures
and draws: the draw standing for both gives each evaluation twice), the
layout engines (constrained layout measures every Axes in each of its two
passes; tight layout once, and again to fit a rectangle, which pltrs
(Rust) repeats when the measurement is replayed). A measurement asks only
the leaves of the Axes measured (`FigureState::measurement_evaluated`: the
figure's size, the Axes' own leaves and place; not its artists' own
transforms, so that measuring costs an Axes of many artists no look at
each), and `add_patch` and its kin those of the artist's Axes and the
artist's own transform (`evaluates_wild`): an artist goes through the
transforms of its own Axes, and what another Axes rests on costs it
nothing. The methods of the package that measure one artist where
matplotlib's code evaluates (`Text.get_window_extent`, a legend's
`get_window_extent` and `get_tightbbox`) ask the same way
(`_transform_trees.measured`).

Artists drawn or measured lazily in the package keep their own count of
draws: an annotation is positioned, and a quiver's transform made, once
per draw, by the replay for the draw running
(`Annotation._positioned_by_draw`, `Quiver._transform_by_draw`), where
their getters make them for the draws that ended.

**Pickled figures keep what they computed.** matplotlib pickles its nodes
with their matrices and validity, so an unpickled figure computes anew
only what the pickled one had not, and what `Figure.__setstate__`
invalidates by setting the resolution again (everything built on
`dpi_scale_trans`). The package does not pickle its trees (they are
rebuilt over pltrs (Rust)'s leaves), but the state of each owner says
which of its nodes were valid (`_transform_trees.without_trees`, in an
order that follows from how the trees are made), and the rebuilt trees
compute those quietly, but for the ones resting on the resolution
(`_restore_validity`).

Not replayed: the first draw of a tight print before its measurement (the
replay measures first, then gives each evaluation of the draw twice: the
counts agree unless the figure's nodes were invalid before the print),
and what matplotlib's `AxisArtist` evaluates measuring.

### Errors

Each module file implements `From<pltrs error> for PyErr`, choosing
matplotlib's exception class and, as far as pltrs (Rust)'s messages allow,
matplotlib's message (`RcError::UnknownKey` becomes matplotlib's
`KeyError: '<key> is not a valid rc parameter (see rcParams.keys() for a
list of valid parameters)'`). Tests match messages with `pytest.raises(...,
match=...)`, so message fidelity counts: where pltrs (Rust)'s message differs
from matplotlib's, prefer fixing the message in the Python-facing `From`
impl, or in pltrs (Rust) when its own message is the problem. Exceptions that
matplotlib defines in Python, such as `ImageComparisonFailure`, are
registered by the package (`_native.register`) and raised through
`binding::registered_error`.

What a program's data can make enormous (a canvas, an encoded file, a
copy of either) is allocated fallibly (`pltrs_core::alloc`): memory the
system refuses, or a canvas beyond `pltrs_raster::MAX_PIXELS`, is an
error caused by `OutOfMemory`, which the binding raises as matplotlib's
allocations do, `MemoryError`, instead of the process ending.

matplotlib's renderers read and format the arguments of their drawing
methods and graphics state setters in Python before they write anything;
pltrs's renderers do so too before pltrs (Rust) draws
(`backends/_pdf_arguments.py`, `_ps_arguments.py`, `_svg_arguments.py`,
`_backend_agg.check_renderer_arguments`), so that wrong arguments fail
with matplotlib's errors, and a vector renderer draws a path collection
too small to reuse its paths path by path through
`RendererBase.draw_path_collection`, as matplotlib's do. The draws of the
package's artist classes refuse what is no renderer as matplotlib's
rasterization wrappers do (`_artist_subclassing._refuse_renderer`).

Problems met while drawing are not raised by the artist meeting them:
pltrs (Rust) draws the rest and returns them all (`DrawErrors`), each with
the kind of exception matplotlib raises for it (`DrawErrorKind`). The
binding has draws end at their first problem instead
(`pltrs::artist::set_draws_stop_at_first_problem`), as matplotlib's draw
ends at its first exception with the artists after it undrawn (and does
not spend the time drawing them), and raises that problem as matplotlib
would have: a
`ValueError` with its message for a value of the program's that cannot be
drawn (malformed mathtext, an infinite line through two identical
points), else a `RuntimeError` naming them all. `draw_without_rendering`
draws with a renderer that paints nothing, so it raises them too.

**An external program that is missing** fails where matplotlib fails, with
its exception: TeX for usetex text is `RuntimeError("Failed to process
string with tex because latex could not be found")` from every path that
typesets (`TexError::Missing` through `From<TextError>`, and the draw error
kind `Library`), a TeX file kpathsea cannot find `FileNotFoundError` in
`dviread`'s words, the PGF backend's TeX system its `RuntimeError` (the PGF
canvas' `draw` starts it where matplotlib's draw measures a text). A
renderer's `draw_tex` typesets before it reads its other arguments, as
matplotlib's does. A distiller of `ps.usedistiller` missing Ghostscript or
`pdftops` is skipped with matplotlib's logged warning (`_try_distill`),
the file written undistilled. `python/tests/test_allow1.py` checks each in
a process whose `PATH` holds none of these programs.

### What a draw did before it failed

matplotlib's draw stops at the first artist that raises: what the artists
before it logged or warned of was given. pltrs (Rust) draws the same
artists in the same order and reports the failure when the native call
returns, and the package drops the callbacks of a native call once one of
them raised (`binding::callback`), so a renderer's notes on what it drew
(the PostScript renderer's "does not support transparency", once per
document) would be lost with a failure a Python painter reported. They
are given as work done before the failure
(`pltrs::warnings::as_done_before_a_failure`, which the binding runs with
the failure set aside).

### Text measured and drawn by the program

matplotlib measures every line of text through the module function
`text._get_text_metrics_with_cache` and draws every text through
`Text.draw`; its tests replace both (`monkeypatch.setattr`, the
`text_placeholders` fixture) to turn texts into boxes. `_text_overrides`
makes such replacements take effect: it hands pltrs (Rust), once, a
`pltrs::text::LineMeasurer` (every line it lays out, tick labels
included, is measured by calling the module function; the font height
metrics come from `FT2Font.get_sfnt_table` as in `Text._get_layout`) and
a `pltrs::text::TextPainter` (every text it draws is drawn by calling
`Text.draw` on a stand-in `Text` in display coordinates, with a renderer
whose `Artist.draw` queues artists that pltrs (Rust) then draws in the
text's place), each with the attribute it stands for (`binding::Watched`:
the module and the name, and the original function). pltrs (Rust) reads
the attribute each time it would use one (`in_force`) and uses it only
while the attribute is not the original, so the module and the class stay
plain (no module class or metaclass watching assignments, which programs
would see as `type(Text)`). Without replacements pltrs (Rust) never calls
Python for text.

matplotlib caches the metrics of the lines it measures per renderer
(`text._get_text_metrics_function`, a `WeakKeyDictionary` of
`lru_cache`s), and its tests read the caches. pltrs keeps them natively
(`pltrs::text::MetricsCache`, `_text_metrics`): the cache of a renderer is
installed on the thread while pltrs (Rust) lays out or draws with that
renderer (`text._measuring_with`: the canvas's draws and prints,
`draw_without_rendering`, `tight_layout`, `Text.get_window_extent`), so
every line it measures meanwhile is found in or added to it without a call
into Python, and measuring from Python goes through the same cache. The
renderer installed is also the one a replaced `_get_text_metrics_with_cache`
receives (a stand-in only when none is installed), as matplotlib hands it
the renderer laying out (`text._layout_renderer`): under the PDF,
PostScript and SVG canvases that is the renderer of their print
(`_print_renderer`), which measures as that backend does but draws
nothing, so `fig._get_renderer()` stays the Agg renderer drawing onto the
raster canvas. A cache knows how its renderer measures (the `_text_metrics`
of `RendererPdf`, `RendererPS` and `RendererSVG`, that of the vector
renderer of a `MixedModeRenderer`; `pltrs::text::RendererMetrics`): while it
is installed, pltrs (Rust) lays the whole text out as that backend does
(unhinted, in AFM metrics where `pdf.use14corefonts` or `ps.useafm` asks,
through `TextToPath`). A measure the program asks for without a renderer
on such a canvas measures with its print renderer and leaves the figure at
72 dpi, as matplotlib's `_get_renderer` runs the print up to its draw
(`text._measured_by_print`). matplotlib's `Text.draw` lays
a tick label out again where it draws it; while a cache is installed, pltrs
(Rust) looks the label's lines up in it again where it draws the label
(`LaidOutText::measure_again`), so the cache sees the same lookups without
the cost of a second layout.

matplotlib's renderers measure lines at their own resolution, while texts
are placed (and their fonts' heights scaled) at the figure's: a renderer of
another resolution handed to `get_window_extent(renderer)` or
`get_tightbbox(renderer)` (another figure's, one kept from before the dpi
changed) gives lines that size. `text._measured_by` installs such a
renderer's cache and has pltrs (Rust) measure the lines at its resolution
meanwhile, and the pads matplotlib converts with the renderer's
`points_to_pixels` (offset boxes, axisartist labels, tables:
`pltrs::artist::set_measured_at`); a `Text` keeps the renderer it was
last measured with until it is drawn, as matplotlib's `Text._renderer`.

The renderer's text drawing can be replaced too. `RendererAgg.draw_text`
and `draw_tex` draw a line onto the canvas (`_native.drawing.draw_text`,
see "Drawing defined in Python").
While a canvas's renderer has a `draw_tex` other than its own (assigned to
the instance, or `RendererBase.draw_tex`), its draws hand pltrs (Rust) a
`pltrs::text::TexPainter`: every TeX line is drawn by calling it, and the
paths it draws with `draw_path` are queued and drawn by pltrs in the
line's place. While `RendererAgg.draw_text` is `RendererBase.draw_text`
(a `Watched` attribute again, read whenever a raster canvas draws text),
raster canvases draw text as paths (`pltrs_render::set_texts_as_paths`),
as `_draw_text_as_path` does.

usetex looks TeX fonts up through `dviread.PsfontsMap.__getitem__`, which
tests replace: pltrs-text's font map lookups
(`pltrs_text::tex::set_font_map_lookup`) call the replacement while the
watched attribute is not the original.

### Drawing defined in Python

matplotlib draws a figure by calling every artist's `draw(renderer)`, so
a subclass of the program's draws whatever its `draw` draws, a path
effect of the program's draws through its `draw_path(renderer, gc, tpath,
affine, rgbFace)`, and an artist drawn with a renderer of the program's
calls that renderer's methods. pltrs (Rust) draws figures itself and calls
the Python code back where matplotlib would (`_python_drawing`), never for
figures without it:

* **Drawings.** The renderers pltrs hands Python are matplotlib's classes
  with their drawing methods done natively (`backends/_native_drawing`):
  `draw_path`, `draw_markers`, `draw_path_collection`, `draw_quad_mesh`,
  `draw_gouraud_triangles`, `draw_image`, `draw_text`, `draw_tex`, the
  groups and filters, each adds its step to a `pltrs::artist::Drawing`
  (`_native.drawing`), and `Artist.draw` with one of them adds the artist,
  drawn as pltrs (Rust) draws it (`_draw_artist`). Between draws the
  drawing is drawn at once (the canvas's `RendererAgg` onto its canvas, a
  `RendererAgg` of the program's onto pixels of its own).
* **Artists.** An artist whose class takes `draw` from outside the
  package (`_program_classes.takes_from_program`), or computes what
  matplotlib's `draw` reads (an arc's `theta1` as a property, the
  `_computed` attributes of the package's class, pushed by
  `_push_computed` when it is drawn, read as often and in the order
  matplotlib's code reads them where it makes, adds and draws the artist:
  `Ellipse._recomputed`, `Arc._given_angles`; an image whose class gives
  its pixels, `make_image`, `get_extent`, `get_transform`, drawn by
  matplotlib's `_ImageBase.draw`, pltrs (Rust) making the pixels over the
  extent and through the transform the class gives), gets a painter
  (`pltrs::artist::ArtistPainter`, `_native.drawing.set_painter`) holding
  the object weakly. Once the figure is laid out, before its artists are
  drawn and with the figure free to change, pltrs (Rust) calls
  `record_drawing` for every painted artist the draw reaches, in drawing
  order: the class' `draw` (without its rasterization wrapper, which pltrs
  applies as `allow_rasterization` does) runs with the renderer of the
  draw recording, then pltrs draws the recording at the artist's place;
  `super().draw(renderer)` is the artist drawn there as pltrs draws it,
  and any artist drawn from Python is drawn with the transform it was
  given as it is then (`evaluate_given_transform`: an `Affine2D` the
  drawing changed in place), as matplotlib's draw reads `get_transform()`.
  The draw may change the figure (an image computing its array, as the
  ribbon box example does); the transforms it reads are those of the
  laid-out figure (`_transform_trees.sync`, the artist's given
  transform handed to pltrs again). The renderer is the one each format
  hands matplotlib's draw: the canvas's `RendererAgg`, or a
  `MixedModeRenderer` around `RendererPdf`, `RendererSVG`, `RendererPS` or
  `RendererPgf` (`FigureCanvas*._program_renderer`, installed for the
  print by `FigureCanvasBase._program_drawing`, `drawing_with`), whose
  `start_rasterizing` is recorded too. A loose artist drawn this way
  resolves its transforms in the figure drawing it. An Axes class of the
  program's (astropy's `WCSAxes`) is painted the same way, however the
  Axes is made (`ConstructionMixin._init_axes_state`). A class that
  measures its artists (`get_window_extent`, `get_tightbbox`) is measured
  before a layout engine lays the figure out and before its tight box is
  computed, as matplotlib's engines and `bbox_inches="tight"` ask, not at
  other draws (`Figure::prepare_layout`): parents first, the artists of
  an Axes measuring itself measured by its own tight box (`Axes.get_tightbbox`
  asks each artist of the Axes), and again before constrained layout's
  second pass (`FigureState::measure_painters_lent`); what the measuring draws is
  recorded and dropped (`measure_artist`), as what matplotlib's layouts
  draw is drawn over.
* **Adding.** matplotlib's `add_*` methods call `set_figure`,
  `set_transform` and the `axes` setter of the artist they add, and its
  constructors the setters of the artist (a line's `set_data`): where a
  class of the program's overrides them, pltrs calls them too
  (`_program_classes.set_artist_props`), and gives the artist its class'
  `zorder` (`take_class_defaults`).
* **Path effects.** An effect of a class of the program's crosses as
  itself (`("python", effect)`, `pltrs::patheffects::CustomPathEffect`):
  pltrs (Rust) calls `effect_draw_path` for every path it draws with it,
  which runs `draw_path` with the renderer recording, and draws the
  recording in the path's place. The effects of an artist share the
  graphics context, as matplotlib hands them all the same one.
* **Renderers of the program's.** `Artist.draw` with a renderer pltrs did
  not make (`RendererBase._draw_artist`, also a `MixedModeRenderer`
  around a renderer of the program's) has pltrs (Rust) draw the artist
  through the renderer's methods (`_native.drawing.draw_through`,
  `forward_drawing`): a figure is laid out and drawn whole, its rasterized
  artists through the renderer's `start_rasterizing`.
* **Graphics contexts of the program's.** While the canvas's renderer has
  a `new_gc` of the program's (`RendererBase.new_gc = ...`, returning a
  `GraphicsContextBase` subclass), every artist pltrs (Rust) draws on the
  raster canvas starts from a context it makes
  (`pltrs::artist::GcSource`, `_native.backends.set_program_gc`, installed
  by `RendererAgg._drawing`), as matplotlib's artists start from
  `renderer.new_gc()`.
* **3-D projections of the program's.** A 2-D patch or collection added to
  a 3-D axes whose class gives `do_3d_projection` (the `Arrow3D` of many
  programs) is drawn through Python, and before it is drawn its
  projection runs (`mplot3d._program_projection`, a preparation
  `_python_drawing.before_drawing` registers): the depth it returns
  orders it among the axes' collections and patches
  (`Axes3D::set_program_depth`).

A print that matplotlib draws twice (laid out by an engine, or cut to its
tight box) prepares the painters twice, so that the program's `draw` runs
as often as matplotlib's.

A painted artist's object is pickled with its figure (the painter is
archived as the object), so a figure with one pickles as matplotlib's
does, or fails with pickle's error for a class pickle cannot find.

### Projections of programs

`matplotlib.projections.register_projection` takes any `Axes` subclass,
and matplotlib promises such a class very little: its `__init__` calls
`set_figure`, `_set_lim_and_transforms`, `_gen_axes_spines`, `_init_axis`
and `clear`, and its draw asks every child for `draw(renderer)`. A
library may override any of them and add what it likes. mpltern's
`TernaryAxes` is the measure: three axes of its own classes (`taxis`,
`laxis`, `raxis`) beside a hidden x and y axis, six spines of its own
class, a polygon as its patch, transforms of three coordinates, and its
own `clear`, `grid`, `tick_params`, `callbacks` and title placement.

pltrs (Rust) holds an x and a y axis per Axes and draws them from its
state. It holds nothing of a projection it does not know. One rule
covers every case. **What the native Axes has, pltrs (Rust) keeps and
asks the class about through hooks. What the native Axes does not have is
matplotlib's own Python object, and pltrs (Rust) holds its place only**:
where it draws, where it is measured, and what happens to it when the
Axes is cleared, pickled or copied.

What a class may override or add, and how each is honoured:

| the class gives | how pltrs honours it |
|---|---|
| `set_figure`, `_set_lim_and_transforms` | run where `__init__` runs them (`_program.set_up`). The trees are matplotlib's, so transforms of any number of input dimensions the class adds (`transProjection`) compute as matplotlib's. |
| `transData`, `get_xaxis_transform`, `get_yaxis_transform`, the text transforms | the hooks of the native x and y axis (`_Adapter`, see "Transforms"). |
| an `XAxis`, `YAxis`, `XTick` subclass named `x` or `y` | the native axis, asked through the hooks (`tick_interval`, `tick_sides`, `tick_label`). |
| an axis under another name (`axis_name = 't'`, listed in `_axis_names`) | a *held axis*: matplotlib's `Axis` in Python (below). |
| `_gen_axes_spines` with spines naming a side | the native spine of that side, its interval from `_adjust_location`. While `_init_axis` runs, `ax.spines` already gives the class' spines, so that `spines['tside'].register_axis` finds them. |
| spines of other names (`'tside'`), any path, any transform | spines of no side beside the frame, when the class makes several of them or spines of the sides too (`draw_class_spines`). Of a class with its own `_adjust_location`, each draw runs that first and draws `_path` as it leaves it, through `get_transform()` as it is then (`Spine._adjust_before_draws`). |
| `_gen_axes_patch` | the outline of the background and the clip of the Axes. A patch the program changes later (`patch.set_xy`) is handed over again. |
| `clear` (or `cla`) | called where matplotlib's `__init__` and the program call it. `super().clear()` clears the native Axes and every held axis, and ends as matplotlib's: each held axis no Axes shares gets `_set_lim(0, 1, auto=True)`, and `_share<name>` is read first, which fails for a class that did not set it, as it fails in matplotlib. An axis hidden before a clear stays hidden (`Axis.clear` leaves the artist its visibility). |
| `draw`, `get_tightbbox`, `_update_title_position`, `autoscale_view`, `get_data_ratio`, `tick_params`, `grid` | called where matplotlib calls them (see "Drawing defined in Python", `_program_locators`, `_program_tick_params`). |
| `callbacks` as a property of the class | matplotlib's registry knows `xlim_changed`, `ylim_changed` and `zlim_changed` only, so a class with another axis keeps a registry of its own. pltrs (Rust) tells the Axes' `callbacks` of a change by reading that attribute then, and connects itself when the Axes is cleared rather than when pltrs' own property is first read (`ClearMixin._hear_limit_changes`). |
| `_axis_names`, `_shared_axes` of its own | the Axes' `_axis_map`, `get_children`, `set_axisbelow`, the unit pipeline and `__repr__` go over `_axis_names`, as matplotlib's do. What reaches a native axis by name asks first whether the name is one (`_AxesBase._native_axis_names`: x and y, z on 3-D Axes). `sharex` and `sharey` join the class' own groups too (`_join_class_group`), which `get_shared_x_axes()` and the held axes read. |
| a subclass of `GeoAxes`, `AitoffAxes`, `HammerAxes`, `MollweideAxes` or `LambertAxes` | a projection class of the program's like any other (`_program.is_program_geo`): a rectilinear native Axes that matplotlib's own `GeoAxes` code makes geographic (below). |

matplotlib itself asks three things of a class listing a third axis,
which mpltern does and the test projection does too: `_share<name>` set
before `__init__`, a `callbacks` registry knowing `<name>lim_changed`, and
an `autoscale_view` taking `scale<name>`.

**A held axis** is an `Axis` whose name is none of its Axes' native axes
(`axis._names_native_axis`). It is matplotlib's `Axis`, ported as it is
(`_held_axis.py`, from matplotlib 3.11.2's `axis.py`), over objects of
the package:

* **State.** Its tickers hold the locator and formatter objects, which
  compute natively as they do for a stand-in axis (`TickHelper._axis_info`
  asks the axis for its view interval and tick space, which the class
  answers). Its ticks are `Tick` objects made by the class' `_get_tick`
  (`majorTicks`, `minorTicks`, lazily, as matplotlib's `_LazyTickList`).
  A tick made by its constructor is an ordinary artist: its lines and
  labels are artists of the figure with the transforms matplotlib's tick
  takes of its Axes when it is made (`Tick._take_axes_transforms`), which
  the tick class may replace. Its `label` and `offsetText` are `Text`
  objects. Its tick parameters are `_major_tick_kw` and `_minor_tick_kw`.
  Its scale is a `scale` object, its units and converter are Python's.
  Its limits are whatever the class' `get_view_interval` and
  `set_view_interval` say: matplotlib's base `Axis` has none either.
  `_set_lim` is matplotlib's, so `<name>lim_changed` is processed on the
  Axes' `callbacks`, the Axes sharing the axis follow through the class'
  `_shared_axes` group, and autoscaling is the class' `autoscale_view`.
* **Dispatch.** A program's axis class derives from the package's `XAxis`
  or `YAxis`, so the package's methods serve both kinds of axis. Each
  method and property `Axis`, `XAxis` and `YAxis` define is wrapped once
  (`axis._serving_held`). For a held axis (an instance marked `_held` by
  `__init__`, before anything else runs) a call goes to matplotlib's
  function of the same name (`_held_axis.function`), else to what the
  classes after `Axis` have (`Artist`'s `set_visible`, `get_zorder`:
  the artist properties the package's classes keep in the native axis).
  For a native axis it goes to the package's own. A property reads and
  writes the axis' `__dict__` (`_held_axis.read`, `write`), as
  matplotlib's are plain attributes. `super()` calls of the program's
  classes arrive there either way, and matplotlib's private methods the
  package has no use for (`_copy_tick_props`, `_update_label_position`,
  `_init`) exist on the classes for them. `_held_axis` loads when the
  first held axis is made, so a program without one never imports it.
* **Place in the Axes.** `Axis.__init__` asks pltrs (Rust) for the axis'
  artist (`Axes::add_program_axis`, `_native.axes.add_program_axis`), a
  `ProgramAxis` painted through the axis' `draw` and measured through its
  `get_tightbbox` (`_python_drawing.draw_through_python`). The Rust Axes
  lists its program axes after x and y. It draws each in the slot
  matplotlib's `get_children` gives the axes (after the spines, before
  the titles), at the axis' own z-order (which `set_axisbelow` and a
  clear set), leaves it out with `axis("off")` and, when it is animated,
  out of every draw but a print. It measures each as an axis for
  `Axes.get_tightbbox` and the layout engines: only while the Axes shows
  its axes, whatever clips it. It keeps it through `clear` (the artist is
  a part of the Axes, as a spine is) and archives it with the figure (the
  painter pickles as the axis object). An axis whose name the Axes does
  not list is no part of it: a plain artist no one draws.
* **Measuring.** Before a layout engine or a tight box measures the
  figure, each painted artist with a `_before_measured` method makes
  itself ready (`_python_drawing.measure_artist`), where matplotlib's
  `Axes.get_tightbbox` does the same steps: an Axes whose class places
  its titles its own way applies its aspect and places them, then leaves
  the measuring to pltrs (Rust); a held axis has its Axes apply the
  aspect, then is measured by its own `get_tightbbox`. The trees are
  brought up to where the figure is first, so a layout engine's second
  pass measures at the positions of its first. A layout engine, and
  `Axes.get_tightbbox(for_layout_only=True)`, ask an Axes and its axes
  with `for_layout_only=True`, as matplotlib's do
  (`ArtistPainter::measure_for_layout`, `_python_drawing._tight_box`:
  without the keyword for a class whose method does not take it).
* **Figure.** A held axis and the ticks its class makes keep the figure
  they were made in when their Axes leaves it (`ax.remove()`), as
  matplotlib decouples the Axes only.
* **Calls, and how often.** The axis' `draw` runs once per draw of the
  Axes that reaches it. Painters are prepared in the order the Axes
  draws its children and only for those it draws
  (`AxesState::drawn_children`), so the spines' `_adjust_location` runs
  before the axes' `draw`, and nothing of an axis runs with the axes
  off. The `draw` is matplotlib's code: `_update_ticks` (the locator's
  `__call__`, the formatter's `format_ticks`, each tick's
  `update_position`), the label boxes, each tick's `draw`,
  `_update_label_position`, `_update_offset_text_position`, the label and
  the offset text. A print that matplotlib draws twice runs it twice.
  `get_tightbbox` runs where a layout engine or a tight box measures the
  Axes, and where the class' own title placement asks.
* **Tick parts.** A tick's parts are drawn by their own `draw`, through
  the transform they hold when drawn. What a tick class sets on a mark
  between draws is read at draw time: a line's `_marker` is the
  `MarkerStyle` it draws with (`_line_markers.LineMarker`), so
  `line._marker._transform = t` (how matplotlib's polar ticks and
  mpltern's turn their marks) reaches the line. A transform holding a
  matrix a class of the program's computes when asked (mpltern's shift of
  a tick label away from its corner, an `Affine2DBase` with its own
  `get_matrix`) is handed to pltrs (Rust) again wherever the text is
  measured (`_live_transforms.follow_given`), as matplotlib computes the
  matrix at each use.

What stays native: the background, the clip, the children, the x and y
axis, titles, legends and layout are pltrs (Rust)'s. A held axis costs
one call into Python per draw plus one per tick part, which only figures
with such an axis pay.

What libraries' Axes classes read while matplotlib's `__init__` runs
their `clear` is there under pltrs too: the tick locations of an Axes its
figure has not added yet (`Axis::ticklocs_as_is` of a removed axes;
pysmithchart draws its grid from them), and matplotlib's `_gridOn`
(mplstereonet's `grid` reads it).

**A subclass of a geographic class** (mplstereonet's `StereonetAxes`, a
`LambertAxes` with a transform, an affine part, grids and a `cla` of its
own) is not drawn by pltrs (Rust)'s geographic Axes, which knows four
projections and nothing a class changes about them. matplotlib's
`GeoAxes` is a plain `Axes` with a few methods on top, and the example
`custom_projection` is the proof that pltrs draws such a class through
the hooks. So `projections/geo.py` keeps matplotlib's code beside what
pltrs (Rust) does for its own Axes, and each method takes one or the
other by the class (`GeoAxes._is_programs`): `__init__` (the cap, the
aspect, the clear), `_init_axis`, `_gen_axes_spines`, `clear` (the grids
set through the class' `set_longitude_grid` and friends, the limits),
`_get_affine_transform` (which asks the class for its core transform
with resolution 1 and maps two points, as matplotlib's does),
`set_longitude_grid`, `set_latitude_grid`, `set_longitude_grid_ends`.
Every method the class overrides is then called where matplotlib calls
it, with matplotlib's arguments. The private attributes programs read
are plain attributes there (`_longitude_cap`, `_center_longitude`,
`_center_latitude`); on pltrs (Rust)'s own Axes they read what it holds
(`geo._NativeSetting`).

What the hooks of every projection class gained for this:

* A gridline's path asks for matplotlib's 180 interpolation steps
  (`AxisPainter::gridline_path`): a transform of the program's decides by
  them whether it bends the path.
* The data transform maps the paths of lines through a
  `TransformedPath`, kept per path (`_NonAffinePart`, `keeps_paths`): the
  class' `transform_path_non_affine` and `transform_non_affine` are
  called once for a line until the transform is invalidated, as
  matplotlib's lines call them, not at every draw.
* A label goes beyond the Axes on a side the class made no spine of
  (`Projection::spines`, matplotlib's `spines.get("bottom", self.axes)`).
* An Axes whose constructor adds other Axes to the figure joins it after
  them (`_constructed_native`).
* Pans and zooms map the pointer back through the inverse the program's
  tree gives (`Projection::axes_to_data`, `_Adapter.axes_to_data`):
  matplotlib inverts each affine part of `transData` apart and
  multiplies the inverses, which leaves another last bit than the
  inverse of the product.

Not done:

* pltrs (Rust) aligns the labels of the x and y axes only: matplotlib's
  `align_labels` knows no other axis either.
* A plain Axes sharing an axis with an Axes whose class keeps groups of
  its own follows that Axes' limits; in matplotlib it does not, since
  the two classes keep separate groups and `set_xlim` walks the group of
  the Axes it is called on. pltrs (Rust) has one group per shared axis.
* The x and y axis of an Axes that left its figure answer no figure, and
  measure nothing (matplotlib's still measure there).

Tests: `python/tests/test_proj1.py` and `test_proj2.py` check a
three-axis projection (`python/tests/curve_projection.py`) and a
sinusoidal projection on each geographic class against values recorded
from matplotlib, with no library imported. `cargo xtask diff --corpus
mpltern pysmithchart mplstereonet pyrolite` runs four libraries'
projections: a ternary Axes with three held axes, a Smith chart (an
`Axes` subclass with transforms, spines, a patch and a grid of its own),
the stereonets (a `LambertAxes` subclass), and a library drawing on the
ternary Axes of another.

### Methods of matplotlib's classes that programs override

Libraries subclass matplotlib's classes and override what matplotlib's
own code calls on ``self`` (a census of about 200 libraries and corpora
is ``spec/diff/overrides_census.json``, made by
``spec/diff/overrides_census.py``). pltrs (Rust) does that work natively,
so each such call is a dispatch: pltrs asks the Python object only when
its class takes the member from the program
(`._program_classes.takes_from_program`, one test per class and member,
cached), and objects of the package's own classes never pay. The places
it asks:

* **Construction.** An Axes class overriding ``__init__``, ``clear``,
  ``cla`` or ``grid`` (`.figure._CONSTRUCTION_CALLS`) is made by its
  constructor, with the positional arguments ``add_subplot`` was given
  (`.axes._construction.given_arguments`); a clear calls the class'
  ``grid`` as matplotlib's does. One overriding ``_set_lim_and_transforms``,
  ``_gen_axes_spines``, ``_init_axis``, ``_gen_axes_patch`` or the axis
  transforms is a projection of the program's whatever its ``name``
  (`.projections._program._PROJECTION_HOOKS`); unpickled, it keeps the
  spines its class made.
* **Measuring.** An artist whose class measures itself
  (``get_window_extent``, ``get_tightbbox``) gets a painter, through which
  pltrs (Rust) measures it for a layout or a tight print, unless matplotlib
  leaves it out of its Axes' box (clipped to the Axes); an Axes is asked
  with ``bbox_extra_artists=None``.
* **Transforms** of the program's map arrays in one call, through their
  ``transform_non_affine`` and then their affine part, wherever pltrs
  maps points (``_native.transforms.apply``, a line's extent).
* **Ticks.** A locator class giving its own ``tick_values`` ticks through
  it where matplotlib's ``__call__`` does.
* **Colour mapping.** A norm of the program's is called with its values
  alone unless pltrs clips.
* **Scales** of the program's keep their Python object natively
  (``PyScale``), which a pickle of the figure holds; unpickled, pltrs asks
  ``scale._program_scale`` for its functions again.
* **Adding.** A class overriding an ``add_*`` method (``add_line``,
  ``add_patch``, ``add_collection``, ``add_image``, ``_add_text``,
  ``add_table``) is called with each artist the plotting methods make,
  with matplotlib's ``autolim`` (`.axes._program_adds`); the package's
  method it reaches adds nothing more, and an override that does not
  reach it leaves the artist out of the Axes, as matplotlib does.
* **Constructors' setters.** `.Text`'s constructor calls the class'
  ``set_text`` and the setters of its explicit keywords it overrides,
  given or not; `.Legend`'s calls its ``set_loc``. The others call the
  setters matplotlib's constructors call (`Line2D`'s styles and data,
  ``Polygon.set_xy``, the collections' ``set_verts``, ``set_segments``
  and ``set_transform``, the images' resampling settings, the legend's
  anchor, title and dragging) with matplotlib's arguments, when the class
  overrides them (`._program_classes.replay_setters`). Which setters a
  class takes from the program is found once, as the class is made
  (``Artist._program_setters``, empty for the package's classes, told
  apart by module name, so that making them costs nothing).
* **Texts.** A `.Text` class answering a getter matplotlib's draw and
  measuring read (``get_text``, ``get_rotation``, ``get_color``,
  ``get_wrap``, ``_get_wrapped_text``; ``get_unitless_position`` for
  measuring) is drawn and measured as it answers: pltrs (Rust) holds the
  answers meanwhile (`.Text._as_class_reads`; ultraplot's ``CurvedText``).
* **Plotting methods made of plotting methods.** matplotlib's
  ``errorbar`` draws its bars through ``vlines`` and ``hlines``, ``bxp``
  its boxes through ``plot``, ``hist`` through ``bar`` or ``fill``,
  ``violin`` through ``fill_between(x)``, ``vlines`` and ``hlines``,
  ``stackplot`` through ``fill_between``, ``stem`` through ``plot`` and
  ``vlines``, ``pie`` and ``pie_label`` through ``text``, ``bar_label``
  through ``annotate``, ``grouped_bar`` through ``bar``, the spectra and
  ``ecdf`` through ``plot``. For a class overriding the inner method
  (`._program_classes.takes_any_from_program`, a native cache hit for the
  others) the outer method runs matplotlib's code (the ``_*_reference``
  modules of `.axes._plotting`).
* **Prints cut to a window** (a tight box, a ``bbox_inches`` given): while
  pltrs (Rust) prints, the figure has the window's size
  (``get_size_inches``, ``bbox_inches``, ``bbox``) and ``transFigure`` is
  moved by its corner, as matplotlib's ``adjust_bbox`` leaves them for the
  print's draw (mpl_scatter_density sizes its bins by it).
* **Spines replaced.** Ticks keep the transform of the spine they were
  made on when the program puts another in its place
  (``ax.spines['bottom'] = spine``), until they are made anew, as
  matplotlib's ticks do.
* **Linear spines** of a class overriding ``_adjust_location``,
  ``get_patch_transform``, ``get_spine_transform``,
  ``_ensure_position_is_set``, ``get_path``, ``get_transform`` or
  ``get_window_extent`` in a plain Axes are drawn and measured through
  Python: the class lays the spine out as matplotlib's draw does, and
  pltrs (Rust) draws the path in display pixels it leaves
  (`Spine._push_layout`).
* **Picking** goes through the children a class gives (``get_children``),
  which pltrs (Rust) holds no tree of, as matplotlib's ``Artist.pick``.
* **Prints.** A figure class's ``draw`` also runs in the draw matplotlib
  makes before a print laid out by an engine or cut to its tight box; a
  layout engine of the program's runs once per such print.
* **Affine transforms** whose class gives its own ``transform_affine``,
  ``transform`` or ``transform_point`` map points through them where
  matplotlib maps a point through the transform alone (a text placed by
  it): `._live_transforms` hands such a transform over as a map pltrs
  (Rust) calls back, which draws paths through its matrix, as
  matplotlib's renderers do; within a composition matplotlib maps through
  the matrix, and so does pltrs. Its ``get_matrix`` is read where the
  transform is handed over (and again where a text is measured,
  `follow_given`), not at every draw.
* **Axis transforms** a projection of the program's gives
  (``get_xaxis_transform``) are taken anew at each clear, as the ticks
  matplotlib's clear makes take them (not at each draw: matplotlib's ticks
  keep theirs, and new ticks copy those of the first).
* **axes_grid1 sizes** of the program's are asked with the renderer of the
  print, else the one the canvas drew with last, and pickle as their
  ``get_size``.

`python/tests/test_gen5.py` and `test_gen6.py` check each with a subclass
of the kind the libraries write; ``cargo xtask diff --generated overrides`` runs the
family (``spec/diff/generators/overrides*.py``).

Libraries also replace methods and functions by assignment, on a class
(mplcursors' ``Axes.scatter``), on one object (ultraplot's
``axis._update_ticks``, pygtc's tick labels' ``set_x``) or on a module
(proplot's and ultraplot's ``scale.scale_factory``); the census is
``spec/diff/patches_census.json`` (``patches_census.py``) and ``cargo
xtask diff --generated patches`` runs the family. A replacement on a class
is a class' member like an override (`takes_from_program` reads it). One
on an object is seen as it is assigned (`.Artist.__setattr__`): an axis
given ``_update_ticks`` is ticked by the program's tick updater
(`._tick_updates.follow_program_update`) where matplotlib calls the
method (``UpdateSite.Matplotlib``; where pltrs needs ticks besides, it
computes them without it), and the ticks the replacement returns are
those drawn. A module function is called where matplotlib calls it:
``scale_factory`` makes each shared axis' scale, and when replaced, the
scales each new axis and each clear make (`.axis.Axis`, `.axes._clear`).
`python/tests/test_gen7.py` checks them.

### pyplot's state

pyplot's figure registry is Python state: `_pylab_helpers.Gcf` maps
numbers to managers, which hold canvases and `Figure` objects (whatever
`FigureClass` made them, with their Python attributes). It is matplotlib's
module, kept as it is, because tests use it directly. pltrs (Rust)'s own
`pltrs::pyplot` registry is not used. The current axes of a figure is
pltrs (Rust)'s (`Figure.gca` calls `_figure.gca`).

### Backends, canvases and printing

Backends are matplotlib's: `backends.backend_registry` names them,
`pyplot.switch_backend` loads `backends/backend_<name>.py` and builds
`new_figure_manager` and `show` from its `FigureCanvas` (the `_Backend`
export machinery), and `pyplot.figure` makes managers through it. As in
matplotlib, `rcParams['backend']` starts as `rcsetup._auto_backend_sentinel`
(or `MPLBACKEND`), which pyplot resolves the first time it needs a
backend: the framework already running, else pltrs' own windows
(`macosx` on macOS, `pltrs` elsewhere), then Qt, GTK, Tk and wx, then
`agg` (no display on Linux makes every interactive one refuse).

* **pltrs' own windows** (`backends/backend_pltrs.py`, the `pltrs`
  backend, which the registry knows beside matplotlib's built-in names
  without listing it among them; `backend_macosx` on macOS) are a GUI
  toolkit backend over pltrs (Rust)'s windows (`_native.gui`, a
  `pltrs::gui::WindowHost`): a window shows its figure's raster canvas
  (whatever `FigureCanvasAgg.draw` and its `blit` put there) above a toolbar
  of the backend's buttons, and reports input, resizes, repaints, toolbar
  clicks, close requests, chosen files and timers as tuples, which
  `backends/_pltrs_loop.py` hands to the canvas (matplotlib's events,
  in matplotlib's coordinates), the timer or the file chooser waiting.
  Navigation, keys and callbacks stay matplotlib's (`NavigationToolbar2`
  or the tool manager over pltrs' toolbar). The subplot configuration
  tool is pltrs' dialog (`SubplotToolPltrs`, as Qt's backend has a
  dialog of its own rather than matplotlib's `widgets.SubplotTool`
  figure): the host reports what the user asks as `"subplots"` events,
  which the tool carries out with matplotlib's calls (`subplots_adjust`,
  `tight_layout`, `ConstrainedLayoutEngine.set`) before showing the
  figure's layout again; under constrained layout it adjusts that
  layout's padding and spacing, or hands the subplots to the subplot
  parameters (see `pltrs::gui`'s subplot tool). The loop blocks until
  something happens, as Qt's does, so an idle window costs nothing:
  input, the next timer, a call another thread queued for the windows
  (see "Threads"), an idle draw or a stop asked for on another thread
  (`_pltrs_loop.wake`), or a signal: around the loop on the main thread,
  Python's signal handler writes to the host's wake pipe
  (`signal.set_wakeup_fd`, as matplotlib's Qt backend sets it; the
  program's own descriptor is put back afterwards, with the signals that
  came meanwhile), so Ctrl+C interrupts at once. The wait itself runs
  without the GIL (`binding::blocking`, through `pltrs_platform`'s wait
  hook), and IPython's input hook and the Jupyter kernel's loop wait for
  their descriptor in a thread, as Qt's socket notifiers do. Its
  framework counts as running once the host is
  made (`cbook._get_running_interactive_framework`). A headless host
  (`_pltrs_loop.connect_headless`) opens windows off screen and takes
  injected input, for tests.

  What a program sees of the user's input is what matplotlib's QtAgg
  gives it, event for event (`pltrs::gui`'s canvas input, shared with
  pltrs (Rust)'s `FigureWindow`): Qt's key names (`meta` for the Super
  key, Caps Lock lowering letters, a modifier key left out of its own
  name), no event for a motion that does not move, for a sideways wheel
  or for Alt and the wheel, a leave held back until the last button is
  released and reported where the pointer went, a draw asked for while
  drawing done afterwards, an idle draw done when the window repaints
  (after a resize, before the input that came after it, however the
  window system batched them), and the blitted region Qt repaints. Every
  event carries its `guiEvent` (`backend_pltrs.WindowEvent`). The
  deliberate exceptions, dead keys composed and Shift+Tab reaching the
  canvas, follow matplotlib's GTK and Tk backends.
  `python/tests/test_gui_events.py` measures this: the scenarios of
  `python/tests/gui_events` (pointer, wheel, keys, picking, resizing,
  every widget and selector, the toolbar's shortcuts, animations) show a
  figure in a real window of a private Xvfb, feed it input through the X
  server (XTEST) and compare every `mpl_connect` event, what the widgets
  report and the pixels they show with what matplotlib's program saw
  under QtAgg (goldens recorded with `python -m gui_events --record`),
  on X11 and on Wayland through a weston nested in the Xvfb.
  `python -m gui_events` (from `python/tests`) runs matplotlib and pltrs
  side by side.
* **Tk** (`backends/_backend_tk.py`, `backend_tkagg.py`, matplotlib's)
  blits through `_tkagg`, pltrs' counterpart of matplotlib's C helper,
  which hands Tk the pixels with its photo commands, as PNG data.
* **Qt, GTK 3 and 4, wx** are matplotlib's modules, kept as they are but
  for their imports; their canvases paint the Agg canvas' pixels through
  the buffer of `copy_from_bbox`'s region (`BufferRegion.__buffer__`).
  Without the toolkit they raise matplotlib's `ImportError` (the Qt
  backends after its binding selection, `qt_compat`). pltrs has no Cairo:
  the `*cairo` backends draw with the raster too, but in Cairo's graphics
  state, as matplotlib's Cairo renderer draws (`backends/_cairo_canvas.py`,
  the mixin of every Cairo canvas: `_native.backends.drawing_like_cairo`
  runs their draws and prints in `pltrs::artist::GraphicsState::Cairo`,
  where lines an artist sets no join for are mitred, as on a Cairo
  context, rather than rounded); otherwise their widgets show the pixels
  the `*agg` ones show (matplotlib's own Cairo variants also differ from
  its Agg ones by their text and antialiasing). wx's canvases call wx
  from its main thread only (`_FigureCanvasWxBase._on_main_thread`): a
  draw, blit, `draw_idle` or cursor change from another thread is shown
  from the main thread's loop (`wx.CallAfter`). wx is not thread-safe,
  and a worker's draw painting the window while the main thread painted
  it corrupted wx's heap.
* **Cairo** (`backends/backend_cairo.py`) is matplotlib's module, which
  needs pycairo or cairocffi as matplotlib's does: `RendererCairo` and
  `GraphicsContextCairo` draw onto a Cairo context of the program's (a
  figure drawn with one goes through their methods, see "Drawing defined
  in Python"), while the `cairo` backend's canvas, `FigureCanvasCairo`,
  is the Agg canvas in Cairo's graphics state, printing PDF, PostScript
  and SVG with pltrs (Rust)'s writers at 72 dpi as matplotlib's Cairo
  surfaces are.
* **WebAgg** (`backend_webagg.py`) runs pltrs (Rust)'s web server
  (`_native.web`, a `pltrs::web::WebRelay`) instead of Tornado: it serves
  matplotlib's pages and files and hands the WebSockets' messages and the
  downloads to matplotlib's figure managers (`backend_webagg_core`), from
  an asyncio loop that also runs their timers, under the URL prefix
  `WebAggApplication.initialize` is given (`WebOptions::url_prefix`).
  So it imports and serves without Tornado installed, where matplotlib's
  module raises `RuntimeError("The WebAgg backend requires Tornado.")`;
  pltrs keeps it working: what programs and libraries do with the module
  is select it (`MPLBACKEND=webagg`, IPython's and ipykernel's backend
  tables) or subclass `backend_webagg_core` (ipympl), and none of the
  corpora or the libraries of the ecosystem's venvs relies on the error.
  `backend_webagg.tornado` gives Tornado's package where it is installed
  (`_lazy_imports.getter`). **nbAgg** is matplotlib's, over the kernel's
  comms.
* **Jupyter's inline backend**: IPython's kernel selects
  `module://matplotlib_inline.backend_inline`, whose module imports
  `matplotlib`; unless that gives pltrs (pltrs loaded as `matplotlib`,
  or its `matplotlib` distribution installed), the registry loads pltrs'
  port of it instead (`backend_inline.py`), which registers the kernel's
  formatters for pltrs' figures.

A canvas draws and prints through the figure's native handle
(`_native.backends`):

* `FigureCanvasAgg.draw` draws the figure onto the raster canvas pltrs
  (Rust) keeps per figure; `buffer_rgba`, `copy_from_bbox` and
  `restore_region` work on it, and `RendererAgg` stands for
  it: `Artist.draw` with it draws the artist onto that canvas, which is
  how `Axes.draw_artist` and `Figure.draw_artist` blit (`axes/_blitting.py`,
  `_figure_interaction.py`). As in matplotlib, `FigureCanvasAgg` has no
  `blit` of its own: the toolkits' canvases, which put it before their
  widget's class, blit with the widget's (Qt's repaints the region), and
  pltrs' windows record the region for the window to show. The vector canvases' `draw` lays the figure
  out without output, as matplotlib's `draw_without_rendering`.
* `Figure.savefig` hands over to `FigureCanvasBase.print_figure`, which
  resolves the keywords (format, dpi, colours, `bbox_inches`), switches
  to the canvas of the format or of `backend=`, and calls its
  `print_<format>`; the built-in ones render with pltrs (Rust)
  (`_print_native`) and emit the `draw_event` matplotlib's `Figure.draw`
  emits. The area to save travels as `bbox_inches_restore`, the keyword
  matplotlib passes to print methods for it, and `savefig`'s
  `transparent` as the canvas's `_transparent` while saving. A print
  method's `metadata` goes to pltrs (Rust) with the other settings, which
  refuses it for the formats that cannot hold it, as matplotlib does.
  matplotlib writes its raster formats other than PNG and raw RGBA with
  Pillow; pltrs (Rust) encodes them, except GIF (whose palette and
  transparency are Pillow's), AVIF (whose bytes are libavif's in
  matplotlib, which pltrs (Rust)'s own AV1 encoder does not write) and
  any `pil_kwargs` but a JPEG `quality`: those are drawn by pltrs
  (Rust) (`_native.backends.print_rgba`) and saved by `image.imsave`
  through Pillow, as matplotlib's `_print_pil` saves them.
* Canvas callbacks belong to the figure (`figure._canvas_callbacks`), so
  that they survive a change of canvas.
* The renderer pltrs hands Python (`RendererAgg`, also what
  `fig._get_renderer()` gives under the vector backends) draws onto the
  figure's raster canvas: its drawing methods (see "Drawing defined in
  Python") draw what they are given onto it at once, between draws.

### rcParams

pltrs (Rust)'s `rc` module is the source of truth: `rcParams[key] = value`
spells the Python value as rc-file text (`True`, `None`, numbers as
Python prints them, sequences joined by commas, a `Cycler` as its repr)
and pltrs (Rust) validates it exactly as matplotlib does; `rcParams[key]` gives
the typed value back (floats, ints, bools, strings, lists of them; other
types, such as colours, keywords and cyclers, as their rc text).
`RcParams` instances other than `rcParams` (`rcParamsDefault`, copies,
`RcParams({...})`) hold a native `RcParams` of their own and only the keys
given to them. `backend` is kept in Python (no pltrs (Rust) parameter: the
backend is chosen by Python code). pltrs (Rust)'s own `pltrs.*` parameters are left
out of iteration, so that code iterating over rcParams sees matplotlib's
keys, but can be read and set by name. `rc_context` saves and restores
pltrs (Rust)'s current parameters.

matplotlib's validators (`rcsetup`, kept from matplotlib with its table
of parameters `_params_list`) give `rcParams[key]` matplotlib's Python
value: `RcParams._get` applies the parameter's validator to the rc text
pltrs (Rust) holds, so that a cycler comes back as a `Cycler`, a sketch as
a tuple, a colour as matplotlib keeps it. pltrs (Rust) keeps the value
normalised (`--` for `'dashed'`), so an `RcParams` also remembers the
value it was given with the rc text pltrs (Rust) made of it
(`RcParams._given`): while pltrs (Rust) still holds that text, the given
value is validated instead, and matplotlib's spelling comes back. The
defaults start with matplotlib's spellings (`_default_spellings`), and
`rc_context` restores the remembered values with the parameters.
`rcParams[key] = value` still validates through pltrs (Rust), which is the
source of truth; the validators are not run on assignment. pltrs takes
`cycler` from the `cycler` package, as matplotlib does (`rcsetup`
returns its `Cycler` objects).

A value matplotlib's validator takes and pltrs (Rust)'s type cannot
hold is kept in Python and read back as given, while pltrs (Rust) keeps
what it had: the marker rcParams (`lines.marker`, `scatter.marker`,
`boxplot.*.marker`), whose validator takes any string
(`RcParams._refused`). matplotlib fails where an artist takes the
marker, with `MarkerStyle`'s error: `Line2D()` and `plot` without a
marker check it (`markers._check_rc`, one set lookup); the Axes
matplotlib can no longer make then (its ticks are such lines) are still
made here.

Style sheets are matplotlib's `style` package over pltrs (Rust)'s
bundled sheets (`_native.rcsetup.bundled_styles`): `style.library` maps
names to `RcParams`, user sheets are read from `USER_LIBRARY_PATHS`
(`get_configdir()/stylelib`, in pltrs (Rust)'s configuration directory), and
`matplotlib._init_tests` adds `_classic_test`. Applying a library sheet
copies its rc text natively (`RcParams._update_from`, no Python round
trip). rc files are read by pltrs (Rust) (`_native.rcsetup.read_file`),
which reports the lines matplotlib logs.

### The configuration directory

pltrs (Rust) looks the configuration and cache directories up
(`pltrs::config`): `PLTRSCONFIGDIR`, else the XDG directories. pltrs
(Python) also reads matplotlib's own `MPLCONFIGDIR` when `PLTRSCONFIGDIR`
is unset, and matplotlib's rc files (`matplotlibrc` in the working and
configuration directories, `MATPLOTLIBRC`) after pltrs' `pltrsrc` at each
place (`config::honour_matplotlib_config_dir`, which `modules::register`
calls before anything looks the directories up), so that it works in
matplotlib's environment unchanged: a program's directory holding a
`matplotlibrc` styles its figures under both. When the directory cannot
be made or written to, a temporary one stands in, and importing the
package logs matplotlib's warning (`_log_temporary_config_dir`), naming
the variable that named the directory. The suite runner sets
`MPLCONFIGDIR` and removes the `PLTRSCONFIGDIR` cargo sets.

### The font catalogue

As matplotlib keeps its font list in its cache directory
(`fontlist-v390.json`), pltrs (Rust) keeps the catalogue of the system's
fonts in its own, `fontlist-v2.pltrs` (`pltrs::fonts::keep_catalogue`,
`pltrs_font::FontDatabase::add_system_fonts_catalogued`). The shared
text engine reads it when it is first used, which is when matplotlib's
`findfont` would first run; nothing reads it at import. A font file that
is new, or whose size or modification time changed, is read again, and a
catalogue that is missing, corrupted, of another version or format, is
rebuilt; it is written atomically, and when it cannot be written the
fonts are used all the same and `font_manager`'s logger warns as
matplotlib's `json_dump` does. Fonts added with `fontManager.addfont`
stay in the process, as matplotlib's do. `ttflist`, `findfont` and the
fallback chains are those of a process that read every font.

A face's file is opened when the face is first used, for metrics,
shaping or drawing: its tables are read then, but its glyph programs
(`glyf`, the CFF `CharStrings`) are read glyph by glyph as they are drawn,
and the tables pltrs draws without (bitmaps, variations) only when asked
for, as FreeType reads a file through matplotlib's file stream. A large
CJK font so costs the memory of the glyphs drawn, not that of its file.

A character no font of the chain has (with `font.enable_last_resort`
off; the last resort font maps every one) is laid out and drawn as the
first font's missing glyph, but not measured: `FT2Font.set_text` passes
over it, so a line's width, height and descent leave it out
(`FontChain::layout`).

### Grids, layout engines, legends

A `GridSpec` holds its native grid for its whole life: pltrs (Rust) and
its layout engines tell grids apart by identity, so a Python grid made
once and indexed many times places all its axes on one native grid, and
`gridspec._wrap` finds the Python object of a native grid pltrs (Rust)
made (`fig.subplots`, `add_subplot(2, 2, 1)` reuse grids as matplotlib's
`_check_gridspec_exists` does). The margins of a grid (`gs.left`, ...)
live in the native grid. `add_subplot` resolves its arguments with
matplotlib's `SubplotSpec._from_subplot_args` and adds the axes on the
native block; `subplot_mosaic` is matplotlib's, placing each label with
`add_subplot` on nested grids.

Layout engines (`layout_engine`) are matplotlib's classes holding their
parameters; a figure gives pltrs (Rust) the description of its engine
(`_native_spec`) when the engine is set or one of its parameters
changes (an engine knows the figures it is set on). The layout methods of
figures (`tight_layout`, the `align_*` methods, `suptitle` and the
super-labels, `get_tightbbox`) are the mixins of `_figure_layout`, over
`_native.figure_layout`. A `GridSpec` made without a figure is left out
of constrained layout, as matplotlib leaves it. pltrs (Rust) lays out
and solves constrained layout's grids; `ConstrainedLayoutEngine.execute`
returns them as matplotlib's does, its private `layoutgrids`, mirrored
as `_layoutgrid.LayoutGrid`s of the values solved (`Figure::execute_layout`,
`pltrs::layout::SolvedGrid`), which hold nothing alive. Ticks read
between draws (`get_ticklocs`, `get_ticklabels`) are computed where the
axes are, without running the layout engine, as matplotlib computes them
(`Axes::ticks`).

Axes locators of axes_grid1, insets and colorbars are described to pltrs
(Rust) (`_place`), which places their axes at every layout. Any other
callable `locator(ax, renderer)` (and a `Divider.set_locator`, a
secondary axis located by a transform other than `transData` or
`transAxes`) is Python that cannot run while pltrs (Rust) lays the
figure out: the root figure keeps what they place
(`axes/_program_locators`), and its layout hook
(`Figure::set_layout_hook`), which pltrs (Rust) runs before every layout
of a draw with no borrow of the figure held, calls them and places each
axes where its locator says until the next draw, as matplotlib calls them
when it draws.

Grids and layout engines of the program's classes (plotnine's): a grid
deriving from `GridSpecBase` alone gets a native grid of its own when
first needed, and an overridden `get_subplot_params` gives the margins
its cells are placed with (`get_grid_positions`, `SubplotSpec.get_position`,
and the native block an axes is added on, which takes them then); an
overridden `get_grid_positions` places the blocks itself. An engine whose
class has its own `execute` is a placeholder for pltrs (Rust), with its
`adjust_compatible` and `colorbar_gridspec`, and the same layout hook
runs its `execute(fig)` before the locators, as matplotlib's draw runs it
before it draws the Axes. So is an engine holding a parameter pltrs
(Rust) cannot hold (a string, a list, a rectangle of other than four
numbers, `_layout_checks`), which matplotlib's keeps and fails on as it
lays the figure out: its `execute` computes with the parameters as
matplotlib's does, raising its errors at the same steps (a `ValueError`
leaves the layout as it was, as matplotlib's draw passes over one), before
pltrs (Rust) lays the figure out.

A `Legend(parent, handles, labels, **kwargs)` makes the native legend of
its parent at once: matplotlib's argument handling (`_parse_legend_args`,
handler maps by handle and type) runs in Python, and the entries reach
pltrs (Rust) explicitly: an artist's handle (a loose line or rectangle
becomes a proxy drawn as it is now), a tuple with its `HandlerTuple`
layout, or the native handle of a container found on the axes
(`_native.legend.candidates`, which lists what matplotlib's
`_get_legend_handles` lists). Its `_legend_box`, the first of its
children, is made of `offsetbox` objects around the legend's artists
from the box pltrs (Rust) lays them out in (`_native.legend.legend_box`).
The box is laid out for the legend (`OffsetBox._laid_out_for`): it is on
the legend's figure and Axes, the program's settings on it reach the
legend, and its offset is the legend's `_findoffset` (where pltrs (Rust)
locates the legend) until `set_offset` pins it at a point
(`Legend::set_box_offset`), as `DraggableLegend`, matplotlib's own
`DraggableOffsetBox` subclass, does while the legend is dragged; setting
the location unpins it. A legend made for a figure (`Legend(fig, ...)`)
is drawn once it is in `fig.legends` (`_figure_artists.LegendList`),
where `Figure.legend` puts it, as matplotlib draws that list.

### Pickling

A figure pickles as an archive of its native state, since pltrs (Rust)
holds everything drawn and pickles load in other processes
(`test_pickle_load_from_subprocess`); the Python objects around it pickle
as they always do. Recording the calls that built a figure and replaying
them was ruled out: it cannot capture what pltrs (Rust) changes itself
(layout, autoscaling) nor artists changed through their natives.

* **pltrs (Rust)'s archive** (`pltrs_core::archive`): a type implements
  `Archive` (`encode`, `decode`) with `archive_struct!` or
  `archive_enum!` next to its definition, which list its fields, so that
  a field added later does not compile until it is archived too (a
  proc-macro derive was ruled out: proc macros need a host C toolchain,
  which the musl build avoids). Trait objects archive the concrete type
  behind them under a tag (`archive_kinds!(dyn Locator { "auto" =>
  AutoLocator, ... })`, next to the trait); an `Rc` reached twice reads
  back as one value. A figure archives whole (`Figure::to_archive`,
  `RawHandle::archive_figure`), removed artists included, under their
  ids, without what only lives while it is shown (canvas, callbacks,
  timers; draggable artists are connected again to the callbacks of the
  figure read back). Reading never panics on malformed bytes. The spec's
  `figure_archive` test archives every figure case before drawing it and
  checks that the figure read back draws the same pixels.
* **What an archive cannot hold** it refers to as *externals*, which the
  program archives beside the bytes. The binding's `ProgramKinds`
  (`modules/archive.rs`) archives its own types behind pltrs (Rust)'s
  traits as their Python objects (a Python `Formatter`, a `Norm`
  subclass, a transform, `Artist()`'s bare artist). A closure is
  archived as its origin (`ProgramFn`): the binding makes the closures
  of Python callables through `archive::tick_formatter`, `data_formatter`
  and `label_format`, which record the callable and how it is called, so
  that reading makes the closure again. A Rust closure without an origin
  is not archived (`TypeError: cannot pickle ...`, as for a lambda in
  matplotlib); functions pltrs makes itself are data (tick labels set by
  text, the spacing of a colorbar's blocks, colormap functions).
* **The package** (`pltrs/_pickling.py`): every native object pickles
  through `copyreg`. An artist's handle pickles as the `_FigureState` of
  its figure and its artist's id; a native value (a transform, a grid, a
  colorbar, a container) as an archive of its own, the handles inside it
  as the `_FigureState`s of their figures. A `_FigureState` carries the
  figure's archive: unpickled, it is first an empty native figure, which
  the handles of the pickle reach, then filled in place
  (`RawHandle::restore_figure`), so that the objects of a pickle may
  refer to each other in any order. Artists pickle their `__dict__`;
  `Figure` is matplotlib's `__getstate__` and `__setstate__` (no canvas,
  back into pyplot with `_restore_to_pylab`), its unpickled state made
  by `_pickling.restore_figure` and finished once its native state is
  read (`_FigureState.when_restored`). `copy.deepcopy` takes the same
  path.
* **Transforms of the trees** an artist was given (`ax.transData` as its
  `_given_transform`) pickle as references to their owner
  (`_pickling.TreeReference`), since the trees do not pickle: the artist
  takes the owner's rebuilt transform when the figure is restored, so
  that, as in matplotlib, both are one object again.
* **Shallow copies** (`copy.copy` of a line, patch, text, collection or
  image of a figure) get a native artist of their own
  (`RawHandle::detached_copy`, `_artist_copying.detach_shallow_copy`):
  owned by the Axes of the original but in none of its lists, kept apart
  from the figure's tree as a removed artist is, so that setting the copy
  leaves the original alone, as matplotlib's copy rebinds each attribute
  it sets. Artists holding others (Axes, figures, legends, contour sets)
  keep sharing them, as matplotlib's copies share the objects they hold.
* **Mappables read back still follow their norm.** matplotlib pickles no
  callbacks, but an unpickled image reads its norm when it draws, so a
  limit the program sets afterwards shows. Here the native artist holds
  the mapping, so `ColorizingArtist.__setstate__` registers the artist
  with its colorizer and the colorizer with its norm in
  `colors._follow_unheard` (weak, by identity), and both `_changed`s tell
  them (`_tell_unheard`). matplotlib's unpickled colorbar has lost its
  callback and stays as it was drawn: the artist is told inside
  `colorbar.leave_untold(True)`, which keeps the colorbars of pltrs
  (Rust) from following that one change.

### Imports inside the package

The suite and the `matplotlib` distribution load `python/pltrs` under the
name `matplotlib`, so:

* modules of the package import each other relatively
  (`from . import _handles`, `from ..figure import Figure`), never as
  `pltrs`;
* the toolkits (`pltrs/mpl_toolkits`) and `pylab.py`, which the suite loads
  as top-level packages, import pltrs by name (`from pltrs import colors`);
  `python/mpl_suite/pltrs_alias.py` (and the distribution's
  `_pltrs_matplotlib_alias`) makes that name resolve to the modules
  loaded as `matplotlib`;
* the native module is `from ._native import lines as _lines` (its
  submodules are attributes);
* importing pltrs loads the modules matplotlib's import loads (those pltrs
  has), and of its own only what defining the classes needs: a helper
  module a module uses only inside its functions, for what few programs
  do (matplotlib's code for unusual arguments, legends, contours, artists
  moved between figures), is bound with `_lazy_imports.module` and
  imported when first used. `python/tests/test_perf7_import.py` holds
  matplotlib's lists and a ceiling on pltrs' own count, and
  `benchmarks/import_cost.py` measures what an import costs;
* `get_data_path()` at import only locates the data directory (and logs
  it, as matplotlib's import does); its files are written when the path
  is next asked for, since writing them maps the pages of the extension
  around the bundled files. Code the package makes from source text at
  import (pyplot's wrappers, the patch styles' constructors) is compiled
  once and kept with the kwdoc tables (`_docstring.compiled`).

### Distributions

`pyproject.toml` builds pltrs (Python) with the tree's own PEP 517
backend (`python/backend/pltrs_backend`), written in Python's standard
library alone so that `pip install .` needs nothing but cargo. It builds
two distributions:

* `pltrs` (the default): the release build of `pltrs-python` and the
  package, a wheel tagged `cp312-abi3-<platform>` whose platform the
  backend reads from the library built (`_tags`: on Linux the libraries
  and symbol versions it needs, against auditwheel's policies; on macOS
  the oldest macOS each slice names; on Windows the architecture and the
  stable ABI's `python3.dll`), never from the Python running the build,
  and checks once written as an index would (`_audit`). `-C glibc=2.17`
  links it with Zig against an old glibc (`_linker`), for
  manylinux2014; `RELEASING.md` says how the release's wheels are
  built, tested and published. Linux wheels also install pltrs' desktop
  entry and icons as data under the prefix's `share` (`python/share`,
  which `crates/pltrs/examples/install_desktop_entry.rs` writes and
  `python/tests/test_desktop_entry.py` keeps equal to what
  `backend_pltrs.install_desktop_entry` writes), for GNOME, which takes
  window icons from there;
* `matplotlib` (`-C distribution=matplotlib`): `python/alias`, pure
  Python. Its modules load pltrs's sources under matplotlib's names
  (`_pltrs_matplotlib_alias`: a meta path finder doing what the suite's
  site and `pltrs_alias.py` do), so that code importing matplotlib by
  name runs on pltrs, and pltrs's objects name matplotlib's modules
  (`type(fig).__module__`, reprs, pickles, loggers), which libraries
  read: arviz-plots picks its plotting backend by the module of a
  figure's class. pltrs's names (`pltrs.x`, which the toolkits import)
  are aliases of those modules. A program importing pltrs before
  matplotlib keeps pltrs's names, matplotlib's then being the aliases.
  The `pylab` module is matplotlib's own. It is named `matplotlib`
  for pip: what requires matplotlib is satisfied by it, and it replaces
  a real matplotlib (or is replaced by one) openly.

`cargo xtask ecosystem` runs pandas and seaborn on the tree through the
alias (`python/ecosystem`), comparing each figure's structure with what
real matplotlib recorded (`expected.json`, recorded with
`ECOSYSTEM_RECORD=1` under matplotlib). `cargo xtask ecosystem suite`
runs the libraries' own suites (`xtask/src/ecosystem/libraries.rs`), the
tests of each that reach matplotlib, from their sdists, or the source
archives of the releases' tags where the sdists ship no tests
(mplfinance, matplotlib-venn) (`ecosystem setup` downloads them into
`target/refs/`, with the data files the sdists leave out, checking their
SHA-256), as `mpl-suite` runs matplotlib's and with its outcome files and
report: in a venv with real matplotlib (`--reference`,
`target/ecosystem/reference-venv`) and in one without it, through the
alias (`target/ecosystem/venv`, installed with matplotlib excluded from
the resolution), or in such venvs of the library's own where its pins
clash with the others' (`target/ecosystem/venvs/<library>`: lifelines'
pandas 2, scanpy's pytest 9), with the core metadata of the
`matplotlib` distribution beside the alias (scanpy reads matplotlib's
version through `importlib.metadata`). Each library runs in a site of its own laid out as its
source tree, with the sdist's `pyproject.toml`, so that its pytest
configuration applies (pandas turns every warning into an error); the
outcomes are `spec/ecosystem/<mode>.json`. What they need of matplotlib
beyond its suite lives where the rest of its kind does: an artist's
`sticky_edges` (lists whose changes reach pltrs (Rust)), an axes'
`_get_lines` and `_get_patches_for_fill` (`axes/_prop_cycles.py`: giving
one axes another's cycle makes both draw from it, `Axes::share_prop_cycle`),
artists made outside any figure as legend handles (`LegendHandle::of_artist`:
lines, patches of any shape and collections; also the handles of a legend,
or of one removed, which seaborn's `move_legend` makes a legend of), and the
ticks an axis makes taking what its first tick has of its own, as
matplotlib's `_copy_tick_props` gives it them (`Axis::make_ticks`). Axes
sharing an axis share its units and tick helpers but not its converter
(`Axis::own_converter` is `get_converter`); the program's changes to
`colors._colors_full_map` name colours natively too
(`pltrs::colors::set_color_name`, seaborn's `set_color_codes`); a
`zoneinfo.ZoneInfo` crosses with the `TZif` data `zoneinfo` reads for it
(from the system or the `tzdata` package, whose files are slim) and is
read as `zoneinfo` reads it, `TZ` string included (`TimeZone::Info`); and
tick helpers ask a program's stand-in axis only what matplotlib's would
(seaborn's legends and `PseudoAxis`: `TickHelper._axis_info`).

The libraries' image comparisons measure pixels against baselines of the
same matplotlib: `ecosystem setup` makes the baselines plotnine, astropy
and mplfinance ship for other versions again under matplotlib 3.11.2, in
the reference venv, from the pinned versions alone
(`target/ecosystem/baselines`, never committed; `xtask/src/ecosystem/baselines.rs`):
astropy's with pytest-mpl's `--mpl-generate-path` (its stored hashes are
of PNG bytes, which measure the PNG encoder rather than the drawing),
plotnine's and mplfinance's from the images their tests save, run again
until none is missing, as plotnine's `tools/update_baseline_images.py`
does. The suites then compare with their own comparison and tolerances
(`--mpl --mpl-baseline-path` for astropy, a link to the baselines where
the others read theirs). What drawing them showed is matplotlib's too: a
tick label keeps the place across its axis the program moved it to
(`TickStyle::label_across`), tick lines measure as lines, an anchored box
of the program's places its child where its own `get_offset` says
(`AnchoredOffsetbox::set_child_offset`, before every layout) and measures
the boxes of the program's inside it first, collections measure the union
of their shapes, an artist a program's `draw` draws more than once is
drawn each time as it was then (`DrawnArtist`), the paths a collection
hands out reach it again when the program sets their vertices
(`_own_state`), an Axes class with its own `tick_params` keeps the ticks
of matplotlib's `Tick` (`axes/_program_tick_params.py`), and tight layouts
ask Axes classes measuring themselves (astropy's `WCSAxes`).

The libraries of the sciences (cartopy, nilearn, sympy, arviz's plots),
of machine learning (shap, missingno) and mpl-scatter-density run as the
others do. The data their tests read that neither the source tree nor
the package holds, cartopy's Natural Earth shapefiles and shap's
datasets, is fetched once by `ecosystem setup`, each file checked
against its SHA-256 (`Library::data`, `xtask/src/ecosystem/data.rs`), into
`target/ecosystem/data/<library>`, which the tests find by a variable
(`CARTOPY_DATA_DIR`) or a link where the library caches its downloads.
Their runs are offline (`Network::Refused`): HTTP and HTTPS go through a
proxy that refuses every connection, so that a test fetching more fails
in both modes alike. A suite parametrized over a set collects its tests
in an order that differs between processes, which pytest-xdist refuses;
`python/ecosystem/plugin/ecosystem_order.py` sorts the tests of each file
by their ids (nilearn). What these libraries need of matplotlib is the
projections and artists of a program's own: a spine of no side
(cartopy's `GeoSpine`: `SpineArtist::of_no_side`, framing the Axes as its
path), an Axes class's own `autoscale_view` run whenever its data limits
change (`_program_locators.py`, after the native autoscale reports the
Axes it left stale), transform-givers (`_as_mpl_transform`) for text,
markers, meshes and contours, a contour set keeping the transform it was
given while its data limits come through it, an Axes class constructed
with the arguments its projection object gives (`_as_mpl_axes`) for
insets too, the private registers libraries read (an artist's `_axes`,
`_artist_private.py`; a figure's `_axstack`, `_axes_stack.py`), arrays a
program changes in place reaching pltrs (Rust) (`_own_state`:
`Poly3DCollection._facecolor3d`), and `MaxNLocator._raw_ticks` as the
place a subclass changes its steps (cartopy's `LongitudeLocator`).

### Type information

pltrs is typed as matplotlib is, so that a type checker reads a program
written for matplotlib the same under pltrs: `py.typed` and the stubs
matplotlib installs lie beside the modules they describe, written by
`python/tools/matplotlib_stubs.py` from matplotlib's source tree, never by
hand. matplotlib's stubs import its modules absolutely
(`from matplotlib.text import Text`), and type checkers read absolute and
relative imports differently (pyright takes `from . import x` in a stub for
a re-export, and a package's relative imports in its `__init__` for
importing those submodules), so each distribution has its own copy:
`python/pltrs/**/*.pyi` import pltrs' modules for matplotlib's, and
`python/alias/matplotlib/**/*.pyi` are matplotlib's as it installs them.
The alias also has stubs of matplotlib's modules without one (`units`,
`dates`, the backends: matplotlib's code without function bodies, but for
what methods assign on their instance), and installs beside each stub
pltrs' module under matplotlib's name, as checkers look for a stub's
source (pyright warns of one without, and shows its docstrings), and
pltrs' toolkit modules beside its toolkits' loaders, whose
`__init__.pyi` are matplotlib's code (`Project.checked_sources`); Python
never imports them, as the alias loads pltrs' files. pltrs' own toolkits
(`pltrs.mpl_toolkits`) have no stubs: `py.typed` covers them, and PEP 561
has no marker for an untyped part of a typed package. pyright reads an
untyped package's code, so it reports on pltrs' toolkits what it reports
on matplotlib's; mypy, which takes an untyped package's names for `Any`
and reports its import (`import-untyped`), reads pltrs' code instead.
Stubs of a module `__getattr__` alone (every name `Any`) would give
mypy's reading but silence pyright, and still no `import-untyped`: the
larger difference.

Instance attributes matplotlib's code keeps plainly (`Wedge.r`,
`ContourSet.zmin`) and pltrs (Rust) holds are properties with a setter
with matplotlib's effect: to pltrs (Rust) where matplotlib's draws read
the attribute (`RegularPolygon.xy`, `Arc.theta1`, `AxesImage.origin`,
`axison`), kept on the object (`_values.assign`) where only a later method
reads it (a `Wedge`'s fields at its next setter, a `StepPatch`'s
orientation at `set_data`) or none (a contour set's extremes). A patch assigned to a part pltrs (Rust) draws and
matplotlib re-places at every draw (a legend's `legendPatch`, an
annotation's `arrow_patch`) gives the part its look and becomes its
object (`_handles.adopt`, `_annotation_arrow.adopt`); `None` where
matplotlib's next draw calls the part's methods is kept as the problem
that draw and measure fail on (`_unusable`). matplotlib
types `pyplot` inline, which pltrs' `pyplot.py` (wrappers made from source
text) does not: `pyplot.pyi` is matplotlib's module without docstrings and
function bodies, with every public name it binds in `__all__`, as mypy
exports a module's imports but a stub's only where it says so.

`cargo xtask diff --types` (`spec/diff/stubs.py`) holds the stubs against
pltrs at run time with `mypy.stubtest`, as matplotlib's CI holds them
against matplotlib (its allowlist and the lines its `tools/stubtest.py`
derives): what matplotlib's own run does not report is a difference of
pltrs' public names (an attribute missing, a property where matplotlib
has a plain attribute, a name matplotlib has not). Instance attributes the
stubs declare are read and assigned as matplotlib's are where pltrs keeps
them natively: a property with a setter, or a descriptor that steps aside
(`_transform_trees.TreeAttribute`); settings that change the tree a box
draws are properties (`_offsetbox_trees.TreeSetting`).

### Jupyter

`cargo xtask notebooks` executes the notebooks of `spec/notebooks` in real
Jupyter kernels (nbclient, ipykernel), once in a venv with real
matplotlib and once with pltrs as matplotlib (`python/alias`), and
compares everything the kernels send (`xtask/src/notebooks`): every
output (PNG and JPEG pixels within matplotlib's RMS tolerance, SVG and PDF
item by item, reprs, HTML, metadata, streams, errors) and the comm
messages of ipympl's widget and of nbAgg, for which the driver
(`execute.py`) plays the browser: it answers the kernel as their client
scripts do and sends the mouse, key, toolbar and resize messages a cell's
metadata lists. pltrs differs by design in its module names and its own
`pltrs` backend in lists of backends, which the comparison reads as
matplotlib's. `cargo xtask notebooks suite` runs the matplotlib tests of
IPython, matplotlib-inline and ipympl, and matplotlib's own notebook tests,
under both. What Jupyter relies on lives where matplotlib keeps it:
pyplot connects to IPython's `post_execute` event (`install_repl_displayhook`,
from `switch_backend`, `ion` and `ioff`), which redraws stale figures after
every cell and every comm message; a figure's Axes observers (its manager's
toolbar) hear of every Axes added; matplotlib's staleness side effects that
the widget backends turn into messages are kept (a sized collection drawn
alone leaves its Axes and figure stale; raster prints time the toolbar's
wait cursor; vector canvases leave their figure at 72 dpi once drawn); and
a Jupyter kernel runs pltrs' windows between its messages
(`_pltrs_loop._kernel_loop`, `%matplotlib pltrs`).

### Sphinx

matplotlib's Sphinx extensions (`sphinxext`: the plot directive,
`mathmpl`, `figure-mpl` and the roles) are matplotlib's modules, changed
only where they import the package, and line for line matplotlib's, as
the tracebacks of a failing plot in a build's warnings show their lines.
They run pltrs through its public API, as they run matplotlib, so a
difference in a documentation build is a difference of pltrs', fixed
there. `python/tests/test_sphx1.py` builds `python/tests/sphinx_project`,
which uses every option and configuration value of the extensions, under
matplotlib and under pltrs as matplotlib, and compares the warnings, the
files, the HTML and the pictures (`sphinx_builds.py`, which also serves
to compare real projects' documentation); matplotlib's own
`test_sphinxext.py` runs in the suite. The plot directive resets the
rcParams with `rc_file_defaults` before each plot, which gives the look's
parameters (`rcParamsOrig`): a documentation build draws in pltrs' modern
look unless `PLTRS_LOOK=matplotlib` is set, while sphinx-gallery, which
resets with `rcdefaults`, draws in matplotlib's.

### Apple silicon

pltrs agrees with numpy and matplotlib on the machine it runs on, not
with itself across machines. On arm64 macOS the reference is one set of
binaries: the wheels PyPI serves for the platform, matplotlib 3.11.2
(`matplotlib-3.11.2-cp314-cp314-macosx_11_0_arm64.whl`, SHA-256
`1a3040b2…5e2863`), contourpy 1.4.0 (`…072a702e…f3c23d`) and numpy 2.5.4,
with Pillow 12.3.0 for images. They differ from the x86-64 wheels in four
ways, each followed there and only there; the x86-64 results stay bit
for bit what they were, and a macOS x86-64 build (which Rosetta runs
beside the x86-64 wheels) follows x86-64.

* **Fused products.** clang compiled the wheels, and clang contracts a
  product and the sum or difference it feeds into one fused multiply-add
  wherever both stand in one expression (`-ffp-contract=on`, its
  default); GCC's x86-64 wheels round the product first. The results
  differ in the last bit, which decides which diagonal splits four
  cocircular points, on which side of an edge a point is, whether three
  points are collinear, and the last digit of every stroked, clipped or
  resampled coordinate. `pltrs_core::numeric::FUSES_PRODUCTS` is true for
  `cfg(all(target_os = "macos", target_arch = "aarch64"))`, and
  `numeric::fused(a, b, c)` is `a.mul_add(b, c)` when it is and
  `a * b + c` when it is not (`sum_of_products`, `difference_of_products`
  and `fused_f32` are built on it). A port writes each contracted
  expression through them, in clang's operand order, so the code reads
  the same on every platform. The rule is syntactic, which makes the
  order certain: of a sum or difference, the left operand is fused if it
  is a product, otherwise the right one; `a * b + c * d` is
  `fused(a, b, c * d)`, `c - a * b` is `fused(-a, b, c)`, `x += a * b` is
  `x = fused(a, b, x)`. A product kept in a variable, converted to
  another type or reached through an overloaded operator (`_tri`'s `XY`
  arithmetic in `TriContourGenerator::interp`) is rounded first, and one
  by a power of two is exact either way. Fused are:
  - qhull's geometry as the Delaunay triangulation uses it
    (`tri/delaunay`), `_tri` (`XY::cross_z`, the plane coefficients) and
    contourpy's interpolation of a crossing in its three algorithms;
  - Agg and matplotlib's path code (`pltrs-core`'s `path`): the affine
    transform of vertices and its composition and inverse
    (`Affine2D::agg_transform_point`, `agg_then`, `agg_inverted`:
    matplotlib's Python transforms are numpy's and are not fused), curve
    subdivision, the stroker's joins, caps and miters, simplification,
    sketching, clipping to a rectangle, the intersection tests, quadratic
    to cubic conversion, Gouraud triangles, and the image resampler's
    filter kernels and floating-point accumulation and blending
    (`pltrs-raster`);
  - numpy's own loops, built the same way: `arange`'s fill and `interp`
    (`pltrs::numeric`);
  - HarfBuzz's interpolation of `trak` tracking. Nothing else in
    `ft2font` changes a value: FreeType computes in integers, and the
    remaining fused instructions are HarfBuzz's variable-font and paint
    code, which matplotlib's shaping does not reach, and two products by
    1/64.
* **Accelerate.** numpy's wheel links Apple's BLAS, whose small products
  round in ways of their own (`numeric::ACCELERATE_BLAS`): a matrix
  product takes its columns four at a time, fusing each row's products in
  pairs, and fuses in order what is left of them; a matrix times one
  vector and the dot of two vectors sum rounded products (four of them in
  pairs). `Affine2D::then` (3×3 products) and `mplot3d::math` (the
  projection of points, 4×4 products) follow it; matrix products of
  other shapes whose rule was not found (three rows by four columns and
  more) and LAPACK's inverse are not.
* **Sorting.** numpy has no SIMD `argsort` off x86, so ties come out in
  the order of its own introsort (`numeric::argsort::generic`).
* **Conversions.** A float no integer holds saturates on arm64 where
  x86-64 gives the smallest integer (`numeric::np_int`), and Pillow's
  libjpeg-turbo narrows the sums of corrupt JPEG blocks as its Neon
  routine does (`pltrs-codec`'s `jpeg::decode::idct`).

`python/tools/fused_sites.py` finds the fused expressions again for a new
release's wheels (on an arm64 Mac, with Xcode's tools alone):
`fused_sites.py wheel FILE.whl` says which wheel it read, with its
SHA-256, and counts the fused instructions of every function of its
extensions (3.11.2: `_qhull` 285, `_tri` 22, `_path` 235, `_image` 339,
`_backend_agg` 353, `ft2font` 145, contourpy 64; the wheels are linked
with link-time optimisation, so a function's count includes what was
inlined into it); `fused_sites.py sources ...` compiles the same sources
with `-O3 -g` and prints the function, the source line and the
expression of each fused instruction, inlined code under the line it
came from. A line only printing, another dimension or an option
matplotlib does not set reaches is left alone. Other builds of
matplotlib for the platform (conda-forge, Homebrew, a source build with
other flags) may contract elsewhere, and GCC's builds for arm64 Linux
contract across statements too; pltrs follows the PyPI wheels for macOS.

The spec's goldens are matplotlib's on Linux x86-64, and most of what
the Mac's matplotlib computes otherwise stays within what pltrs is
allowed against them (values within 1e-9, images within an RMS of 1).
Where matplotlib's own output there fails that check (contour vertices
and triangulations that take another branch, the order of equal values,
a few images of them), it is recorded there as that platform's golden
(`spec/goldens/platforms/aarch64-apple-darwin`, see CLAUDE.md and
`spec/reference/spec_support/golden.py`), and the spec is green on the
Mac against both. The fixture tests of `pltrs-core` and `pltrs-raster`,
which compare bit for bit, have that platform's fixtures beside the
common ones (`tests/fixtures/aarch64-apple-darwin`, written by the same
`generate.py` run there), and smoke tests pinning such a value hold both
(`python/tests/test_platform_arithmetic.py`). The trapezoid map shuffles
its edges in libc++'s order there (`tri/finder/shuffle.rs`). What is not
followed is in `spec/diff/allowlist.txt` with the proof: Agg's rasterizer
running into its cell limit on saturated coordinates, numpy failing to
allocate what a saturated date asks for, and the sign of rounding noise
in numpy's arm64 build of pocketfft.

### aarch64 Linux

Docker on an Apple-silicon Mac, Graviton and the Raspberry Pi run
aarch64 Linux, whose reference is again its own binaries: matplotlib
3.11.2's and contourpy 1.4.0's `manylinux_2_28_aarch64` wheels, built by
GCC 14.2.1 (Red Hat 14.2.1-11, the compiler of the
`quay.io/pypa/manylinux_2_28_aarch64` image), and numpy 2.5.4's. GCC
contracts products into sums too, but by other rules than clang, so the
contraction is named per platform, `pltrs_core::numeric::Contraction`
(`Unfused`, `Clang`, `Gcc`, the constant `CONTRACTION`), and
`FUSES_PRODUCTS` is `CONTRACTION.fuses()`. For musl the same holds:
matplotlib has no musl aarch64 wheel, and a source build there is GCC's.

* **Across statements and inlined operators.** GCC's default for C++ and
  GNU C (`-ffp-contract=fast`) fuses a product into the sum that uses it
  wherever they meet after inlining, where clang only fuses within one
  expression. `_path.affine_transform`, through which every affine
  transform of matplotlib's Python maps arrays, keeps its products in
  variables (`t0 = sx * x; t1 = shx * y; t = t0 + t1 + tx`): clang
  rounds them, GCC fuses the first. So `Affine2D::transform_point`
  follows `CONTRACTION` (`transform_point_as`), and under GCC it is
  Agg's `trans_affine::transform` (`agg_transform_point`) bit for bit:
  a caller mirroring either agrees there, and the split between the two
  matters on arm64 macOS alone. `_tri`'s `XY` arithmetic is fused through
  its operators: the crossing of a tricontour
  (`TriContourGenerator::interp`, `tri::contour::interp_point`) and the
  trapezoid map's enclosing box (`tri::finder::expanded`).
* **Another operand.** Which product of a sum GCC fuses follows the order
  its `widening_mul` pass meets them, not the syntax, and depends on the
  function a computation is inlined into: in
  `Triangulation::calculate_plane_coefficients` the cross product's x and
  y components fuse their second product and the z component its first,
  where the out-of-line `XYZ::cross` fuses each first
  (`tri::triangulation::plane_as`), and Agg's `calc_intersection` fuses
  its numerator's second product where `_draw_gouraud_triangle` dilates a
  triangle (`outline::calc_intersection_fusing`, `pltrs-raster`'s
  `render::gouraud`). `numeric::FusedProduct` names the product a port
  fuses (`Contraction::difference_of_products`).
* **Not in ISO C.** GCC does not contract ISO C at all, and numpy builds
  its C as C11: `arange`'s fill and `interp` are not fused on aarch64
  Linux (`numeric::fused_iso_c`, fused on arm64 macOS only, as clang
  contracts ISO C too). numpy fuses there only in its SIMD kernels
  (`tanh`, float32 `sin` and `cos`, complex `multiply`, einsum's sums of
  products), as on every aarch64 (`mlab::Complex::mul_at`).
* **The same as clang** where GCC's GIMPLE says so: qhull's distances and
  hyperplanes (built as GNU C, so contracted), contourpy's interpolation
  of a crossing in its three algorithms, `XY::cross_z`, the dot product of
  the planes and Agg's `trans_affine::transform`.

Found as for macOS, then confirmed: `python/tools/fused_sites.py gimple
SOURCE...`, run in the image above (it has the compiler), compiles
matplotlib's sources with the wheels' flags (`-O3`,
`-fvisibility=hidden`) and prints GCC's `.FMA`/`.FMS`/`.FNMA`/`.FNMS`
statements per source line and function, with the debug binds that name
their operands; a reading is settled by an exact emulation of it
(`Fraction`: one rounding per fused operation) against the wheel on
seeded inputs under emulation (the plane coefficients: 3,596 of 3,596
triangles, against at most 2,663 for any other pattern). The tests of
each such site evaluate all three compilers' patterns
(`Contraction::fused`, `plane_as`, `transform_point_as`, `interp_point`,
`expanded`), with matplotlib's own values where it has them. The arm64
casts (`numeric::np_int`) and the Neon IDCT of libjpeg-turbo
(`pltrs-codec`) follow the processor and hold on aarch64 Linux too;
Accelerate does not (numpy links OpenBLAS there, whose kernels are not
followed). Goldens of the target are in
`spec/goldens/platforms/aarch64-unknown-linux-gnu/` (`PLATFORM` in
`pltrs-spec`), and the fixtures of `pltrs-core` and `pltrs-raster` in
their `tests/fixtures/aarch64-unknown-linux-gnu/`, written by the same
scripts run there. Not followed there: Gouraud shading, where
matplotlib's aarch64 build interpolates the colours otherwise than its
x86-64 one in most triangles for a reason that is no fused product (a
rebuild of its `_backend_agg` with the wheel's flags reproduces the
wheel, one with `-ffp-contract=off` and `-fsigned-char` still differs;
`agg_reference`'s tolerance for `gouraud_clip_path`), the extrema of
curves (`np.roots`, LAPACK), OpenBLAS's kernels and, likely, the
complex products numpy's FFT fuses there (one value of `mlab.csd` in
`number_types`' sweep is one ulp off matplotlib's; task REL4). The release
workflow's containers compare `python/tools/drop_in.py`'s drawings with
matplotlib's there (`check_dist.py --compare-drawings`), pixel for pixel.

### Pythons and dependency versions

As on each platform, pltrs agrees with matplotlib on each Python and
dependency version it supports (CPython 3.12 to 3.14, `pyproject.toml`'s
lower bounds to today's releases), which the release workflow runs the
smoke tests on.
Where matplotlib's value or message is the interpreter's or a
dependency's own (Python's wording of an error, numpy's reprs, scalar
promotion and warnings, python-dateutil's reading of a zone file), pltrs
gives what matplotlib gives with that version, and a smoke test pinning
it asks the environment (`python/tests/build_facts.py`: the message of
the same operation on a plain object, a fact probed of numpy or
python-dateutil) rather than pinning one version's. pltrs (Rust) is told
at import what it models of numpy (`pltrs::warnings::set_products_warn`:
whether `np.dot` and `@` warn), and asks python-dateutil 2.7's zones,
which round their offsets to whole minutes, through Python
(`_date_locators._dateutil_keeps_offsets`).

### Processor levels

numpy's wheels for x86-64 are built for one baseline (`X86_V2`: SSE4.2)
and hold two more copies of their hot loops, for `X86_V3` (AVX2 with FMA)
and `X86_V4` (AVX-512), of which numpy picks the best the processor runs
when it is imported. The copies do not compute the same, and matplotlib's
results follow, so "the machine it runs on" means the processor too:
pltrs computes at numpy's level, `pltrs_core::numeric::CpuLevel`
(`Baseline`, `V3`, `V4`), resolved once per process.

* **Where the level comes from: the numpy that is installed decides.**
  The level is what numpy dispatches to, not what the processor could
  run. `pltrs/__init__.py` hands `_native.numeric.follow_numpy`, before
  anything is computed, whether numpy runs its copies for the groups
  `X86_V3` and `X86_V4`: the processor has the group (`__cpu_features__`,
  which a user's `NPY_DISABLE_CPU_FEATURES` empties) and numpy holds a
  copy for it (`__cpu_baseline__` and `__cpu_dispatch__`). A wheel may
  hold none: numpy's for Windows has no `X86_V4` copy (its build leaves
  the group out for MSVC), so numpy and pltrs run at `X86_V3` there on an
  AVX-512 processor, while its Linux wheels dispatch to both groups in
  every release from 1.26 to 2.5 (numpy before 2.4 has no groups and
  dispatches the same copies by the features `AVX2` and `AVX512_SKX`,
  which are read then, the same way). pltrs (Rust) on its own asks the
  processor (`is_x86_feature_detected!`) with numpy's rule for the groups,
  unless `PLTRS_CPU_LEVEL` names a level (`v2`, `v3`, `v4`). Every other
  architecture is `Baseline`: arm64 macOS computes as before. A kernel
  that depends on the level reads the resolved value once per call
  (`argsort_at` takes the level as an argument, for tests).
* **What follows it: the sort.** numpy's unstable `argsort` sorts indices
  with its own introsort at the baseline and with Intel's x86-simd-sort
  at `X86_V3` and `X86_V4`, compiled once for registers of 4 keys and
  once for 8 (`pltrs::numeric::argsort`: one port over both widths). Up to
  256 values the two compilations agree (one bitonic network); above,
  their partitions leave ties in another order, and with them the order
  matplotlib draws 3-D points and faces of equal depth in
  (`np.argsort(depths)[::-1]`), the labels a contour keeps, and whatever
  else sorts by `argsort`. `crates/pltrs/tools/argsort_digests.py` holds
  the proof against numpy (every length to 600 and a spread to 10⁶, at
  the three levels) and checks a processor's own level with numpy alone
  (`--check`).
* **The sort by numpy's release: numpy sorts the long arrays itself.**
  The x86-simd-sort numpy vendors changed twice, and the order of equal
  values with it: numpy 1.25 and 1.26 have no AVX2 sort (with AVX2 they
  sort as without), leave ranges of up to 64 keys to the networks and
  partition longer ones; numpy 2.0 to 2.2 have the AVX2 sort and networks
  for 256 keys; numpy 2.3 to 2.5, which pltrs (Rust) replays, also leave
  an array already in order alone (the tool's digests of numpy 1.26.4,
  2.0.x, 2.1.3, 2.2.6, 2.3.x, 2.4.4 and 2.5.4 fall into these three sets
  at `X86_V3` and `X86_V4`, and into one without AVX2). All of it is in
  arrays of more than 64 values: up to there every release sorts by one
  network, or by numpy's own introsort where it has no SIMD sort. So
  pltrs (Python) does not replay numpy's sort of those: `pltrs/__init__`
  registers `numpy.ndarray.argsort` (as `"argsort"`), and
  `pltrs::numeric::argsort` has it sort every array of more than 64
  values (`argsort/numpy.rs`, `LONGEST_REPLAYED`): the order of the numpy
  at hand whatever its release, and sooner than the replay from about
  that length on (numpy sorts with the processor's registers, the replay
  lane by lane: several times sooner from a few thousand values on). The
  call is of numpy's C method, so it makes no Python-level
  call and runs no code of the program's; the GIL is held wherever pltrs
  (Rust) sorts, and numpy releases it while it sorts, as it does under
  matplotlib (`python/tests/test_threads.py`). Shorter arrays are
  replayed, which costs less than the call, and numpy is asked nothing
  about them until one comes out with equal values (any other has one
  order, whoever sorts). Then, once per process, numpy sorts four seeded
  arrays of up to 64 values: if the replay at the processor's level
  orders them as numpy does, it goes on; if numpy's introsort does (numpy
  1 with AVX2), that is replayed; if neither does (a build with another
  sort), numpy sorts the short arrays too. Arrays with NaN go to C++'s
  `std::sort`, which is the C++ library's and not numpy's, so they are
  asked about, and routed, on their own. `_native.numeric
  .argsort_routes()` says what was decided. pltrs (Rust) alone has no
  numpy: it orders equal values, at every length, as the numpy the
  goldens are recorded with does (2.5).
* **What does not follow it: the maths.** Only numpy's Linux wheel links
  Intel's SVML, and only its AVX-512 loops call it: at `X86_V4` on Linux
  `exp`, `log`, `log2`, `log10`, `exp2`, `expm1`, `log1p`, `power`,
  `cbrt`, `tan`, the inverse trigonometric and the hyperbolic functions
  of float64 arrays come from SVML and differ from the C library's in the
  last bit of some results: `log10` 38%, `log` and `log2` 31%, `power`
  3 to 7%, `exp` 3%, `sinh` 18%, `arcsinh` 13%, `arcsin` and `arccos`
  71%, `arctan2` 3%, `arctan` 0.1% (10⁵ values each; float32 too, except
  `exp`, `log`, `sin` and `cos`, which numpy computes itself at both
  levels). Reductions, `sqrt`, `sin`, `cos`, `hypot`, the casts,
  `linspace`, `arange`, `interp`, `histogram` and the FFT are the same
  bits at every level. Everywhere else (`X86_V3`, `X86_V2`, Windows,
  macOS) numpy calls the C library, as pltrs (Rust) does. pltrs computes
  with the C library at every level: one bit off at `X86_V4` on Linux,
  which stays within what the spec and the differential tester allow
  (1e-9, 1e-6). Of the spec's 3,402 images, 19 differ between the levels
  under matplotlib itself (log and asinh scales, contours, streamplots,
  hexbin; the largest RMS 0.16), none past the tolerance, and no
  structural fact or vector document differs. The float64 kernels numpy
  calls (`__svml_*8_ha`, the `_ha` files of its `svml` directory) carry
  an "INTEL CONFIDENTIAL" notice of their own beside the directory's
  BSD-3-Clause licence: nothing of them is ported.
* **Where one bit decides: numpy's own values.** A rounding that feeds
  something discrete shows: the power an `AsinhLocator` rounds a tick to
  (`floor(log(sinh(...)))`), and the boundaries a colorbar draws for a
  `LogNorm`, of which an edge on a pixel's centre falls to one pixel or
  the next. At those two places pltrs (Rust) asks for the values through
  `pltrs_core::numeric::Ufunc::map`, and pltrs (Python) puts numpy's own
  ufuncs behind it when it is imported (`_native.numeric.use_ufuncs`; no
  table of pltrs' own), so that matplotlib's decision is reproduced
  whatever numpy runs. `AsinhLocator.tick_values` makes four calls
  (`arcsinh`, `sinh`, `log` or `log10`, `power`) and `LogNorm.inverse`
  three (`log10` of the limits, `log` of the base, `exp`), of a few
  microseconds each: the ufuncs matplotlib calls at the same places, as
  often, and no figure without an asinh axis or a `LogNorm` reaches them.
  pltrs (Rust) alone computes them with the C library; the spec waives
  the three `AsinhLocator` probes for it at `X86_V4` and passes them at
  the lower levels.
* **numpy's BLAS follows the processor too, and is not followed.**
  OpenBLAS picks its kernels by the processor (`SkylakeX` with AVX-512,
  `Haswell` with AVX2), which changes long dot products, large matrix
  products and LAPACK's results (inverses, solutions, eigenvalues) in the
  last bits; the 3×3 and 4×4 products pltrs replays are the same in both.
  In the spec that is one probe (a Bézier curve with control points at
  1e17), waived at the lower levels, and no figure; as with LAPACK on
  arm64 macOS, replaying OpenBLAS per kernel does not pay. A machine held
  to a level for a reference run is held to its kernels all the same
  (`OPENBLAS_CORETYPE`), so that the reference is a faithful one.
* **The baseline.** Without AVX2 numpy's complex multiply rounds both
  products where its AVX2 and AVX-512 loops fuse one
  (`pltrs::mlab::Complex::mul_at`: `psd`, `csd`, `cohere`), and float32
  `exp`, `log`, `sin` and `cos` come from the C library.
* **aarch64.** numpy's aarch64 wheel dispatches too, from its ASIMD
  baseline to `ASIMDHP`, `ASIMDDP`, `ASIMDFHM` and `SVE`, but none of
  these copies computes another value than the baseline: with each of
  them switched off in turn (`NPY_DISABLE_CPU_FEATURES`, under emulation
  of a processor that has them all) its `argsort`, `sort` and `partition`
  of ties, its maths ufuncs, products and reductions are the same bits.
  Its `argsort` is the generic introsort, whose ties fall as at x86-64's
  baseline, and pltrs computes at `Baseline` there, as it does on arm64
  macOS. Its maths differ from x86-64's at every level (they are the C
  library's of the platform), and OpenBLAS picks kernels per core there
  as well, which is not followed.

The spec's goldens are recorded at `X86_V4`. What matplotlib computes
otherwise at a lower level, and pltrs' check refuses, is that level's
golden (`spec/goldens/platforms/x86_64-v3`, `x86_64-v2`; see CLAUDE.md),
looked up after a target's own goldens and before the common ones;
`cargo xtask spec --level v3` and the smoke tests of
`python/tests/test_cpu1.py` run the lower levels on an AVX-512 machine.

### Windows

matplotlib's `win_amd64` wheel is built with MSVC against the UCRT, and
pltrs on Windows computes as it does whichever toolchain builds pltrs:
an MSVC build calls the UCRT's maths library, and a GNU build is linked
against it too (`pltrs-core`'s build script writes the import library of
`api-ms-win-crt-math-l1-1-0.dll`, as MinGW's own `sin` or `pow` differ
from numpy's in the last bit). Where C's `long` reaches matplotlib's
results it has 32 bits there: FreeType's sizes (`Size::checked`) and
HarfBuzz's feature values (`Feature::from_str`). Its `_tri` was built
with MSVC 14.51, whose `std::shuffle` gives the trapezoid map's order
(`shuffle_msvc`, checked against the wheel itself). numpy's Windows
wheel has no `X86_V4` copy of its loops, so pltrs computes at `X86_V3`
at most there (`CpuLevel::detect`; pltrs (Python) asks numpy). Its
x86-simd-sort falls back to the C++ library's `std::sort` (arrays with
NaN, partitions that stall), MSVC's STL's there: `numeric::argsort`
sorts as that one does on Windows (`argsort/msvc.rs`), against numpy's
Windows wheel's digests (`digests-windows.txt`): pltrs (Rust) alone at
every length, and beside numpy for the arrays of up to 64 values it
replays (those with NaN reach `std::sort`; longer arrays are numpy's own
sort, see "Processor levels").

The spec's goldens of that platform are in
`spec/goldens/platforms/x86_64-pc-windows-msvc`, recorded on Windows as
the Mac's are, and read by both builds there: Ghostscript's console
program and its arguments are compared as `gs` like a host's path to it
(`pltrs-spec`'s `ghostscript`), and the reference run keeps the C
locale's dates (`%c`) where matplotlib's test setup finds en_US.UTF-8.
pltrs (Python) reads `SOURCE_DATE_EPOCH` as matplotlib's writers do,
with Python's `datetime`, whose range is the platform's (the UCRT's
`gmtime` refuses a time before 1970: `OSError`); pltrs (Rust), given
such a date itself, writes it.

## Adding a function: a worked example

`Text.get_rotation()` and `Text.set_rotation()`:

1. **Rust** (`crates/pltrs-python/src/modules/text.rs`): write the native
   functions and register them.

   ```rust
   /// The rotation in degrees, counter-clockwise.
   fn get_rotation(text: Text) -> PyResult<f64> {
       Ok(text.rotation())
   }

   /// Sets the rotation in degrees, counter-clockwise.
   fn set_rotation(text: Text, degrees: f64) -> PyResult<()> {
       text.set_rotation(degrees);
       Ok(())
   }

   // in register():
   module.add("get_rotation", get_rotation)?;
   module.add("set_rotation", set_rotation)?;
   ```

   Arguments convert through `FromPy` (here `Text` through
   `artist_handle!`, `f64` as `float()` would), results through `IntoPy`.
   Errors of pltrs (Rust) convert through the module's `From` impls with `?`. If
   pltrs (Rust) lacks what matplotlib does, add it there first (and run the
   output identity check if outputs could change).
2. **Python** (`python/pltrs/text.py`): the method with matplotlib's
   signature and docstring; the signature handling stays here.

   ```python
   def get_rotation(self):
       """Return the text angle in degrees between 0 and 360."""
       return _text.get_rotation(self._native)

   def set_rotation(self, s):
       """Set the rotation of the text: degrees, 'horizontal' or 'vertical'."""
       if isinstance(s, str):
           s = _api.getitem_checked({"horizontal": 0, "vertical": 90}, rotation=s)
       _text.set_rotation(self._native, float(s))
   ```

   Where plotting methods take the property as a keyword, add it to the
   module's keyword table too (`lines::LineKwargs` for lines), and an
   alias to `define_aliases` if matplotlib has one.
3. **Check**: `cargo xtask python test`, then the test files concerned:
   `cargo xtask mpl-suite test_text`. A new matplotlib *module* is a new
   `modules/<name>.rs` with its line in `modules/mod.rs`, and a new
   `python/pltrs/<name>.py`.

## Running the suite and reading its results

```sh
cargo xtask python setup                 # once: .venv-pltrs, and pytest-xdist in .venv
cargo xtask mpl-suite --reference        # real matplotlib (rarely needed)
cargo xtask mpl-suite                    # pltrs: builds the extension (release), runs, reports
cargo xtask mpl-suite test_colors        # only the files whose path contains a pattern
cargo xtask mpl-suite --failures         # also every failing test, with its reason
cargo xtask mpl-suite --imports          # every missing import of each test file
```

* **Where it runs.** `target/mpl-suite/<mode>/site/` lays out the package
  under test as matplotlib's source tree would be, with the sdist's test
  packages (`baseline_images`, data) copied inside: `site/matplotlib/`
  links the package's entries (installed matplotlib, or `python/pltrs`)
  plus a real `tests/`, and likewise each toolkit. Test ids read
  `matplotlib/tests/test_x.py::test` in both modes. The reference venv is
  not modified: matplotlib's wheel has no baseline images, so its
  `test_getattr.py` could not be collected there; the site gives the
  reference run the sdist's test data honestly. In pltrs mode
  `sitecustomize.py` (`pltrs_alias.py`) aliases `pltrs`, and a toolkit
  pltrs lacks gets a placeholder `__init__.py` so that test modules keep
  their package names.
* **Environment.** pytest-xdist with 8 workers (`PLTRS_SUITE_WORKERS`
  for another number), pytest-timeout at 600 s a
  test, `MPLCONFIGDIR` under the mode's directory (and no
  `PLTRSCONFIGDIR`, so that pltrs reads `MPLCONFIGDIR`), matplotlib's pytest
  options (`--import-mode=importlib`), and no font list cache from an
  earlier run (matplotlib would not notice new fonts). The committed
  outcomes were recorded with TinyTeX, Ghostscript and Inkscape on `PATH`:
  run both modes the same way. TinyTeX runs the usetex and PGF tests;
  `testing.compare` converts PDF and EPS files to PNG with Ghostscript
  (`~/.local/gs/bin`) and SVG files with Inkscape, as matplotlib's does,
  so those image comparisons run in both modes. Inkscape, the fonts the
  tests ask for by name (WenQuanYi Zen Hei, FreeMono, Noto Sans, Noto
  Sans CJK) and the TeX packages TinyTeX lacks (pgf, koma-script and those
  of single tests) come from `python/tools/suite-tools.sh install`, into
  `~/.local/mpl-suite` where nothing else looks. The runner hands both
  modes the variables of `~/.local/mpl-suite/environment`
  (`XDG_DATA_HOME` for the fonts, which matplotlib, fontconfig and pltrs
  read alike, and `TEXMFHOME`, `TEXMFVAR`, `TEXMFCONFIG` for the TeX
  tree), and says which tools a run lacks: without them those tests skip
  in both modes, with matplotlib's messages.
* **Outcomes.** `spec/mpl-suite/<mode>.json` holds the counts, the
  collection errors (file and reason) and every test id's outcome
  (`passed`, `failed`, `error`, `skipped`, `xfailed`, `xpassed`), one per
  line; commit `pltrs.json` with your work so that progress shows in git.
  A run with patterns replaces only its files' entries. Reasons of
  failures go to `target/mpl-suite/<mode>/details.json`, pytest's output to
  `pytest.log` beside it.
* **The report.** The counts of each run; per test file, the tests
  passing under matplotlib, those of them passing under pltrs, the share
  and the main blocker; the total share (the measure); how many tests the
  reference skips that pltrs does not (they must skip the same way); then
  the blockers: each reason with the reference-passing tests it holds
  back, the most first, and the files it holds back most.
* **GUI toolkits.** The GUI backends' tests (`test_backend_qt.py`,
  `test_backend_tk.py`, `test_backend_gtk3.py`,
  `test_backend_webagg.py`, `test_backends_interactive.py`, ...) need a
  display, the toolkits and their bindings, and the tests of dates,
  units and plotting need pandas. pandas, tornado and the Qt bindings
  (PyQt6, PySide6's essentials, PyQt5; PySide2 has no wheel for CPython
  3.14) are pinned in `python/requirements-suite.txt`, which `cargo
  xtask python setup` installs into both venvs. GTK 3 and 4 with
  PyGObject and pycairo, cairocffi, wxPython and the X libraries of Qt's
  xcb plugin come from `python/tools/toolkits.sh install`, which unpacks
  Debian's packages into `~/.local/gtk` and builds the bindings there
  for the venvs' Python, without root. On Linux the runner then gives
  both modes the same: the toolkits' variables (`toolkits.sh env`:
  library, typelib and data paths; their site goes on `PYTHONPATH`
  behind the runner's own entries) and a display, `$DISPLAY`'s or an
  Xvfb of its own on a free display number (`-noreset`, so that the
  workers' display checks are never refused while it resets), stopped by
  its process id when the run ends, with `$WAYLAND_DISPLAY` removed, and
  `LC_ALL=C.UTF-8` (the toolkits set the process's locale from the
  environment, which would otherwise make the outcome of
  `test_use_locale` depend on whether its worker ran a Qt or GTK test
  before, and on the host's locale). Qt then picks its xcb plugin itself
  (`QT_QPA_PLATFORM` stays unset); where the plugin lacks a library the
  runner runs Qt offscreen and says so. It prints what it has and what
  it lacks: the tests of what is missing skip, and the report counts
  them as not passing. The committed outcomes are recorded with all of
  them. A run of some files puts them in the whole suite's order
  whatever the patterns' (a Qt test leaves its application in its
  worker, where a later GTK test skips). `toolkits.sh run COMMAND` runs
  any other command with the toolkits, such as `cargo xtask python
  test`, whose smoke tests of the real toolkits
  (`python/tests/test_backend_gtk_wx.py`) need them and a display.
* **Collection errors hide the next gap.** A test file that fails to
  import shows only its first missing import; `--imports` lists all of
  them, per file and in total, weighted by the reference-passing tests of
  the files needing them.

## Choosing what to work on

Work by test file: a file is the unit the suite reports, and the file's
missing imports (`--imports`) are what its tests need first. Pick a file
(or a group sharing a missing module), make it import, then make its tests
pass, then run its pattern and commit the new `pltrs.json`. The blocker
list, sorted by tests held back, is the plan, the largest blockers first.
Now that every test passing under matplotlib passes under pltrs, the
suite guards what is done, and new work comes from the other measures
(the libraries' suites, the notebooks, the differential tester's
families and corpora, `--docs`, `--types` and `--private`), each of
which groups what differs by cause in the same way.

## Shared files

Most work touches only its own module's files: `modules/<name>.rs` and
`python/pltrs/<name>.py`. These are shared, so keep edits there small and
additive:

* `crates/pltrs-python/src/modules/mod.rs`: one `mod` line and one
  `register` line per module; merge conflicts here are resolved by keeping
  both lines.
* `python/pltrs/__init__.py`, `pyplot.py`, `axes/_axes.py`, `figure.py`,
  `artist.py`: add methods and functions, keep the existing ones in place.
* `binding/`: the core. Add a conversion or a helper where it belongs, with
  its tests; change existing behaviour only deliberately, as everything
  depends on it.
* `spec/mpl-suite/pltrs.json`: never resolve a conflict by hand; take
  either side and rerun `cargo xtask mpl-suite` after merging.

## Conventions for the Python code

* Idiomatic Python, matplotlib's names and signatures; comments only
  where the code cannot speak.
* Docstrings are matplotlib's, copied into the source with the decorators
  and snippets that interpolate them (`_docstring.interpd`, `Substitution`,
  `copy`, `kwarg_doc`, the `%(Class:kwdoc)s` tables), so that `help()`,
  `inspect` and Sphinx read what they read in matplotlib; where
  matplotlib's callable has none (``# docstring inherited``), ours has none
  either. What pltrs does differently underneath goes into comments, not
  into the docstring. `cargo xtask diff --docs` measures signatures,
  docstrings and names against matplotlib's; a module documenting an
  artist class's properties (`%(Line2D:kwdoc)s`) imports that class' module.
  What docstrings cost at import is kept small: the tables of the
  package's own classes are kept between runs in the cache directory
  (`_docstring._KwdocTables`, read while no file of the package or its
  extension has changed), and docstrings are cleaned with
  `_docstring.cleandoc`, which gives `inspect.cleandoc`'s result quicker.
* Class layout is matplotlib's: where matplotlib's class defines a method
  of its own, ours does too, if only to call the shared one (``def
  draw(self, renderer): super().draw(renderer)``), so that `__qualname__`,
  `__module__` and ``help`` read the class'. A mixin that several classes
  share, or that comes before the class' documented base in its MRO, has
  no docstring of its own (a comment says what it does), so that ``help``
  shows what matplotlib's shows; a constructor whose inherited docstring
  such a mixin hides gets it (``__init__.__doc__ = Axes.__init__.__doc__``).
* The names matplotlib's modules have from their imports (``np``, ``mpl``,
  the sibling modules, the classes and constants they re-export), which
  programs import from them, are imported at the end of the module, after
  the comment "What matplotlib's module also has from its imports";
  `cargo xtask diff --docs` lists the ones missing.
* No duplication: shared behaviour lives in a base class (`Artist`) or a
  helper module (`_handles`, `cbook`), as in matplotlib.
* Code taken from matplotlib (pure-Python modules such as `_api`,
  `_pylab_helpers`, the testing decorators) is kept close to the original
  and says so in its module docstring; it is under matplotlib's licence
  (`LICENSES/matplotlib.txt`, `THIRD_PARTY.md`).
* pltrs requires numpy, `cycler` (in `rcsetup`, whose validators return
  `Cycler` objects as matplotlib's do), Pillow, `python-dateutil` (`dates`
  parses date strings and resolves time zone names with it) and
  `packaging` (`parse_version` and `_get_executable_info` read versions
  with it, imported where first called), nothing else. matplotlib's other
  requirements, which pltrs (Rust) replaces (contourpy, kiwisolver,
  fontTools, pyparsing), change nothing pltrs does by being installed:
  a quad contour set refuses an `nchunk` contourpy refuses with
  contourpy's error itself (`_contour_chunks`, ported). The one object
  only they can give is a contour set's `_contour_generator`: where
  contourpy is importable it is the contourpy generator matplotlib would
  make, with the same arguments (`_contour_chunks.generator_arguments`),
  made when first asked for and shared with the contour sets made from
  it (`contour(cs, ...)`); pltrs (Rust) still computes and draws every
  contour. Without contourpy it is pltrs' own `contour._ContourGenerator`,
  which holds the field pltrs (Rust) contours again: a program reading
  contourpy's object depends on contourpy itself. A `TriContourSet`'s is
  the `_tri` module's generator (pltrs' `_tri`, matplotlib's C++
  extension over pltrs (Rust)'s triangulation). `python/tools/check_dist.py
  --install` installs a wheel alone and fails on an import of a package
  pltrs does not require, unless matplotlib imports it as optional too or
  its `OPTIONAL` says why it may be missing. The pltrs venv also holds
  what matplotlib's *test files* import at module level (`pyparsing`,
  `contourpy`), so that those files can be collected.
* Unimplemented keywords raise `NotImplementedError("pltrs: ...")`.
