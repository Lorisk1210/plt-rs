# pltrs: design

pltrs (Python) is a Python package over pltrs (Rust) with matplotlib's
syntax. Code written for matplotlib runs after one change, the import:

```python
import pltrs.pyplot as plt          # import matplotlib.pyplot as plt
from pltrs import colors            # from matplotlib import colors
from pltrs.mpl_toolkits.mplot3d import Axes3D
```

The measure is matplotlib 3.11's own test suite, run unchanged against
pltrs (`cargo xtask mpl-suite`). Every test that passes under real
matplotlib also passes under pltrs. This must stay so.

This document gives contributors the decisions that they must follow and
the place for new code.

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

The crate uses no PyO3. `crates/pltrs-python/src/ffi` declares by hand the part of CPython's limited API that the crate uses.
The extension (`_native.abi3.so`) loads in CPython 3.12 and later.

The minimum is 3.12 because its limited API adds `PyObject_Vectorcall`,
`PyErr_GetRaisedException`/`PyErr_SetRaisedException` and
`PyObject_GetTypeData` with a negative `basicsize`. With these, the
binding assumes no CPython struct size. It uses only the layouts that
the limited API exposes: the object header (`ob_type`), `Py_buffer`, and
the structs for types, methods and modules. Reference counts go through
`Py_IncRef`/`Py_DecRef`. To check that all imported symbols are in the
3.12 limited API, compile references to them against CPython's headers
with `Py_LIMITED_API=0x030c0000`.

The extension links no libpython, because the interpreter that imports
it supplies the symbols. The Rust unit tests embed an interpreter.
`build.rs` finds a shared libpython 3.14 or later through
`PLTRS_PYTHON`, `.venv-pltrs`, `.venv` or `python3`, in this order. It
links the library only into the test binary, which starts it with the
PEP 741 `PyInitConfig` API. Without a libpython, the test build is empty
and cargo warns, so `cargo test --workspace` works without Python.

On x86_64 Linux with glibc, the linker orders the functions as
`crates/pltrs-python/symbol-order.txt` lists them (see `build.rs`). The
code that every program runs comes first. So a process maps about 5 MiB
of the library's code instead of 11 MiB. Other targets keep the
compiler's order, because the file holds the names of this build.
`crates/pltrs-python/tools/symbol_order.py` writes the file from what the
benchmark scripts run. The linker puts a function that the file does not
name after all others, so the file becomes stale as code changes. Write
it again before a release and before an official benchmark
(`RELEASING.md`). `python/tests/test_startup_work.py` fails if the code
that a small program holds grows by a quarter.

### `unsafe`

Only this crate and `pltrs-platform` lift the workspace's ban on
`unsafe`. `ffi` holds the declarations. `binding` holds the wrappers,
with a `// SAFETY:` comment on every block
(`clippy::undocumented_unsafe_blocks` is on). `modules` and all code
above it are safe Rust and never see a raw pointer.

### The binding core (`binding`)

* `Python<'py>` is a zero-sized token that proves that the thread holds
  the GIL. It is not `Send`. Each entry from Python (`gil::entry`: native
  functions, slots, module init) opens a `GilScope`, which counts the
  thread as a GIL holder and gives the token. Other Rust code uses `Python::with_gil`, which calls
  `PyGILState_Ensure` only if the count is zero.
* `Bound<'py>` is a strong reference for use under the GIL. `Owned` is
  the `Send + Sync` strong reference that Rust state keeps, for example a
  callback in a pltrs (Rust) closure. Use it through `bind(py)`. If code
  drops an `Owned` where the thread may not hold the GIL, it goes to a
  pool. The next entry from Python releases the pool.
* `PyErr` / `PyResult<T>`: a native function returns `PyResult<T>`, and
  the binding raises an `Err` when the function returns. `ExceptionKind`
  names the built-in classes that Rust raises. A Rust panic never
  unwinds into C: `entry` raises
  `RuntimeError("pltrs internal error (Rust panic): ...")` instead.
* `FromPy` / `IntoPy` use Python's rules and messages. `bool` takes the
  truth value of any object. Integers take `__index__`, and floats take
  what `float()` takes without parsing. `String` takes only `str`.
  `Vec<T>` takes any iterable except a string, and returns a `list`. A
  tuple takes an iterable of its length. `BTreeMap<String, V>` takes any
  object with `items()`. `PathBuf` takes `os.PathLike`, and `Bytes` takes
  `bytes`.
* `NdArray<T>` (`T` is `f64`, `i64`, `u8` or `bool`) moves numbers in
  bulk. It reads nested sequences and any buffer exporter: numpy arrays
  of any numeric dtype, byte order and strides, `array.array` and
  `memoryview`. It returns a numpy array that owns a copy. `FloatArray` reads an
  argument as `numpy.asarray(arg, float)` does. The binding uses no
  numpy C API.
* Deferred work (`binding::install_deferred`) is a dict of work that
  Python leaves to the next native call. Before any native call runs,
  the binding empties the dict into the function installed with it.
  `Line2D.set_data` converts its data at once and leaves the numbers
  there, as matplotlib leaves them to the next draw.
* The native object (`_native.Native`) holds any Rust value for Python.
  Python cannot make or subclass it. See "The native object".
* Native functions are plain Rust functions or closures. Register them
  with `Module::add(name, function)`. See "Adding a function".
* `Callback` is a Python callable that pltrs (Rust) calls. See
  "Callbacks".
* The registry (`_native.register(name, object)`,
  `binding::registered_error`) holds the package objects that native
  code needs, for example the exception `ImageComparisonFailure`. The
  native module never imports the package by name, so it works under any
  name (see the suite's aliasing).

### The native object

A native object holds one `'static` Rust value in a `RefCell`. For an
artist, the value is the artist's `RawHandle`. A value that is tied to
its thread also records that thread (see "Threads").

* `artist_handle!(Line2D)`, in the module file that owns the type, lets
  native functions take and return `Line2D`. The same native object
  works with the functions for all artists, which take a `RawHandle`.
  These are in `_native.artist`: visibility, z-order, alpha, label,
  `remove` and `id`, the artist's id in its figure. A function that
  takes a `Line2D` accepts the native object of any handle of a line,
  also an untyped `RawHandle`. For another kind of artist, it raises
  `TypeError`. Every type given to `artist_handle!` implements the
  `TypedHandle` trait of pltrs (Rust), which recovers the type from an
  untyped handle.
* `native_value!(RcParams)` does the same for values that are not
  artists. The function gets a clone. To change the value in place, a
  function takes `Native<T>` and uses `borrow_mut()`.

### Threads

As in matplotlib, any Python thread that holds the GIL can use any pltrs
object. Neither library is made for concurrent use, but no object is
tied to the thread that made it. `binding/threads.rs` describes the
model:

* pltrs (Rust) is single-threaded code. Handles share their figure
  through `Rc` and `RefCell`, so they are not `Send`. The GIL makes
  sharing sound. Only native calls and the native type's slots touch
  native values, and they hold the GIL. The binding releases the GIL
  only in two waits that touch nothing: `home`'s wait for the main
  thread and the window event loop's wait (`binding::blocking`). A
  callback that releases the GIL runs no Rust code until it has it again. abi3 builds exist only for CPython with a GIL.
* pltrs (Rust) keeps per thread the state that matplotlib keeps in
  process-wide variables, such as rcParams, the registries and the style
  library. Each such variable is a `pltrs_core::thread_state!`. A thread
  that works with a `pltrs_core::state::State` reaches the variables in
  it.
* Every Python thread works with the interpreter's `State` while it
  holds a `GilScope` (a native call, a slot, `Python::with_gil`). So
  rcParams and `rc_context` are global, as in matplotlib. Only the state
  of one call and its callbacks stays per thread. An example is the
  renderer stacks of `_text_metrics`, `_python_drawing` and
  `_transform_trees` (`threading.local`).
* Thread-local storage must not hold a value that another thread can
  reach, because it is dropped without the GIL. The metrics cache of
  `text._measuring_with` lives across native calls. When its thread
  leaves its outermost scope, the binding parks the cache in the shared
  state.
* Never borrow shared state across a call into Python, because Python
  can switch threads there. Drop what a change replaces after the borrow
  ends. Hold no lock while Python runs.
* A native call that calls into Python keeps its figure borrowed. If
  another thread's outermost call needs that figure, it waits without
  the GIL, up to five seconds.
* The windows of pltrs (Rust) (`_native.gui`'s host) are tied to their
  thread, because macOS and Windows require it. `wrap_thread_bound`
  makes such objects. They refuse other threads with
  `RuntimeError: this pltrs WindowHost belongs to another thread`. If another thread
  deallocates one, its value leaks.
* While the main thread runs the window loop, it runs the calls that
  other threads make on windows (`binding/home.rs`). The caller waits
  without the GIL. `backends/_pltrs_loop.py` puts
  `_native.serve_threads(True)` and `(False)` around its loop and calls
  `_native.run_queued_calls()` at each step. If the main thread does not
  serve, a call waits up to one second, as tkinter does. Then the call
  is refused. So a main thread that waits for the caller gets an error,
  not a deadlock.

The tests are `python/tests/test_threads.py`,
`test_backend_pltrs.py::test_interactive_thread_safety` and matplotlib's
`test_interactive_thread_safety`. The last one needs the toolkits and a
display (see "Running the suite").

### The stack guard

A program can recurse through drawing, for example with a formatter
that saves its own figure. Native frames run between CPython's checks,
so the native stack can end before CPython's limit. The process then
dies.

`binding/stack.rs` refuses a nested native call with `RecursionError`
if less than 64 KiB, the reserve, of the thread's stack is left. Every
nested call passes through `function::call_nested`, which makes the
check. The binding reads the stack bounds of each thread once:
`pthread_getattr_np` and `/proc` on Linux, `pthread_get_stackaddr_np` on
macOS and `GetCurrentThreadStackLimits` on Windows. Then the check is
one comparison.

The reserve comes from measurement. One recursion goes at most 44 KiB
deeper between two nested calls. A draw goes at most 47 KiB below its
last nested call in an optimized glibc build. It goes 48 KiB in the musl
wheel and 56 KiB in the development build. Do not hold large values by
value in a frame, because they cost stack in every thread. For this
reason, `Figure::to_format` does not inline its save paths into one
frame. The DEFLATE trees, the renderers and the figure state of a layout
are on the heap.

So a musl thread with the default 128 KiB stack draws with the program's
callbacks. CPython 3.9 to 3.13 in the `musllinux` images and uv's musl
builds have such threads. A draw inside a draw needs about 90 KiB and
does not fit there. Its error starts with Python's "maximum recursion
depth exceeded", because code matches on this text. Then it gives the
thread's stack size, the stack that pltrs needs and
`threading.stack_size`. pltrs does not set `threading.stack_size`,
because a library must not set it for the whole process.

The reserve must hold all that a call does below its check, also after
one of its calls is refused. Two kinds of work go deeper than the
measured draws:

* Shaping text for the first time needs 12 KiB in the development
  build.
* Typesetting a formula needs about 7 KiB for each level of fractions.
  The pltrs parser accepts 100 levels. matplotlib's parser accepts 20
  levels but not 30.

Both are pure Rust. If the stack is too short for them,
`pltrs_text::deep` runs them on a separate thread with enough stack
(`pltrs_core::stack::with_room`). The binding tells pltrs (Rust) where
each thread's stack ends. A formula level opens at `{`, `\`, `^` or `_`
and needs at most 8 KiB. The separate thread hands its font lookups and
mathtext messages back to the caller's thread. It does not write to the
font database and takes no lock that its callers hold.

Without this work, a nested draw goes at most 47 KiB below its check in
the development build and 46 KiB in an optimized build. `test_stack1.py`
measures this on saves in four formats with `PLTRS_DEEP_WORK_APART`,
which always uses the separate thread. It fails if less than 12 KiB of
the reserve is left. `_native._stack_room()` returns the stack position
and the reserve.

The binding also checks the calls of native code into Python, against
48 KiB. CPython 3.14 refuses a Python call if less than 32 KiB is left
(twice `_PyOS_STACK_MARGIN_BYTES`) and raises
`RecursionError: Stack overflow (used 478 kB)`. With the 48 KiB check, matplotlib's message
always comes first, on every build. CPython 3.12 and 3.13 do not check
the stack, and this case was a segfault there. `python/tests/test_stack1.py`
also tests every recursion, the deepest formulas and small threads.

## The Python package

### Where logic lives

pltrs (Rust) does the work: validation, layout, drawing, saving, color
and tick computation. Python holds only what must be Python:

* matplotlib's signatures (positional forms, keywords, defaults, aliases)
* `set(**kw)` dispatch
* pyplot's registry of Python figure objects
* the `_api` decorators
* the `cbook` helpers that work on Python objects
* the pytest code of `testing`

If a Python method does more than change its signature into native
arguments, move that logic into Rust.

### The look

If a program imports `pltrs` first, pltrs (Python) uses the modern look:
the `pltrs-modern` style sheet, Inter and constrained layout (see
`pltrs::look`). If a program imports `matplotlib` first, pltrs uses the
matplotlib look, and the output is matplotlib's. The alias and
matplotlib's suite import it as `matplotlib`.

The extension loads once per process, as `pltrs._native` or as
`matplotlib._native`. `modules::register` reads this name. For
`matplotlib._native`, it calls `pltrs::look::set_default`. Then it starts
the parameters from the look in force. The toolkits and `pylab` import
`pltrs` by name, but the alias serves them the loaded package. So they
cannot decide the look.

`PLTRS_LOOK`, `PLTRS_FIGURE_LOOK` and `PLTRS_CHROME_LOOK` override the
import name. Code that needs the look must read the look in force, never
the variables. `rcParams` and `rcParamsOrig` hold the parameters of the
look. `rcParamsDefault`, `rcdefaults()` and `style.use("default")` give
matplotlib's defaults, as `pltrs::rc::reset` does. Comparisons with
matplotlib use the matplotlib look: `cargo xtask mpl-suite` and the
benchmarks (`xtask/src/bench`) set `PLTRS_LOOK=matplotlib`.

### How a class wraps a native handle

An artist object holds `self._native`, the native object of its pltrs
(Rust) artist, and `self._figure`, its root `Figure`. Methods call the
native functions of their module:

```python
class Line2D(Artist):
    def get_linewidth(self):
        return _lines.get_linewidth(self._native)
```

A native function returns a new native object each time. But matplotlib
code expects the same Python object back (`ax.lines[0] is line`). So
each root `Figure` keeps the registry `self._artists`, which maps artist
ids to objects. `_handles.wrap(native, cls, figure)` changes a handle
into its object:

```python
lines = _handles.wrap_all(_axes.get_lines(self._native), Line2D, self.figure)
```

`wrap` returns the existing object, or a new `cls` around the handle
(`cls._from_native`). The caller always knows the figure and the class.
An artist object and its figure refer to each other, so the object lives
as long as its figure. All references are Python references, so the
garbage collector sees the cycle.

### Artists made before they are added

matplotlib code can make an artist first and add it later:
`line = Line2D(x, y)`, then `ax.add_line(line)`. The same holds for
`add_patch`, `add_collection`, `add_image`, `add_artist` and
`fig.add_artist`.

pltrs (Rust) supports this for every artist that owns no other artist.
Such an artist starts *loose* (`Line2D::loose`, `Patch::loose`,
`Collection::loose`, `Text::loose`, `Table::loose`, `Custom::loose`). It
lives in a holder that is a figure state but not a figure, so its handle
works before it is added. `Axes::adopt_artist(artist, AddAs::Line)` and
`Figure::adopt_artist(artist, clip)` move it into the figure under a new
id and return the new handle. `AddAs` names the `add_*` method, which
decides the data limits and the `_child<n>` label. An artist without a
transform gets the default one (`transData`, `transSubfigure`). An
artist removed from the same figure comes back under its old id.

In Python, a constructor makes the loose artist with a `new` function of
its module, for example `_lines.new(x, y, kwargs)`,
`_text.new(x, y, text)`, `_annotation.new(text, ...)` or `_table.new(ax, loc, bbox)` for
`Table(ax)`.
Patches have `_patches.new_rectangle(...)` and a `new_<shape>` for each
class. Collections have `_collections.new_line_collection(...)` and a
`new_<kind>` for each class. `Artist()` makes a plain artist that draws
nothing (`_artist.new()`). Then `Artist.__init__` sets `_figure` to
None.

The `add_*` methods (`axes/_base.py`, `Figure.add_artist`) call
`Artist._add_to_axes` or `_add_to_figure`, which a class can override.
These call `_native.artist.adopt_into_axes` or
`adopt_into_figure`. Then `_handles.adopt` gives the object the new
handle and registers it in the figure. The new handle is an untyped
`RawHandle`, which the `artist_handle!` conversions accept. The loose
holder is then empty. An artist's `axes` is the Axes that pltrs (Rust)
has it in.

`artist.set_figure(fig)` alone moves a loose artist into the figure as a
removed artist (`Figure::take_artist`), as in matplotlib. The figure does
not draw it, but measures it (`RawHandle::measured_extent`). It draws it
where the program asks (`RawHandle::draw_onto`, a `RendererAgg` of no
figure that draws onto its own pixels).

An artist's `figure` is the figure or subfigure that it is on. A
subfigure's `figure` is its root. `_figure` is always the root, which
keeps the registry. `_handles` reaches it through `figure._figure`.

matplotlib's constructors of artists that own other artists take the
parent: `Axes(fig, rect)`, `Legend(ax, ...)`, `SubFigure(fig, spec)`,
`Quiver(ax, ...)`, `Barbs(ax, ...)` and `QuiverKey(Q, ...)`. So pltrs
makes these artists in their figure, as removed artists. pltrs (Rust)
lays out the last three for their Axes (`Axes::removed_quiver`).
`fig.add_axes(ax)` and `ax.add_collection(q)` add them back. Until then,
a quiver, barbs or a key is decoupled (`Artist._decouple`, see "Artists
removed, cleared and added though they are no artists").

An `InsetIndicator` is a plain loose artist with a loose `Rectangle`
until an Axes adds it. Then `Axes::adopt_indicator` makes the rectangle
its frame and gives it four `ConnectionPatch` artists (`_native.insets`).

Offset box constructors take no parent. So until it is added, every
`OffsetBox`, such as an `AnchoredOffsetbox` or an `AnnotationBbox`, is a
loose `pltrs::offsetbox::ProgramBoxArtist` (`_native.offsetbox.new_box`).
The artists of its tree are loose too.

When a program adds the box, the box describes its tree to pltrs (Rust)
(`OffsetBox._native_tree`, `_native.offsetbox`). `BoxItem::from_loose`
moves each artist into the box. The box that added the tree is its owner
(`_offsetbox_trees.TreeOwner`). It gives every Python artist of the tree
its handle in the figure. Each box knows its place (`_node`): the
owner's handle and its index in tree order, parents first. Its figure,
Axes and staleness are the owner's (`_tree_owner`). An `OffsetBox` gives
its children its Axes, and an `AnnotationBbox` gives them none.

Through its place, every package box has matplotlib's protocol, for
matplotlib's algorithms in Python:

* `get_bbox` returns the extent from the layout
  (`pltrs::offsetbox::node_extent`).
* `get_offset` returns where the parent placed the box at the last draw
  (`node_origin`). A hidden box keeps this value.
* `get_window_extent` combines the two.
* `draw` draws the artists where pltrs (Rust) placed them. A container
  draws through matplotlib's `OffsetBox.draw`.

A text area places its text through
`offset_transform + _baseline_transform`, which `Placement::Text`
updates at each layout. The transforms that matplotlib's draw sets on
the leaves read the last layout (`offsetbox._FollowsLayout`). These are
the `offset_transform` of text areas, drawing areas and aux-transform
boxes, and the `dpi_transform` of a drawing area. An aux-transform box outside any
tree gives its children its transform and measures them as matplotlib's
`get_bbox` does. After the layout, the children read their transform
from pltrs (Rust).

matplotlib reads the settings and children of a box at each draw. So the
owner gives its tree to pltrs (Rust) again after a change
(`TreeOwner._sync_tree`). `pltrs::offsetbox::rebuild_tree` keeps the
artists that the tree held, takes the loose ones and releases the
others. The owner does this at three times:

* when a box setting such as `pad` changes
  (`_offsetbox_trees.TreeSetting`, `OffsetBox._changed`)
* before Python measures a box of the tree
* before each layout of a draw, through the layout hook of the figure
  (`axes._program_locators`)

The last one finds changes that Python cannot see, such as a list of
children changed in place. An anchored box resends its anchoring
settings in the same way.

A box of a program class can size and draw itself
(`OffsetBox._sizes_itself`). Such a class is an `OffsetBox` subclass
that describes no tree, or a package subclass that overrides `draw`,
`get_bbox`, `get_offset` or `_get_bbox_and_child_offsets`. The box is a
leaf of the tree, `("program", visible, box)`, and its own painter (see
"Drawing defined in Python"):

* After the layout, pltrs (Rust) asks the box for its extent
  (`ArtistPainter::box_extent`, through `_python_drawing.measure_box`).
  Then it lays out the boxes above it again.
* Before its `draw` runs, `set_offset` gets the place from its parent
  (`OffsetBox._place_as_laid_out`).
* Added alone, the box is a painted artist.
* It draws its children itself. A package box among them becomes a box
  that the program box owns (`ProgramOffsetBox::add_box`,
  `OffsetBox._join_children`). pltrs lays it out with the figure, but
  draws it only when the program's `draw` does.
* A program box among the children passes its children on.
* A subclass of a package leaf, such as a `DrawingArea` that overrides
  `draw`, gets the box that its class describes. The package methods
  that it calls (`super().draw(renderer)`) measure, place and draw it.
* pltrs measures such boxes once per layout. matplotlib's parents
  measure a child again in each `get_bbox`, `get_offset` and `draw`.

The cells of a table are part of its native state, not artists. A
Python `Cell` is the cell at its `(row, column)` key. A table places its cells when it is drawn or
measured, as matplotlib's `_update_positions` does. Until then, a cell
stays at the origin with its own size (`TableArtist::place_as_is`,
`Table::place_cells`). A cell made with `Cell(xy, width, height)` lives
in a loose table without an Axes. `table[row, col] = cell` copies it
into a table and makes the object that table's cell.

### Artists removed, cleared and added though they are no artists

In matplotlib, `remove` and `clear` decouple an artist: its `figure` and
`axes` read None. It keeps its transform and clip box, so it measures
where it was.

pltrs (Rust) keeps a removed artist apart in its old figure
(`DetachedArtists`). The Python object keeps its root figure
(`_figure`), with its registry and trees. `Artist._decouple` marks it
decoupled: `get_figure()` and `axes` read None, and `set_figure` and the
`add_*` methods accept it again. It also computes what matplotlib's last
draw left on the artist, such as an annotation's transform
(`_keep_drawn_state`). A child Axes keeps the place from its host
(`_leave_host`).

Removal fails as in matplotlib:

* A second removal raises the `ValueError` of the list
  (`_remove_again`). The removal of an Axes' legend does not fail.
* The removal of an Axes is its figure's `delaxes`, which raises
  `KeyError` once the Axes is out.
* A child Axes leaves its host's list of child Axes
  (`_check_removable`).
* The parts that an artist made of itself, such as spines and titles,
  raise `NotImplementedError` (`RawHandle::is_part`).

If `Axes.add_artist` gets an object that is not an artist, such as a
colorbar, its `set_figure` fails. The Axes keeps it anyway
(`axes/_foreign_children`). Every later draw of the figure fails on it
in the draw hook, because matplotlib's `Axes.draw` asks each child
whether it is animated. If `add_collection` gets a collection of another
figure, matplotlib lists and draws it before the `RuntimeError`. In
pltrs, a stand-in artist of the Axes, painted through Python, draws it
with the collection's `draw`.

pltrs (Rust) holds each artist in one place of one figure. So the
stand-ins also list artists that matplotlib lists in a second place
(`_foreign_children.held_elsewhere`, `relist`):

* An artist of another figure, given to `add_artist`, is listed before
  the `RuntimeError`.
* An artist removed from another figure moves into the figure of the
  Axes and reads it as its own (`_listed_in`).
* An Axes part, a figure artist, a box of an offset box tree or an Axes
  is listed again and drawn twice.
* `Figure.add_artist` with a child of one of its Axes lists it among the
  figure's artists too (`relist_in_figure`).

The removal method of the stand-in takes such an artist out again
(`_listing`). `remove` checks it before it refuses a part. A part taken
out in this way is decoupled while its owner still draws it. In
matplotlib, the draw of the owner then fails. pltrs keeps this failure as
a problem of the owner (`_unusable`). A hidden part does not fail.

The other add methods give the artist the figure first. They fail first
on an artist of another figure (`add_held`). After `artist.axes = None`,
`axes` reads None (`_axes_unset`). When an add method takes the artist
back, `_recouple` undoes all of these states.

A stand-in draws through the artist's `draw`. For a *mirror* (a tick
label or line, an axis label, an offset text), this draws nothing,
because pltrs (Rust) draws mirrors with their axis. But matplotlib's
artist draws what it last got wherever it is drawn. So the stand-in asks
the part for a separate artist made like it (`Mirroring.alone`). This
artist has the text or data, properties, axis transform and figure of
the mirror.

### Removed and cleared parts measured where they were

In matplotlib, `remove`, `delaxes` and `clear` only take an artist out
of lists. A removed Axes keeps its position, limits, tick helpers and
children. A title or a tick that `clear` replaced keeps its text and
place. All of them still answer `get_window_extent` and
`get_tightbbox`. pltrs (Rust) keeps removed artists outside the store of
the figure, where layout and measuring code does not look. So the design
is:

* A removed tree stands in while it is measured
  (`figure::standing_in`, `RawHandle::standing_in`). During a measure,
  the removed artist and all that it owns move into the store. No artist
  lists them, so the figure does not draw them or give them to a layout
  engine. But `FigureState::prepare` autoscales, places and ticks a
  removed Axes as any other, and lookups by id find its parts. These measuring entry points
  do this: `measured_extent`, `extent_as_is`, `Axes::tight_bbox`,
  `Axis::tight_bbox`, `Axes::ticks` and `Figure::tight_extent`. A new
  measuring entry point must do it too. No other code needs a change for
  each kind of artist.
* A removed artist remembers where it was. An Axes or a subfigure
  removed from a figure frame keeps that frame as its `home` and stays
  placed in fractions of it. `FigureState::frame_bbox` and the transform
  trees follow the `home`. The artist still belongs to that figure
  (`Figure::parent_of`). A removed subfigure stays the figure of the
  artists removed with it.
* `clear` makes new titles (`FigureState::renew_titles`). The old titles
  stay parts of the Axes in `AxesState::former_titles`, with their text
  and last place, but the Axes does not draw them. The title object
  that a program holds still reads its own text.
* A program's artist keeps what it was made with until it is placed. A
  spine made with `Spine(...)` or `linear_spine` has `transData` and no
  axis until something asks for its position (`Spine::is_unplaced`). An
  Axes that was never drawn places the spine that takes a side's place
  when it makes its ticks. The children of a clipped `DrawingArea` keep
  the outline of the area from their first draw
  (`offsetbox::apply_placements`).
* Ticks that `clear` and `reset_ticks` drop retire (`Tick._retire`). Ticks
  mirror what pltrs (Rust) draws, so a dropped tick stops mirroring. Its parts become separate artists with
  the text, location and transforms of the last draw. Code reads these
  transforms inside `spines._transforms_read()`. If a new tick asks for
  them, the Axes sets the position of an unset spine and makes the
  ticks again (`Spine._ensure_position_is_set`). A retiring tick asks
  for nothing.

### Axes sharing an axis across figures

matplotlib has one `Grouper` per axis name for the Axes of all figures.
So `ax2.sharex(ax1)` and `plt.subplot(..., sharex=ax_of_another_figure)`
can join Axes of two figures. pltrs (Rust) keeps one state per figure,
with the groups of its Axes. Links join Axes across figures
(`axes::share_across`). Each of the two Axes holds the other weakly. The
group of an Axes is its figure's group plus the groups of its peers
(`FigureState::peers`).

The code that keeps a group in step reaches the peers after the Axes in
the same figure. It borrows each other figure for the change and skips a
figure that code further up the stack uses:

* Limits (`set_axis_view`): the peers take them without emitting. Their
  limit callbacks run after those of the changed figure, and their
  figures become stale (matplotlib's `draw_idle`).
* Autoscaling: an axis is stale if a peer's axis is stale. It autoscales
  over the data and sticky edges of all peers (`shared_data_across`).
* Scales, tick helpers and all else that `update_shared_axis` shares.
* Removal and `fig.clear()`: the Axes leaves, and the others stay
  together (`leave_peers`). A cleared Axes takes its peer's axis again.

pltrs lays out and draws each figure alone. A figure's archive holds no
links. An Axes pickles the Axes of other figures that it shares with. This brings their figures into the pickle, as in
matplotlib, and joins them again on load (`axes/_sharing.py`,
`Axes::join_across`). The `locator` and `formatter` objects of a shared
axis are one object for all figures (`_tick_helpers._held_by_peers`). A
shared locator counts the ticks that fit along the axis in its own
figure.

### Picking

`Figure.pick` runs on button presses and scrolls, as in matplotlib
(`backend_bases._pick_handler`). pltrs (Rust) keeps the pickers `True`
and a tolerance (`Picker`). It orders the pickable artists as
matplotlib's recursion over the children does (`Figure::pick_order`). It
hit-tests them with `contains` and the tolerance of the class
(`_pick_radius`). A callable picker stays with the Python artist, which
adds itself to the artists to order. The pick then calls the picker.

### Transforms

`transforms` is matplotlib's Python transform tree. It computes with
`_native.transforms`. The trees of figures and Axes are matplotlib's too
(`_transform_trees`):

* `fig.transFigure` is a `BboxTransformTo` of `fig.bbox`, a
  `TransformedBbox` of `fig.bbox_inches` through `fig.dpi_scale_trans`.
* The `transSubfigure` of a subfigure maps its `bbox_relative`.
* The `transData` of an Axes is `transScale + (transLimits + transAxes)`
  over its `_position`, `_viewLim` and scales. Its `bbox` is the
  `TransformedBbox` of `_position` through the `transSubfigure` of the
  figure.
* `_set_lim_and_transforms` of the class builds them. Polar Axes use
  `projections/_polar_trees.py`, and geographic Axes use `GeoAxes`.

pltrs makes each object once, at the first read of any of them
(`TreeAttribute`), and keeps it. Thus `ax.transData is ax.transData` and
`line.get_transform() is ax.transData`. The trees compose, invert,
freeze, invalidate and pickle as matplotlib's do. pltrs makes them again
when it reads a pickle.

If the scale maps the view onto one point, the trees are not finite.
Examples are a logit axis with limits beyond 1, or a function scale
whose inverse is NaN. `transLimits` then gets an infinite scale, and no
entry of `transData` is finite. pltrs (Rust) builds the same matrices
(`ProjectionView::rectilinear`, `bbox_transform_from`) and draws what
they give:

* Nothing in data coordinates is drawn, and no axis shows ticks. The
  patch, the title and what is placed in axes fractions stay.
* The tick labels, the axis label and the offset text are at NaN
  (`AxisTexts::unplaced_labels`, `measured_at_nan`).
* The box of an image is at NaN, so it intersects no clip box and pltrs
  makes no image.
* Contour labels on a line at NaN fail as in matplotlib, at `math.floor`
  of the NaN that numpy's `interp` gives. `numeric::interp` searches as
  numpy does (`ContourError::LabelCutAtNan`).

The leaves of the trees are values of pltrs (Rust)
(`Figure::tree_leaves`). They are the positions, limits and scales of an
Axes, the box of a subfigure, and the size and resolution of the figure.
Polar and geographic Axes add the values that their trees use. pltrs
reads the leaves when it builds the trees, and keeps them current:

1. A figure with built trees is watched (`Figure::watch_changes`). It
   records itself as changed when a handle borrows its state for a
   change.
2. At the end of every native call, an after-call hook reads the
   recorded figures again (`sync_watched` in
   `modules/transforms/leaves.rs`, `take_changed_figures`).
3. `_transform_trees._sync` sets the changed leaves as matplotlib sets
   its own. This invalidates what is built on them.

The hook costs almost nothing after a call that changed no watched
figure. The Python code of a draw reads the leaves of the laid-out figure
(`_transform_trees.sync`). If the program sets a leaf
(`ax.viewLim.set(...)`, `ax.dataLim`, `fig.bbox_inches`), `_LeafWatcher`
gives it to pltrs (Rust).

The trees show the current state, as in matplotlib. Before a draw,
`ax.transData` maps the limits before autoscaling (`(0, 1)` for a new
Axes). It also maps the position before the aspect and the layout engine
change it. pltrs (Rust) updates its state
at the same moments as matplotlib:

* A read of the limits (`viewLim`) autoscales.
* `get_position()` applies the aspect (`Axes::apply_aspect`), but not the
  layout engine.
* Contour sets and stream plots autoscale when they are made.
  `Axes::contour` asks for the view again, as `Axes.contour` does. The
  labels go into that view, and nothing pending is applied.
* A draw lays out the whole figure.

`canvas.inaxes` and `Axes.contains_point` test the Axes at its current
place. A removed Axes keeps its transforms at its last position.

pltrs computes window extents from the current trees, as matplotlib
does. A line uses its vertices padded by its markers, and a patch its
path. A collection uses `get_datalim` with the identity, which is null
for scatter offsets in data coordinates. A text uses
`RawHandle::extent_as_is`, and an `Artist` gives a zero box. matplotlib lays out legends, tables and
annotation boxes when it measures them. pltrs gives their place in the
current figure (`_extent_as_is`). For anchored offset boxes and the parts
of axis artists, pltrs gives their place in its layout
(`_laid_out_extent`).

matplotlib updates some state only when it draws the artist, such as the
transform of an annotation or the path of a spine. For this state, pltrs
uses the draw count (`RawHandle::draw_count`,
`_native.artist.draw_count`). The Python side keeps each computed value
with its count. It computes the value again when the count changes.

`_live_transforms` is the only bridge between the two trees:

* **To pltrs (Rust).** A transform crosses as nested tuples
  (`_native.transforms.native`, `to_native`) of composites, blends,
  `ScaledTranslation`s and affine maps. Some tree nodes stand for a
  native transform: `ax.transData`, `ax.transAxes`,
  `ax.get_xaxis_transform()`, `fig.transFigure` and `fig.dpi_scale_trans`.
  Such a node carries it as `_live` and crosses as it. A box of an Axes
  or figure crosses as the transform that spans it (`_spanning`). An
  affine transform of the program crosses as its matrix. Any other one
  crosses as `("python", t)`, a native map that calls back its
  `transform` and the `transform` of its inverse.
* **Who uses it.** Everything that gives pltrs (Rust) a transform uses
  `to_native`. This includes `Artist.set_transform`, the `transform`
  line property and `Collection.set_offset_transform`. The `transform=`
  keywords of `scatter`, `quiver`, `barbs` and `streamplot` pass
  `given_to_native(t)`. It is None when absent,
  and the native function uses its default.
* **Back to Python.** `describe` gives a native transform back as the
  tree nodes that it names (`from_native`). A native transform without a
  node, such as the map of a projection, comes back as a
  `LiveTransform`. `of_artist(artist, native)` gives the transform or
  offset transform of an artist (`IdentityTransform` for none). It gives
  the same object while the native transform stays the same (`kept`).
  Thus `Artist.get_transform` and `Collection.get_offset_transform`
  return the object they got.
* **Clip boxes.** A native clip box is the unit box through the patch of
  its Axes or figure, as with `set_clip_path(self.patch)`. If the patch
  is not a `Rectangle`, its `TransformedPatchPath` is the clip path
  instead. A clip path that the program sets replaces it. A clip box
  that the program sets keeps it. The axis and the gridlines of its ticks
  do the same (`_artist_clipping.patch_clip_path`).
* **Adopted artists.** `add_artist` gives offset boxes, inset
  indicators, quiver keys and tables matplotlib's transform and clip box
  (`take_parent_defaults`). Axis mirrors answer with matplotlib's
  transforms (`get_xaxis_text1_transform` and similar).

A non-affine transform of the program crosses wrapped in
`_live_transforms._ProgramMap`. pltrs (Rust) maps whole paths through a
matplotlib `TransformedPath`, which keeps the last path until the
transform is invalidated. Thus an affine change does not call the
transform again. pltrs asks for the inverse only when a point first goes
back through it (`_InverseMap`). matplotlib inverts nothing when an
artist gets a transform, and programs count calls of `inverted`.

pltrs calls Python callables that map values with arrays, as matplotlib
does. This applies to program transforms and scales, function scales and
secondary axes (`FuncTransform`), and the `(trans, inv_trans)` functions
of curvilinear grids. pltrs (Rust) collects the values
that a path, a set of ticks or a limit needs, and calls once
(`modules/transforms/arrays.rs`: `AxisMap::forward_all`,
`PointMap::forward_all`, `AuxTransform::transform_points`). A masked
answer reads as NaN. Where matplotlib calls `Transform.transform`, a
one-dimensional transform gets a column of shape `(N, 1)`
(`AxisMap::forward_column`, `scale::ArrayLayout`).

A secondary axis gives its forward function the host limits as one array
wherever matplotlib's `_set_lims` runs (`tick_sites`,
`axes_tight_bbox_at`). An axis keeps its view limits mapped through its
scale while both stay the same (`AxisState::scaled_ends`). Formatters
keep matplotlib's `__call__` per tick, through `format_ticks`.

An `Axes` subclass that the program registers with `register_projection`
is a projection unknown to pltrs (Rust) (`projections/_program.py`). An
example is `SkewXAxes` of the SkewT-logP example:

1. `set_up` builds a rectilinear native Axes as matplotlib's `__init__`
   does. It builds the trees with `_set_lim_and_transforms` of the class,
   from a new view, no data, and the unit box for an Axes of a grid. The
   grid cell replaces the unit box later.
2. It calls `_gen_axes_spines` and `_init_axis`, from the mixin
   `axes/_projection.py`.
3. It gives the Axes to a native `Projection`, which asks the Axes
   through an adapter (`_native.projections`, `Axes::install_projection`).

The adapter gives the data transform, which is `transData` without its
final `transAxes`. If the class overrides one of these methods, the
adapter also gives its answer: `get_xaxis_transform`,
`Axis.get_view_interval`, `Tick.draw`, `Spine._adjust_location` and
`get_data_ratio`. For `Tick.draw`, the answer is the sides that it
leaves visible on a tick made alone. pltrs asks for the data ratio
wherever matplotlib applies the aspect (`Projection::data_ratio`). That
is in the draw of the figure (`_get_draw_artists`) and where the Axes is
drawn.

Each clear, also the clear by `add_subplot`, gives the Axes the patch of
`_gen_axes_patch` (`_program.cleared`). If the patch is not the unit
square, its outline clips what the Axes clips
(`Axes::set_patch_outline`). A `Rectangle` clips as its box. Images clip
within the Axes box, as with `get_clip_box() or self.axes.bbox`.

pltrs (Rust) asks the adapter during a layout or a draw. Then the figure
cannot answer about itself. Thus each hook passes its view limits, and
`_viewLim` holds them during the call. The transforms of the class are
matplotlib's Python classes. They reach pltrs (Rust) as the transforms of
the Axes (`_live`). A transform that does not end with `transAxes`
crosses followed by its inverse.

A registered subclass of `PolarAxes`, such as `RadarAxes` of the radar
chart example, works the same way with a polar native Axes.
`PolarAxes.__init__` makes it (`Figure::removed_axes_with_projection`).
It runs the construction of the class: `__init__`, `_gen_axes_spines`,
`_init_axis` and two clears. Then `set_up` gives it to pltrs (Rust)
(`ProjectionState::ProgramPolar`). pltrs (Rust) keeps the polar
settings, ticks and labels. It asks the adapter only for the data
transform and for the spines. The data transform comes from the
`PolarTransform` of the class, whose `transform_path_non_affine` draws
every path. Each clear gives the Axes the patch outline
(`_program.cleared`, `PolarFrame::Path`). A
`Wedge` patch is the exception: pltrs (Rust) fits it to the view, as
matplotlib's `draw` does. Its patch mirror reads the center, angles,
radius and width of the last fit (`PolarAxes._fit_patch`). A
`Spine(axes, spine_type, path)` draws its path through its transform
(`SpinePatch::Path`). During the questions, `get_rorigin` and
`get_rsign` answer from the trees. The non-affine part computes in the
view of the evaluation.

Some subclasses compute their transforms in `get_transform()`. matplotlib
reads it in the `draw` of the artist, so pltrs evaluates it at draw time.
If the class of an artist takes `get_transform` from outside the package,
its figure keeps it (`_computed_transforms`, from `_handles.adopt`). The
draw hook of the figure (`Figure::set_draw_hook`, set through
`_native.transforms.set_draw_hook`) runs after the layout and before the
artists draw. It sets the native transform of each such artist.

### Values as matplotlib types them

pltrs (Rust) holds numbers as `f64` and colors as `Color`. matplotlib
answers with the objects it keeps, often the objects that the program
gave. After `line.set_zorder(5)` the getter gives the int `5`, and after
`line.set_color([0.5] * 3)` it gives the list. `_values`
makes the answers match:

* A setter calls `remember` with the given value. The getter returns it
  (`given`) while pltrs (Rust) still holds its conversion.
* Limits are numpy floats (`float64s`).
* Colors from the property cycle read back as the cycle holds them
  (triples, for `tab10`).
* `to_rgba` gives the int `1` as alpha for a color of three components.

`cargo xtask diff` compares the answers of pltrs with matplotlib's
(`spec/diff`).

matplotlib's setters keep many values unchecked and fail only when they
compute with them. If pltrs (Rust) cannot hold a value, the Python side
keeps it: `_values.keep` for a value that reads back, `_unusable.keep`
for one that fails later. pltrs then draws and measures the artist
through Python, where matplotlib's computation raises matplotlib's error:

* Patches keep their geometry so (`_kept_geometry`). If a field is not a
  number, or the arithmetic fails or warns, the patch recomputes its
  geometry as matplotlib does (`Ellipse._recompute_transform`,
  `Annulus._recompute_path`).
* Spines keep their bounds, center and size. If a spine cannot be laid
  out, its Axes fails to draw (`Spine._locate`).
* pltrs does matplotlib's arithmetic on Python or numpy scalars the same
  way (`Affine2D.scale`, `Path.arc`, `Collection._bcast_lwls`). Thus it
  raises and warns the same.

matplotlib uses some unchecked values only when it draws or measures,
such as the position of a text. If pltrs (Rust) cannot hold such a
value, the artist keeps it (`_unusable`). It then fails with
matplotlib's errors, in the order of matplotlib's draw (`_kept_order`).
A text converts its coordinates as `Text.draw` does
(`Text._set_text_coordinate`). An annotation places its point as
`_check_xy` and `update_positions` do (`_annotation_kept`).

In matplotlib, `hexbin` and `pie` are numpy computations over artists.
pltrs runs that code over its own artists for arguments that pltrs
(Rust) does not take or refuses (`axes/_plotting/_hexbin_reference.py`,
`_pie_reference.py`). Other methods check what the first steps of
matplotlib check, in its order, before pltrs (Rust) computes:

* `hist2d` bins unusual arguments with numpy's `histogram2d`.
* `bxp` merges its properties.
* `stackplot` checks its first layer as `fill_between` does.

### Keyword arguments and aliases

* Declare aliases on the class, as in matplotlib:
  `@_api.define_aliases({"linewidth": ["lw"]})`. `cbook.normalize_kwargs`
  resolves them.
* `Artist.set(**kwargs)` normalizes and calls `set_<name>(value)` for
  each key. It raises matplotlib's `AttributeError` for unknown keys.
  Subclasses override setters as in matplotlib.
* Plotting methods normalize aliases and pass the artist properties to
  the native function in one dict. pltrs (Rust) applies them when it
  makes the artist, so a given color does not advance the property cycle.
  The module file converts the dict into the native property set
  (`lines::LineKwargs` builds `LineProps`). For unknown keys it raises
  `AttributeError("Line2D.set() got an unexpected keyword argument 'x'")`.
  Extend that table when you add properties.
* Python passes native functions exactly their arguments, positional,
  with `None` for an absent one. Defaults and keyword handling stay in
  the Python signature.
* If pltrs accepts a keyword but does not implement it yet, it raises
  `NotImplementedError("pltrs: savefig(bbox_inches=...)")`. Never ignore
  such a keyword silently. The blocker list of the suite then names it.

### Arrays

Python normalizes array-likes with numpy where matplotlib does. Masked
arrays, units and lists go through `np.asarray` in the Python method.
Native functions take arrays as `NdArray`. Results that matplotlib
returns as arrays come back as `NdArray` (numpy arrays). Lists come back
as `Vec`.

pltrs (Rust) keeps copies of what it gets. matplotlib keeps the arrays of
the program and reads them again when it draws. See the next section.

### State programs change in place

In matplotlib, many getters return the attribute itself.
`collection.get_paths()` is the list of `Path`s that the draw reads, and
`line.get_xydata()` is the array of the line path. Programs change such
objects in place and expect the artist to follow, for example
`body.get_paths()[0].vertices[:, 0] = ...`. matplotlib also keeps some
given objects without a copy. Examples are the `xy` of a polygon, the
paths of a `PathCollection`, the array given to `set_sizes` and the path
of a `PathPatch`. pltrs (Rust) holds that state itself.

`python/tests/mutability` (`python -m mutability`) changes in place each
object that a public getter or attribute gives, and each object given to
an artist. It compares the read back, the next draw, the data limits, a
measure and a pickle with `python/tests/mutability/reference.json`. It
asks each row of a drawn scene and of a scene before its first draw.
Some rows call a setter between the change and the next read. `setters.py`
has one such call for each native function that passes by (see "What it
costs").

One mechanism, `_own_state`, handles this state. It has two halves.

**The object handed out or taken in is the artist's own.** A class names
each such piece of state once, as a `State`:

* `read` reads the native value as the object of matplotlib's attribute.
* `take` gives pltrs (Rust) the content of such an object. It uses only
  the native setter of the property, because nothing was set.
* `key` tells what an object holds, such as the bytes and shape of an
  array.

The states of a class are in a separate module, imported when a getter
first needs them (`_collection_state`, `_line_state`, `_patch_state`). An
artist keeps a record per state in its `__dict__`. The getter makes it
with `hand_out`. A setter makes it with `keep`, where matplotlib keeps
what it gets. A record holds the object, its content when pltrs (Rust)
last took it, and the native value then.

* *Identity.* `hand_out` gives the recorded object while the native state
  is unchanged. If another way changed the native state, it gives and
  records a new object, as matplotlib does. A setter that binds a new
  object in matplotlib forgets the record (`drop`) or records the new
  object. After `set_offsets(new)`, the old array belongs to nobody. Some state gets a new object at each draw in matplotlib. Such a
  state has a `version`, the draw count of the artist. If matplotlib
  returns a new object at each call, the getter has no state.
* *Views share.* One buffer in matplotlib is one buffer in pltrs. A line
  shares its `_xy`, `_x`, `_y` and path vertices. A polygon shares its
  `xy` with its path vertices.
* *Read-only stays read-only.* Paths that matplotlib shares between
  artists, such as the unit circle, are handed out read-only. They refuse
  a change, as in matplotlib.

**pltrs (Rust) draws from it.** `sync(artist)` gives pltrs (Rust) each
recorded object whose content changed. It runs only before pltrs (Rust)
reads the artist:

* Before each native call that takes the native object of the artist. A
  native object can ask to be told (`binding::own_state`,
  `_native.artist.tell_before_calls`), and an artist with a record asks.
  This covers every getter, measure, hit test and copy without a list.
  Only the outermost native call of a thread tells. Calls made while an
  object is told tell nobody.
* Before pltrs (Rust) reads the artists of a figure together. The layout
  hook of the figure tells them before each layout (`sync_figure`, see
  "Grids, layout engines, legends"). The package tells them before
  `relim`, a pickle, a measure or a search of the artists
  (`tell_figure`).

What `sync` gives makes nothing stale (`pltrs::figure::unseen`) and moves
no data limit, as in matplotlib. pltrs (Rust) computes limits again where
matplotlib does, such as in `relim`. A `fill_between` collection measures
the points it fills, whatever its polygons are.

**What it costs.** An artist without a record pays nothing. Its setters
test one key of its `__dict__`. Every native call pays a check that no
object asks: 349 ns per call from Python, or 357 ns when any artist has a
record. An artist with records pays a comparison of each recorded object
at each native call that reads it and at each draw. It also pays one
copy of each changed object. The cost is per call so that `contains`, a
measure and a getter see the change at once. Three rules keep it small:

* *Native functions that read none of this state tell nobody.* They are
  listed in `binding::own_state::PASSING_BY`. The list has 35 functions,
  each with a line that says what it reads or writes. The hand-over waits
  for the next call that reads the artist. Thus a setter and a change in
  place are two writes in either order, as in matplotlib. Follow these
  rules for the list:
  * Add a function only after you read its Rust code. An unlisted
    function tells, which is slower but never wrong.
  * If a listed function writes recorded state, only setters that forget
    the record (`drop`) may call it. Otherwise the old object overwrites
    the new value.
  * `test_mut1.py` fails on a listed function without rows in the sweep.
  * The collection setters of face colors, edge colors and alpha are not
    listed, because they read the colors of the other side natively.
* *An array that only its record holds is not compared.* If only the
  record holds an array (`_own_state._only_recorded`), no code can change
  it. pltrs skips it until a getter hands it out again
  (`_Record.settled`). The reference count is compared with a count
  measured at import (`_count_unheld`), because the count differs by
  interpreter version. Only an array that owns its numbers and has no
  weak reference qualifies. For a masked array, nobody else may hold the
  numbers or the mask. Lists, paths and views are always compared.
  Without the GIL, the rule is off, also for the lists of rcParams.
* *Offsets and sizes are compared with pltrs (Rust)'s own.* `State.same`
  compares an array with the native value in place
  (`_native.collections.holds_offsets`, `holds_sizes`). This keeps no
  copy. Only states that nothing native writes may use it: the offsets
  and sizes of 2-D collections. Line numbers do not, because pltrs (Rust)
  converts them again when an axis changes units. If the native value is not bit for bit
  the same (another dtype), pltrs compares with the old content.

A native call that reads an artist costs 0.1 to 0.2 µs without records.
With 1,000 held scatter offsets it costs 1.8 µs, with 100,000 it costs
40 µs. The benchmark `calls/scatter_1e5_read_then_1000_calls` holds the
offsets and calls `set_linewidth` 1,000 times. `test_mut1.py` makes sure
that these calls compare nothing. `set_edgecolor` still tells and costs
about 40 µs on that scatter.

Records hold their objects. pltrs (Rust) holds only a weak callable of
the artist. Pickles leave records out and hold the native state after a
last `sync`. An assignment to a recorded path
(``path.vertices = vertices``) gives the path to pltrs (Rust) at once
(`path_assigned`).

**What is no live object.** pltrs (Rust) computes some state at each
read, where matplotlib stores it. pltrs keeps this state from its first
hand-out, with the same record:

* The path of a wedge, an annulus or an arrow. pltrs (Rust) draws its
  content as the outline (`Patch::set_program_path`) until a geometry
  setter makes the path again.
* The colors of a collection as its last draw mapped them. If the
  collection maps values, changed edge and hatch colors last until its
  next draw (`_collection_state.shown_until_drawn`).

pltrs makes the path of a line drawn in steps at each draw. A program can
read and change it, but the change is not drawn. After such a line drew
its markers, its path is its numbers without steps, new at each draw
(`Line2D._path_without_steps`). This lasts until its data, transform or
draw style change.

**What the package's own code reads** gets no record. Before a getter
makes the first record, it checks the caller
(`_own_state._read_by_package`: the outermost package function in the
call). A member of the artist (``scatter.get_offsets()``, ``line._x``)
or a getter that passes the state on (``shadow.get_path()``) counts as
the program and makes the record. Any other caller, such as `boxplot`,
a legend handler, `getp` or a draw, gets a new object and no record.
Thus pltrs compares only state that a program or a library asked for.
The common reads of every figure skip the getters and the caller check:
`colorizer._array_of(mappable)` for `get_array()` and
`image._extent_now()` for `get_extent()`.

**Transforms** have matplotlib's own way to say that they changed. A
transform given to an artist that holds an `Affine2D` gets a parent node
(`_live_transforms._GivenWatcher`). This covers `set_transform`,
`set_offset_transform`, clip paths and the `transform=` of `plot` and
`scatter`. A change of the `Affine2D` through its methods, as in
`t.clear().rotate_deg(angle)`, gives the transform to pltrs (Rust) at
once. Nothing is compared at a draw.

**The lists of rcParams** belong to no artist. In matplotlib,
`rcParams["font.sans-serif"]` is the parameter itself, and programs
insert into it. An `RcParams` keeps each list that it hands out
(`__getitem__`), with a copy and the native rc text of that time. A new
read gives the same list while pltrs (Rust) holds that text. pltrs first
takes what the program changed. Any native call can read the parameters.
Thus the binding compares the kept lists with their copies before each
outermost native call (`install_lists` in `binding::own_state`). That
costs 12 ns per list. Two rules keep lists off the common path:

* *A list is kept only while a program holds it.* If only the record
  references it (`sys.getrefcount`), nothing can change it. pltrs takes
  its changes and forgets it at the next read of any parameter. Thus
  ``rcParams["font.sans-serif"].insert(0, name)`` keeps a list for one
  statement.
* *A walk over all the parameters keeps none.* Reads in the order of the
  last iteration are a copy (``dict(rcParams)``, ``items()``,
  `__repr__`). The lists belong to the copy, but matplotlib's copy shares
  them. `copy()` also gives separate lists.

The package reads list parameters with `_get`, which keeps nothing. Thus
a program that changes no list in place has no list compared
(`python/tests/test_mut1.py` checks this). A list that stopped being the
parameter, because the program set it another way, belongs to nobody.
The end of an `rc_context` does not put it back. It puts back the lists
of its start, as the same objects with their changes, as in matplotlib.

**Lists of artists** are one list each, as in matplotlib.
`Figure.texts`, `artists`, `legends` and `images`, and `Axes._children`
and `child_axes` hand out the same list each time
(`_own_state.same_list`). Its class adds and removes artists on changes
in place (`_figure_artists.DrawnList` and its subclasses). It updates
itself from pltrs (Rust) when it is used. Nothing is kept for a figure
whose lists no program read. The package reads the lists through
`_texts_now()`, `_children_now()` and similar, which hand nothing out.
If a program also assigns the attribute (`Figure.images`,
`_values.reassignable`), an assignment of the list back does nothing.
The `majorTicks` and `minorTicks` of an axis are its own lists. pltrs
fills them when the axis makes ticks or draws, and leaves them alone in
between.

**What matplotlib makes once and draws nothing from** is kept from its
first read without a record (`_own_state.kept_once_read`). This covers
the `levels`, `layers`, `cvalues` and label lists of a contour set, and
the `texts` and `legend_handles` of a legend. `clabel` fills the same
label lists again.

**State that is no artist's record** is checked when the figure tells
its artists (`_own_state.follow`), without a native call:

* The limit boxes of an Axes with built trees. The watcher of each box
  (`_transform_trees._LeafWatcher`) compares its points array with what
  it last took. Thus `ax.viewLim.get_points()[1, 1] = top` and
  `ax.viewLim.intervalx[1] = right` set the drawn limits. The Axes also
  checks before `get_xlim`. pltrs writes its limits into the same array,
  so the array of a program stays the box's.
* The limit boxes of a 3-D Axes (`_limit_boxes`).
* A `FixedLocator` that keeps an array of the program: the array given to
  it or to `set_xticks`, or its `locs` after a read.
* A colormap whose lookup table a program read (`cmap._lut`, a property
  that detects reads from outside the package). Every live figure asks
  its watch (`colors._LutWatch`). The colorizers give a changed table to
  pltrs (Rust) again, as `Colormap.set_bad` makes them do.

**More states of the kind above:**

* the mapped array by its private name: `_A` hands out what `get_array()`
  gives, and the package keeps the array as `_mapped`
* the vertices of a quad mesh (`get_coordinates()`), taken back with
  `Collection::set_mesh_coordinates`
* the box aspect of a 3-D Axes, the data of 3-D lines and scatters, and
  the `U` and `V` of a quiver
* the clip box of an artist, if it is a `Bbox` of the program
* the patch that a `Shadow` shadows, keyed by the outline that the shadow
  draws
* the triangulation of a `TriMesh`, keyed by its points, triangles and
  mask. A flat `tripcolor`, `triplot` and the contour sets take theirs
  when made, as in matplotlib.

**The first draw of a collection sets its colors again** in matplotlib,
in its first `update_scalarmappable`. A change in the arrays from
`get_facecolor()` or `get_edgecolor()` before that draw reads back, but
is never drawn. After it, the arrays belong to the collection. pltrs
does the same (`shown_until_drawn` of `_collection_state`).

**A draw style set anew** makes matplotlib compute the numbers of a line
again at their next use (``_invalidx``). It takes the x numbers from the
data that the line got, and the y numbers as they are. In pltrs,
`set_drawstyle` does this at once (`Line2D._recache_x`). Thus a change in
the y numbers of the program array is drawn, and a change in its x
numbers is not. The array no longer belongs to the line. matplotlib's
private `_x`, `_y`, `_xy` and `_path` stay the old objects until that
next use, and pltrs makes them new at once.

**Before the first draw** two records live only until the first draw of
the artist (`State(until_drawn=True)`). They are the clip path of an
artist, which matplotlib transforms at the first draw, and the array
given to `Collection.set_facecolor`. Some sweep rows changed before the
first draw still differ, because pltrs took the content when it made the
artist:

* the `colors` of a `ListedColormap`. To follow them, pltrs would compare
  every program colormap at every draw.
* the unit data of an `axline`. matplotlib's first draw shortens the line
  from them by accident.
* matrices and boxes changed before matplotlib caches them, such as
  `dpi_scale_trans` and the `bbox` of an Axes
* the array of a `NonUniformImage`. pltrs draws the change, and
  matplotlib does not.

`expected.json` lists them with the keys `first_draw`, `first_kept` and
`first_value`.

**The 3-D artists** (`mpl_toolkits/mplot3d/art3d.py`) of matplotlib keep
the third dimension in attributes and project it at every draw. pltrs
(Rust) keeps and projects the geometry. Two descriptors give
matplotlib's attributes (`mplot3d/_properties.py`), however the artist
was made:

* `Field` is an attribute of the geometry: `_segments3d`, `_segment3d`,
  `_code3d`, `_faces`, `_invalid_vertices`, `_codes3d`, `_3dverts_codes`
  and the `_dir_vec` of a text. `_verts3d` and `_offsets3d` are records
  already. An artist that the program made keeps the given object. An
  artist that a 3-D Axes made reads the native value
  (`_native.mplot3d.kept`) into the container of matplotlib's method
  (`mplot3d/_fields3d.py`). Examples are a list of arrays for `quiver`
  and `plot_wireframe`, one array for `errorbar`, and lists of tuples for
  `stem`. The object becomes a record at the first read. An assigned
  value stays and goes to pltrs (Rust) at once.
* `Left` is what a projection leaves behind: `_vzs`, `_z_markers_idx`,
  `_offset_zordered`, `_data_scale`, `_facecolors2d`, `_edgecolors2d` and
  the `_path2d` of a patch. At its first read after a projection, pltrs
  computes it with matplotlib's formulas, from the fields and the last
  projection matrix (`Axes3D.M`). It stays one object until the next
  projection. Draws of the Axes and projections that the program asks
  for count as projections. An assigned value reads back until the next
  projection.

The plain values are properties: `_axlim_clip`, `_depthshade`,
`_depthshade_minalpha`, `_closed` and `_zsortfunc`. pltrs (Rust) sorts
only by matplotlib's three `_zsortfunc` functions, and assigning another
raises an error.

*Subclasses of the program's.* matplotlib's `Axes3D.draw` asks each
visible collection and patch for its depth (`do_3d_projection`). If a
program class overrides that method on an `art3d` class, pltrs calls it
before each draw (`mplot3d/_program_projection.py`, behind
`takes_from_program`). It calls the overridden method, which makes pltrs
(Rust) project the artist. The returned depth orders the artist
(`_native.mplot3d.set_program_depth`).

Some 3-D patches are drawn by another patch class. For example, qiskit's
`Arrow3D` is a `Patch3D` and a `FancyArrowPatch`. pltrs does not lift
such a patch into its native 3-D geometry, because that would replace the
arrow by the outline of two points. The patch keeps `_segment3d` in
Python, and `Patch3D.do_3d_projection` projects it (`art3d._draws_itself`).

A program can convert a 2-D collection after a 3-D Axes added it
(`line_collection_2d_to_3d`, `poly_collection_2d_to_3d`,
`patch_collection_2d_to_3d`). pltrs (Rust) lifts a collection only when
the Axes takes it, so pltrs removes it and adds it again
(`art3d._lift_added`). It is then the last child of the Axes.

`Axes3D.relim` is the inherited 2-D method. It puts the projected extent
of the lines into `dataLim`, and nothing of the collections and patches.
The 3-D axes draw nothing from this box.

**Values handed back.** If matplotlib returns what a setter got, pltrs
returns the same object:

* `get_markevery()` gives the given tuple, slice or float.
* After `update_from`, two artists give one clip box. The second holds
  the first weakly.
* `tick_params(reset=True)` makes new `Tick` objects.

Strings and floats that pltrs (Rust) holds come back equal but not
identical. Identity would cost a write on the common path of every
artist.

**What stays a copy, and why.**

* `get_view_interval()` and `get_data_interval()` of an axis. Views would
  build the transform trees at every tick computation.
* The array of `Figure.bbox_inches`. matplotlib's draw also ignores a
  change in it, and only `get_size_inches()` follows it.
* Positions given to a patch, segments given to a `LineCollection` and
  colors given to a line, as lists or arrays. Records would cost a
  comparison per artist and draw.
* The alpha column of the face colors of a collection with its own alpha.
  pltrs (Rust) resolves the colors with that alpha at each draw.

`_own_state.held_path` finds the path that an artist keeps by its
content. Program transforms that cache their results by path use it.

### Units

Plotting methods convert values in units to numbers before pltrs (Rust)
gets them. Values in units are dates, categories and the quantities of a
converter in `units.registry`. The conversion is the matplotlib Python
code:

* `axes/_units.py`: `_process_unit_info`, `_convert_dx`, `_unit_change_handler`
* `Artist.convert_xunits`
* `Axis.update_units`, `convert_units`, `set_units`

pltrs (Rust) has the converters for dates and category names. The native
axis keeps their converter, unit and category names. `Axis.update_units`
and `convert_units` give it their values (`axis_values`), and it installs
their tick helpers. Dates go over as date numbers with the zone of the
first date (`dates._axis_zone`). This zone becomes the unit of an axis
without a unit, as with `default_units`. Python keeps the `tzinfo` as
`Axis.units`. `set_units` with a zone also gives the zone over. Date
locators place ticks on the wall clock of their dates, not always in the
zone of the locator (`tick_values(vmin, vmax)`, `_zoned`).

The program owns any other converter, and its axis keeps it in Python.
`_update_axisinfo` installs its `AxisInfo` natively
(`_tick_helpers.use_axis_info`, `Axis::take_axis_info`). Lines keep its
values (`_xorig`) and convert them again when the unit changes. An artist
made outside an Axes converts its values when it joins one
(`_convert_units_for`). An Axes follows unit changes of its axes only
until its first clear. The clear disconnects the handler
(`Axis::follows_units`) and keeps the data limits until new data replace
them.

Patches, texts and annotations keep values in units as given, because
matplotlib converts them at each draw or measure (`_unit_values`). When
such an artist joins an Axes, it gives pltrs (Rust) the conversion
(`_convert_units_for`). If the units of an axis change (`set_units`,
`set_converter`), the axis converts the values again before the `'units'`
callbacks run. So the handler computes the data limits from new numbers.
This also happens on an Axes without a handler, because matplotlib
patches convert at every draw.

While any geometry field of a patch is in units, the patch keeps all its
geometry fields (`keep_geometry`). The reason is that matplotlib converts
them together, for example the far edge of a rectangle as ``x + width``.
The getters return the given values (`_values.InUnits`). A conversion that
a setter cannot do yet waits for the draw or measure and fails there. An
example is a left edge in dates beside a width in numbers. Fields that
matplotlib uses without units take the Python `float` of the object
(`_kept_geometry.floated`). Examples are the center of a wedge or regular
polygon, and a fancy box. Plain constructors and setters recognize plain
values by their type, without a call, so other artists do not pay.

Category data that a program gives `set_units` (a `UnitData`) replaces the
names that pltrs (Rust) placed (`set_categories`). This also applies on
the axes that share the axis. The `UnitData` becomes their units object.
pltrs reads names added in place (``units.update([...])``) the next time
it converts or reads categories. Masked dates convert as with `date2num`
and keep the mask. The numbers under the mask are those of `date2num`
(seconds since 1970), which `bar` and `stairs` use.

A zone goes over as a native model where pltrs (Rust) has one
(`_date_locators._native_tz`). These zones are the names of `zoneinfo`,
pytz and the dateutil files, and fixed offsets named as
`datetime.timezone` names them. Any other `tzinfo` goes over as a
`_program_zones.ProgramZone` (`pltrs::dates::ZoneRules`,
`TimeZone::Program`). Examples are `dateutil.tz.tzlocal()`, `tzstr` and a
program class. pltrs (Rust) calls its methods as `date2num` and `num2date`
do: `astimezone` to and from UTC, and `tzname`. It asks for the offsets of
all ticks of a locator in one call. It asks for names only where a format
writes them. So it never calls the zone more often than matplotlib. Zones
compare by identity. dateutil `gettz` reads a name for which pltrs (Rust)
has no zone, such as a POSIX `TZ` string in :rc:`timezone`
(`pltrs::dates::set_zone_finder`).

### Colour mapping

Colormaps and norms are the matplotlib Python classes (`colors`). The
registries of `cm` come from the colormap definitions of pltrs (Rust). A
mappable keeps its Python `Colorizer`, and pltrs (Rust) draws with a copy
(`modules/colors/mapping.rs`):

* A native `pltrs::colors::Colormap` parameter accepts any Python
  colormap, as its lookup table.
* A native `Box<dyn Norm>` parameter accepts a norm of a matplotlib class,
  as the pltrs (Rust) norm of that kind with its limits.
* Any other norm, such as a user subclass, maps data in Python. pltrs
  (Rust) calls the callables that the registered `native_norm` of the
  package gives for it (`modules/colors/python_norm.rs`).

pltrs wraps a mappable that pltrs (Rust) makes (an image) and attaches it
with `_ScalarMappable._attach_colorizer`. `changed()` gives the native side
a new copy.

matplotlib maps data through the norm object when it draws, and the
`__call__` of the norm sets missing limits. pltrs (Rust) scales its copy
instead. So when the program next reads the mappable, the mappable gives
the norm object the limits of the draw (`ColorizingArtist._scale_as_drawn`).
This happens only if the norm is still the one given over, and the
mappable was drawn since. Each class gives the limits in `_drawn_clim`. An
image gives those of its resampled data, and a collection those of its
array. A 3-D collection counts the draws of its axes.

Some colorizers map several variates: a `BivarColormap` or a
`MultivarColormap` with a `MultiNorm`. The values are a structured array
or a tuple of arrays, for example in `pcolormesh` (matplotlib 3.11). Such
a collection gets its colors from the Python colorizer, through `to_rgba`
over its norms and colormap (`Collection._map_variates`). This runs each
time the array, mapping or alpha changes. pltrs (Rust) puts the colors on
the sides that its mapping colors (`Collection::set_program_colors`).
Plotting methods of one variate refuse such a colormap.

matplotlib maps a collection when it draws it (`update_scalarmappable`),
and the getters report that mapping until the next draw. pltrs (Rust) maps
on request. So when the values, norm or colormap change, the collection
keeps its last mapping for `get_facecolor` and `get_edgecolor` until the
next draw (`CollectionArtist::mapping_shown`). `set_array` moves the old
mapping there and does not copy it.

### Callbacks and subclassing

Python code runs inside pltrs (Rust) through `Callback`. The native
function takes the callable, and a pltrs (Rust) closure or trait object
holds it. `Callback` is `Send`, because pltrs (Rust) requires this for
code that runs during layout and drawing. `axis/python_helpers.rs` shows
the pattern. The Python formatter of an axis becomes a pltrs (Rust)
formatter. It calls `format_ticks` and then `get_offset`, as the
matplotlib `Axis` does, through the adapter that `_tick_helpers` makes.
During the call, `_axis_info()` and the `Axis` of the formatter answer from
the pltrs (Rust) description of the drawn axis.

Python subclasses that override drawing or computation (`Formatter`,
`Locator`, `Transform`, `Artist`) use the same route. A Rust adapter calls
their methods through `Callback`. A Rust value must hold weakly the Python
objects that can reach it, for example an event callback its figure.
Otherwise the cycle runs through Rust, where the Python garbage collector
cannot see it.

The callback traits of pltrs (Rust) cannot raise. If the callable raises,
the binding does these steps (`binding/callback.rs`):

1. It keeps the exception in the frame of the running native call.
2. The callback returns a fallback (`None` from `call`).
3. The binding skips the later callbacks of that call.
4. When pltrs (Rust) returns, the native function raises the exception.

Each native call has its own frame, so a nested native call cannot take
the exception of its caller. A callback outside any native call turns its
exception into a `RuntimeWarning`.

The matplotlib Python code catches some exceptions of program code. At
these places, pltrs (Rust) asks the binding through
`pltrs::layout::failures`, which `modules/figure_layout.rs` installs.
`binding::forgive` takes an exception of the requested class out of the
frame. The results are those of matplotlib:

* The pass of a layout engine stops where the program code failed.
* `Figure.draw` ignores a `ValueError` of that pass and draws without the
  rest of the layout.
* After a `TypeError`, measuring an axis for a layout measures it again in
  full (`_get_tightbbox_for_layout_only`).

### What keeps a figure alive

A figure goes away when the program forgets it. A web server that draws
per request, a notebook kernel or a window that redraws for hours must not
grow. Two rules keep this true.

* **pltrs (Rust) holds no Python object that reaches back to its
  figure.** Native objects take no part in garbage collection. So Python
  never collects a cycle through native state. A Python object of the
  figure holds each program object that pltrs (Rust) calls. pltrs (Rust)
  holds it weakly:
  * Scales, transforms given to artists and path effects go through
    `_program_classes.held`. The root figure keeps them in
    ``_held_for_native``, because pltrs (Rust) shares a scale among the
    axes that share it. An artist in no figure keeps its own. A `Held`
    proxy pickles the object itself.
  * Tick helpers go through their adapters, which the axis holds.
  * Agg filters go through `_artist_effects.held_filter`, which the artist
    holds.
  * Keywords of plotting methods follow the same rule.
    `_live_transforms.given_to_native` and the `_native_kwargs` of lines
    hold a transform for the root figure of the Axes. A line takes path
    effects through its setter.
  * The colorizer holds a norm or colormap that maps in Python
    (`colors._native_norm`, `_native_colormap`). pltrs (Rust) holds a norm
    read from an archive strongly, because nothing else holds it.
  * The memo of answers holds program maps weakly
    (`transforms::program_calls`).

  The package keeps what it makes for an artist on the artist
  (`_live_transforms.kept`: a clip path, a transform). Do not use a table
  with weak keys for this. These objects reach the artist again, so a weak
  key never releases it. Caches keyed by a program class hold the class
  weakly (`_program_classes.class_cache`). State shared between figures (a
  class attribute, a module table) must not refer to the objects of one
  figure. For example, the shared unit rectangle of `Rectangle.get_path`
  refers to its rectangle weakly. An artist gives back the object that it
  got (``line.get_transform() is t``).
* **A removed artist lives only while something reaches it.** pltrs
  (Rust) counts the handles of each artist (the `FigureCell` census).
  Before a draw, the figure releases the removed artists without handles
  (`FigureCell::release_unheld`). The figure registry (`_handles.Registry`)
  holds the Python objects of artists in the figure, and those of removed
  artists weakly. It sweeps when it doubled since its last sweep. A sweep
  drops the artists that were also out of the figure at the sweep before.
  An artist made for the figure is out of it until pltrs adds it.
  `Figure.clear` sweeps at once. The object of a removed artist holds the
  objects of the removed artists that it owns. A tick helper registry
  gives the helpers of a removed Axes to that Axes.

`_native.diagnostics.live_counts()` gives the counts of figure states,
native objects and kept removed artists (`pltrs::diagnostics`).

The leak harness is `python -m leaks`, run from `python/tests`. It makes,
draws, saves and drops a few hundred scenes under pltrs and under
matplotlib. It reports what stays alive or grows only under pltrs. The
scenes include figures that about forty libraries make
(`leaks/libraries.py`, in the venvs of `cargo xtask ecosystem setup`).

* ``--serve N [lib_seaborn]`` runs a request loop.
* ``python -m leaks.holders SCENE`` shows what holds the objects of a
  leaking scene. On the Python side, these are the objects whose
  references the collector cannot account for, with the chain from a
  module to them.
* `cargo xtask python test` runs the short mode (`leaks/test_leaks.py`).
  It compares with the matplotlib outcomes in `leaks/reference.json`,
  which ``python -m leaks --record`` records.

### Private names other libraries use

Libraries read, write and call private names of matplotlib
(``ax._get_lines``, ``text._color``, ``fig._suptitle = t``). The census
`spec/diff/private_census.json` lists them. `spec/diff/private_census.py`
makes it with ``ast`` from the libraries and corpora installed here.
`cargo xtask diff --private` measures pltrs against it. Each family of
names has one means:

* **Plain fields** are private attributes that a public getter returns
  unchanged (``Text._color``, ``Line2D._drawstyle``).
  `spec/diff/private_fields.py` writes them from the matplotlib source
  into the table `FIELDS` in `_artist_private.py`. A read misses the
  instance and reaches `PrivateAttributesMixin.__getattr__`. It calls the
  pltrs getter of the class that defines the field, not a program
  override. If a package class overrides that getter with a read of the
  field, the read fails. The matplotlib `NonUniformImage` and its
  ``_filterrad`` do the same.

  Some fields have a setter, libraries assign them, and pltrs code never
  assigns them. Such a field gets an assignable class attribute
  (`__init_subclass__`). It reads through the getter and assigns through
  the setter, so the next draw, getter and pickle see the value. To add a
  field, regenerate the table. Native state that no plain getter returns
  gets a property over the native getter and setter. Put it in the class
  or in a mixin of private state.
* **Assignable attributes** are public or private attributes that a
  program assigns and reads back. They use `_values.assignable`:
  * Use ``assignable(getter, setter)`` if later matplotlib code sees the
    effect (a draw, a measure, a pickle).
  * Use ``@assignable`` on a getter alone if matplotlib code never reads
    the value again, or reads it only while it makes the object. Use
    `_values.take` in the method that reads it once.
* **Private helpers** that matplotlib runs over public state
  (``_get_lines(ax, x, y)``, ``_set_gc_clip``, ``_set_artist_props``) are
  ported unchanged. Long helpers go into a lazily imported reference
  module. Forwards keep the matplotlib signature. They set ``__wrapped__``
  to a function with its parameters, as `_AXIS_METHODS` in `axis.py` does.
* If pltrs code keeps internal state under a private name that matplotlib
  also uses, rename it before you add the matplotlib name. For example,
  spines keep `_kept_path` behind the matplotlib ``_path``. Never put a
  property on a name that pltrs code assigns.

### Axes' axes: tick helpers, ticks and texts

An `Axis` (`ax.xaxis`) reaches its pltrs (Rust) axis through its Axes and
its name. pltrs (Rust) keeps all the state of the axis. These mechanisms
give the axis its matplotlib objects:

* **Tick helpers** (`_tick_helpers`, `helpers` in `_native.axis`). Python
  refers to the locators and formatters of pltrs (Rust) through tokens,
  whose identity stays unique while held. Each root figure keeps
  `{identity: (token, helper object, axis)}` (`figure._tick_helpers`). So
  `ax.xaxis.get_major_locator()` returns the same object each time, also
  on the axes that share the axis.
  * pltrs installs a `ticker` locator (or a polar one) set from Python
    natively, from its spec. If an attribute change changes the spec,
    pltrs installs it again in place (`TickHelper.__setattr__`).
  * pltrs installs any other helper through an adapter that pltrs (Rust)
    calls. This includes formatters, because they keep what they computed
    for the ticks drawn last.
  * A helper that pltrs (Rust) installed itself becomes an object of the
    matplotlib class, from its spec (`ticker/describe.rs`). matplotlib
    formatters change in place. So if an attribute change changes the spec
    of such a formatter (`ticklabel_format`, `set_useOffset`), an adapter
    replaces the native formatter.
* **What ticking leaves in helpers** (`_tick_state`, `labelled` and
  `labelling` in `_native.axis`). The matplotlib `_update_ticks` leaves
  results in the attributes of the helpers. Examples are `_locs`, `offset`,
  the order of magnitude and `_format` of a formatter, the `_freq` of an
  `AutoDateLocator` and the `ndivs` of an `AutoMinorLocator`. Tick updates
  are drawing or measuring the axis, reading its tick labels, and the
  axisartist `get_tick_iterators`. pltrs (Rust) numbers each set of ticks
  when a tick update first places it (`TickSet::labelled`). It keeps the
  locations and the description of the labelled axis. The attributes that
  an update sets are `_tick_state.ticked` descriptors. If pltrs (Rust)
  ticks natively, a descriptor first makes the helper object quietly
  repeat the update (`format_ticks(locs)`, a locator call). This happens
  once per set of ticks.
* **The axis as an artist**. The draws of its Axes leave out an animated
  axis, but a print does not. pltrs draws an axis with an image filter
  through the filter (`Axis::set_animated`, `Axis::set_agg_filter`). This
  follows the matplotlib `Axes.draw` and `allow_rasterization`.
* **Mirrors** (`_axis_artists`, `mirrors` in `_native.axis`). The ticks
  (`Tick` with `label1`, `label2`, `tick1line`, `tick2line`, `gridline`),
  the axis label and the offset text are loose `Text` and `Line2D`
  artists. A native `sync` fills them from the axis when pltrs hands them
  out, and every setter pushes the change back (`push`). The properties of
  one tick are those that differ from the style of its level. pltrs (Rust)
  keeps them per tick (`Axis::set_tick_label_props`). The path effects of
  a tick label or axis label go separately, and only after the mirror got
  some (`push_tick_label_effects`, `push_axis_label_effects`). The text
  then draws through them instead of `path.effects`, because the
  matplotlib `Text` keeps its effects (`axis_layout::rc_with_effects`).

The axes of a 3-D Axes (`axis3d.Axis`) are `XAxis` objects, as in
matplotlib. `_native.axis` routes some functions to the 3-D axis that an
`Axes3D` native object and a name (also `"z"`) select (`axis/owner.rs`,
`axis/deep.rs`). These are the functions of the tick helpers, the mirrors,
the tick parameters, the label pad and the tight box. `_native.axis`
refuses the other functions with `NotImplementedError`, so that they do
not act on the projection plane.

A 3-D axis draws every tick with the style of its level, except the label
pad of each major tick. It moves its texts at every draw. So its mirrors
hold the positions of the last layout, and pltrs does not read back
positions set on them. If a program reads the tick labels after a draw,
the labels move back to their locations along the axis, as with
`_update_ticks`. pltrs refuses properties that one tick would keep for
itself.

pltrs ticks a 3-D axis once per layout. It issues the warnings of the
matplotlib tick updates at the matplotlib sites (`mplot3d/tick_sites.rs`).
An example is the numpy warning about the tolerance of minor ticks
outside the domain of the scale. The sites are:

* each draw of the gridlines and of the ticks of an edge
* each placement of a title (the y axis)
* each tight box
* each read of the tick labels or the minor locations

Values in units on 3-D axes go to pltrs (Rust) unchanged
(`Axis._native_units`), and pltrs (Rust) converts them. Axes that share an
axis share its units but not its converter (`Axis3D::own_converter`).

The matplotlib `Tick` objects sit at 0 without a label until
`Axis._update_ticks` places them. Drawing or measuring a visible axis and
reading its tick labels place them. pltrs (Rust) records if the ticks of
an axis were ever placed (`Axis::ticks_placed`, and
`_native.axis.place_ticks` for what Python runs). Mirrors of ticks never
placed sit at 0 without labels. The lines of the mirrors hold the data of
the matplotlib ticks (`Tick._place_lines`). This is first the data of
their class, and after a tick update `(loc,)` along the axis. Their native
lines keep what pltrs (Rust) draws.

The gids of an axis, its texts, its ticks and their parts are gids of the
axis (`AxisGroup`), and mirrors push theirs. The axis draws the matplotlib
SVG groups with them: `matplotlib.axis`, then the `xtick` or `ytick`
groups, then the label and the offset text. A tick group holds a `line2d`
for each visible line and a `text` for each label that is not empty. A
3-D Axes draws its background and the `pane3d`, `grid3d` and `axis3d`
groups of its axes before its own group.

If the locator, formatter or scale of an axis is in Python, the program
ticks the axis where the matplotlib `Axis` does. Examples are a program
class, a `FuncFormatter`, a function scale, a secondary axis and the
colorbar of a program norm. pltrs (Rust) models the call sites of
`Axis._update_ticks` (`axes/tick_sites.rs`):

* `Axis.draw` and `get_tightbbox`
* the siblings and the spine of the label placement
* the title placement
* the spines of `Axes.get_tightbbox`
* the passes of the layout engines
* the draw before a print

At these sites, pltrs (Rust) calls the `TickUpdater` of the figure
(`axes/tick_updates.rs`). `_tick_updates` installs it once a figure has
such an axis. It runs the matplotlib `_update_ticks` with the program
objects. So the program code runs as often, in the same order and with
the same arguments as under matplotlib. pltrs computes its own helpers
natively where it needs them. The ticks that the program gave stay valid
while the geometry stays the same.

If a program axis class answers `get_view_interval` itself (as skewt
does), the program also ticks that axis (`AxisState::program_view`). The
package locators and `ScalarFormatter` ask the axis for its view interval
where matplotlib does. Autoscaling asks the axis if it is inverted
(`TickUpdater::inverted`), as `set_xbound` does.

pltrs calls a program scale where matplotlib artists map through it
(`axes/transform_sites.rs`, walked with the tick updates). A program scale
is a function scale, a scale class or the functions of a secondary axis.
The sites are:

* every tick label measured (`_get_ticklabel_bboxes`) and drawn
* every tick line and gridline drawn: its path, then its points
* every spine measured and drawn
* the view limits, once while they stay the same
  (`TransformedBbox(viewLim, transScale)`, `AxesState::scale_box`)
* the path of a line, kept while its data and transform stay the same
  (`lines/program_path.rs`, the matplotlib `TransformedPath`). A line that
  draws a slice of its points maps it again at every draw.
* the coordinates of a mesh, at every draw (`QuadMesh.draw`)

Each call gets the arrays that matplotlib gives. For a blend of
`transData` and `transAxes`, these are the coordinates that the blend
maps. The shape is:

* a row where drawing maps through `transform_non_affine` of the scale
* a column where matplotlib calls `transform`: autoscaling, the tick
  updates, the function of a `functionlog` scale
  (`_scale_calls.ScaleCalls`)
* a masked row for the coordinates of a mesh (`ArrayLayout::MaskedRow`)

pltrs also lays out and draws the same parts in its own way, with the
same values in other batches. That work runs quietly
(`transforms/program_calls.rs`). The program maps answer it from their
answers since the layout began. So the program sees only the matplotlib
calls. pltrs computes a curvilinear grid when a floating axis is made, as
the matplotlib grid helper does (`AxisArtist._init_line`,
`_native.axisartist.init_line`, with the figure free). It computes the
grid again only for other limits.

pltrs calls the transforms of a program projection at the same sites
(`axes/projection_sites.rs`). Each tick label measured and drawn maps its
point through the label transform of its side. The `tick_label` of the
adapter gives this transform evaluated. Each gridline and tick mark maps
its path, then its points. The adapter keeps the maps of the plane that it
evaluates, per transform and view (`_Adapter._kept`). So the called Python
object keeps the answers of the program (`PointMap::program_key`,
`_answers_alike`), and the quiet work of pltrs answers from them.

pltrs calls a program norm (`PythonNorm`) as the matplotlib colorbar and
mappables call it:

* A collection autoscales the norm once, when it is made (`_scale_norm`).
  At every draw, it normalizes its array as a masked array of its shape
  (`update_scalarmappable`, `Mapped::normalize_drawn`). This call is a
  site of the draw of the Axes (`program_collection_drawn`), and the paint
  uses the result.
* A colorbar autoscales the norm to the values of its mappable in each
  `_process_values`. It normalizes its boundaries in `_mesh` and
  `_do_extends`, and the values of its extensions alone
  (`Norm::normalize_scalar`). It normalizes its boundaries again wherever
  the matplotlib `_ColorbarAxesLocator` runs and applies the aspect
  (`colorbar_located`): `_get_draw_artists`, `Axes.draw` and tight boxes.

If a norm comes from a scale (`norm._scale`, the functions of a
`FuncNorm`), that scale scales the long axis of the colorbar. It calls the
program functions. For another norm, a function scale calls the norm
itself. The arrays are those of matplotlib (`Norm::normalize_shaped`,
`ArrayForm`). They are plain for boundaries and axis values. They are
masked for the array of a mappable and the coordinates of a mesh.

Axes that share an axis hold the same `Ticker` objects, as `sharex` gives
them. These are the objects of the first Axes of the chain
(`Axis._take_shared_tickers`). Through them, the Axes read the helpers
that pltrs (Rust) shares among them. A helper is bound to the axis that it
was last set on (`_tick_helpers._locator_axis`).

While pltrs (Rust) lays out a figure, it works on the figure state taken
out of its cell (`FigureCell::work_on`, `figure/lend.rs`). For each call of
the program tick updates, it lends the state back to the cell
(`FigureState::lend`). So the program finds the figure free, as the layout
left it. It can read the figure (limits, ticks, positions) and change it
at once, and the rest of the layout sees the change. Only the thread that
lays out the figure finds it free.

While pltrs (Rust) paints a figure, the state stays in its cell, lent to
the painting thread (`FigureCell::paint`, `figure/painting.rs`). The draw
borrows the state only while it reads it. A `DrawContext` borrows it when
first asked. It releases it when it hands out its renderer or on request
(`DrawContext::free`). `DrawContext::free` takes the context mutably, so
nothing read from the figure stays borrowed. The draw does not borrow the
drawn artist from the figure either. The figure keeps its artists shared
(`artist/kept.rs`), and the draw holds the artist it draws. A change to
that artist goes to a copy, which the figure keeps from then on. The draw
finishes the artist as it found it. For this reason every artist is
`Clone` (`ArtistClone`), and every projection too (`ProjectionClone`).

So program code that a draw runs finds the figure free. Some of this code
runs where the draw calls a renderer: a path effect, a program renderer,
the TeX painter. Other code runs after the draw releases the figure: a
replaced `Text.draw`, a recorded `draw` override. This code can change the
figure and read the change back. It can make changes that return what
they changed (`set_title`), and the rest of the draw sees them. A figure
that pltrs paints counts as laid out. An extent measured meanwhile is the
extent of its layout (`FigureCell::is_painted`).

Program code that maps values for a draw also runs with the figure free.
At each call site, pltrs gathers what the call needs. Examples are a
resolved transform, the wrapping box, style and resolution of a text, and
the canvas of a box image. Then it releases the figure, makes the call,
and continues the draw with the result:

* A transform knows if it runs program code
  (`ResolvedTransform::calls_program`, from `AxisMap::calls_program` and
  `PointMap::calls_program`). `DrawContext::free_for` releases the figure
  before such a transform maps a line or its markers. The same applies to
  a gridline, a tick mark, a spine and the position of a text.
* pltrs lays out the texts of an axis through a `FigureReader`: the tick
  labels, the axis label, the offset text and the spines they clear. In a
  draw, the context releases the figure around three calls. These are the
  placing transforms, a program projection that places a tick label, and
  a program text measurer (`text::has_line_measurer`). In a layout, the
  context holds the figure.
* An image releases the figure before it maps its data through its norm
  and transform. It maps a copy of its mapping and puts the copy back
  after, because a program norm can change the image, which copies it.
* A box image releases the figure before a program function gives its box.
* Autoscaling lends the figure to the program locator and scale during a
  layout (`FigureState::lend`), as `autoscale_view` runs them in the draw.
* An axis maps values through its scale apart from the figure
  (`Axis::scaled`), for the transforms of the tick updates.

Draws without program code pay nothing: a transform that calls no program
code keeps the figure borrowed.

Program code that makes or maps the outline of a patch (a program
transform or box style) also runs with the figure free. A draw releases
the figure first (`PatchParts::program_outline`,
`PatchShape::MAPS_OUTLINE_ALONE`). pltrs measures the data limits of an
artist added to an Axes apart from the figure. The reason is that
`_update_patch_limits` makes the path first and calls a box style even for
a patch in axes fractions.

pltrs asks a projection for its data ratio with the figure lent
(`FigureState::data_ratio_of`, while `Axes::apply_aspect` works on the
figure apart). It asks for the spans of the spines in the same way
(`Spine._adjust_location`). It asks after a layout settles the view, and
again after the figure is laid out for a draw. While the program moves the
Axes in its answer, pltrs asks again (`FigureState::span_program_spines`).
The layout and the draw use the answer for that view
(`AxesState::spine_interval`). A tight box maps the positions of its texts
through a program transform with the figure lent. Then it measures them
quietly (`program_texts_measured`).

Other code that pltrs (Rust) runs during a layout or draw finds the
figure in use. During a layout, the state is out of the cell. During a
draw, the state is borrowed for reading. This code is:

* the code that the resolution of a transform runs. These are the hooks of
  a program projection other than its data ratio and spine spans: its
  evaluated transforms, its `get_view_interval`, its `Tick.draw` probe.
* the axisartist grid finders where an axis artist or its gridlines draw,
  except `AxisArtist._init_line`
* texts measured through program transforms outside tight boxes

This code can still change the figure in matplotlib. So a native call made
while another native call of its thread runs goes through
`pltrs::figure::reenter` (`binding/function.rs`, `binding/reentrant.rs`).
`reenter` stops the call where it reaches the figure in use, so that the
`RefCell` does not panic:

* A native function that returns nothing is a change (`set_color`,
  `set_text`, `set_visible`...). pltrs queues it with its arguments and
  calls it again when the figure is free. In a draw, this is after the
  artists are drawn and before the figure counts as drawn and sends its
  draw event (`pltrs::figure::set_draw_end_handler`). Otherwise, it is
  when the outermost native call of the thread ends. That native call
  raises the exception of the change.

  pltrs makes each change at the tick of the thread draws at which the
  program asked for it (`pltrs::figure::replaying_changes_from`). An
  artist changed during its draw ends fresh, and an artist changed after
  its draw ends stale. A call queued again (a path effect that runs for
  every marker) takes its last place. Changes that queue changes run in
  rounds. The rounds end when a round queues exactly itself again (a
  `relim` that runs a transform that calls `relim`). After 64 rounds,
  pltrs raises `RecursionError`.
* Any other native function raises `RuntimeError`: the figure is in use.
  This applies to a read during a layout (a formatter that calls
  `ax.get_xlim()`). It also applies to a change that returns what it
  changed (`set_title` returns the title) during a draw or layout.

Reads during a draw work. A read of a view limit borrows the figure for
change only if autoscaling is pending (`FigureState::view_is_stale`).
Event callbacks (`draw_event`, limit changes), `draw` overrides and
program axes locators run with the figure free.

Some callbacks that the matplotlib draw calls can draw the canvas
(`canvas.draw()`, `draw_idle()`, `fig.draw(canvas.get_renderer())`). The
Agg canvas runs `draw_idle()` at once. Such callbacks are formatters,
locators, transforms, norms, path effects, the `draw` of an artist, and
`xlim_changed` callbacks of limits that the draw autoscales. matplotlib
then paints the whole figure into the buffer of the running draw, and the
outer draw paints the rest over it. A PNG save also draws into the
renderer of the canvas. So some artists come out twice, and a state
halfway through autoscaling can show.

pltrs (Rust) calls most of these callbacks during layout and paints the
figure once, in its final state. The nested draw runs on the lent figure
and paints the buffer of the canvas, not the buffer of the save. pltrs
does not copy the doubled image, because no program asks for it. What the
callback reads, the state after the draw and every later draw are those
of matplotlib. The `reentry` generator checks this with its
`xlim_changed-stale` kind and its `--draw_idle-at-save` programs. The
image difference is in `spec/diff/allowlist.txt`. A `fig.draw(renderer)`
from code that pltrs runs while it paints (a path effect, the `draw` of an
artist) paints into that renderer.

A draw stops where program code that runs before painting fails. If a
layout hook or draw hook of the package raises (`DrawHook::stopping`), the
figure is laid out but its artists are not drawn.

matplotlib accepts data that its 3-D artists cannot project, and fails
only when it draws them. Examples are a line through 2-D points, segments
of 2-D points, and positions with coordinates of different lengths. In a
3-D Axes, 2-D collections and patches also fail, because they lack the
`do_3d_projection` that the draw asks for. When the package gets such
data, it runs the matplotlib computations (`mplot3d/_draw_problems.py`,
ported from `art3d` and `proj3d`). It records their exception with pltrs
(Rust) (`Axes3D::set_draw_failure`, `pltrs::mplot3d::DrawFailure`) and
keeps the data as given. The draw of the Axes then does these things:

* It reports the exception where matplotlib meets it. Collections and
  patches are projected after the background, and lines and texts are
  drawn among the other artists.
* It leaves the artist unprojected and undrawn.
* It projects no line in a draw that ends at a collection.

`do_3d_projection` raises the exception. It runs the same computations
through the projection of the last draw (`M`). Before the first draw, it
fails as in matplotlib.

A `Line3D` holds its 3-D data and the 2-D data of `Line2D`. Each draw
replaces the 2-D data with the projection. If the program sets 2-D data
(`set_xdata`, `set_data`), pltrs measures the line by them until the next
draw (`get_path`, `get_bbox`, extents). `Line3D._numbers` gives them to
pltrs (Rust) while they are newer than the last draw (`_data_set_at`). The
matplotlib draw converts no 2-D data. So data that converts to no numbers
leaves the projection in place.

Sometimes a figure holds an artist, but matplotlib does not give the
artist the figure. An example is a child that an `AuxTransformBox` takes
in a figure, because its `add_artist` sets no figure. Such an artist is
figureless (`ArtistBase::figureless`), and its `get_figure()` is None.
Measuring or drawing a text among these artists raises the matplotlib
`AttributeError` (`OffsetBoxError::NoFigure`).

Some parasites of an axes_grid1 host follow the view limits of the host
(`twin`, `get_aux_axes(viewlim_mode=...)`). They take the limits when the
limits of the host change (`FigureState::sync_parasites`), as `_sync_lims`
does on `xlim_changed` and `ylim_changed`. Between two changes, a parasite
can have different limits. A parasite with an aux transform takes the
`transAxes` of its host. A parasite without one has its own box and
transforms (`Parasite::active`):

* At first, the box is its own position.
* The draw of the host gives it the box of the host (`HostAxes.draw`
  calls `apply_aspect(rect)`).
* `apply_aspect()` brings back its own position. Each measure of its tight
  box calls `apply_aspect()`.

Between draws, its ticks, tight box and transforms are those of that box.
The Python object keeps the given positions (`set_position`,
`set_subplotspec`) for its getters.

Before a print, matplotlib sometimes draws the figure once without a
renderer: with a layout engine, or to cut the figure to its tight box. A
print renderer notes some things while it draws, for example the partly
transparent colors of the PostScript backend. It logs them as drawn, so
also when an artist fails later. If the earlier draw fails first, the
print renderer notes nothing (`Figure::render_vector_noting`).

matplotlib keeps some parts as separate artists, and pltrs (Rust) draws
them as parts of their artist:

* the box of a text (`Text.get_bbox_patch()`)
* the arrow of an annotation (`Annotation.arrow_patch`)
* the arrow of an annotation box (`AnnotationBbox.arrow_patch`). This is
  the matplotlib patch until the box joins a figure, then a mirror.

These parts are mirrors, as the ticks are (`_text_box`,
`_annotation_arrow`). They are loose `FancyBboxPatch` and
`FancyArrowPatch` objects filled from the look of the part
(`_native.text.bbox_patch`, `_native.annotation.arrow_patch`,
`_native.offsetbox.annotation_arrow_patch`). Their setters give changes to
the part (`restyle_bbox`, `restyle_arrow`). The `get_window_extent` of the
arrow is where the annotation draws it. `Text.get_window_extent(annotation)`
is the text alone. The box patch moves where each draw of the text placed
it. Its path and transform follow the draw count of the text.

Mirrors are objects of the matplotlib classes, not of subclasses
(`type(ax.set_xlabel("x")) is Text`). pltrs makes `Text`, `Line2D`,
`FancyBboxPatch`, `FancyArrowPatch` and `FontProperties`
`_mirrors.mirrorable` (or `pushing`). An instance attached to a part has a
`_mirror`, a `_mirrors.Mirroring`. Every setter of such an instance pushes
the instance into the part, and the part answers for the extent and the
figure. The `FontProperties` of a text mirrors the font of the text. It
stays the same object while the font stays the same.

`ContourLabeler.clabel` and `add_label_near` place every label through the
Python method `_split_path_and_get_label_rotation`, as the matplotlib
`labels` loop does. pltrs (Rust) places the labels. It calls the method
through the callable that Python gives `_native.contour.clabel`, with each
line, vertex index, position, width and spacing. The method returns the
rotation and the cut line that pltrs (Rust) computes
(`_native.contour.split_path`).

### Staleness

pltrs (Rust) keeps for each artist if it changed since its last draw
(`ArtistHandle::stale`). A draw makes the artist fresh. A change through
its handle, or a child added or removed, makes it stale again. Its
containing artists and the figure also go stale. Animated artists are an
exception (`figure/stale.rs`). `Artist.stale` reads and sets this flag, so
the matplotlib propagation through `stale_callback` needs no Python. Some
matplotlib draws leave the artist stale, for example an annotation with a
clipped point or an inset indicator. For such an artist,
`Artist::freshened_by_draw` returns `false`. The draw counts, and the
artist stays stale.

matplotlib draws some artists as parts of another artist, and pltrs (Rust)
draws them with it. Examples are an `Axis` with its Axes, table cells with
their table, and the boxes in an added offset box. Such an artist is fresh
if its drawer drew since it went stale (`_drawn_with.DrawnWith`, through
`_native.artist.draw_count` of its `_drawer()`).

A program can put a `stale_callback` on any artist, also on the figure.
It sees the changes of pltrs (Rust), which set no `stale` in Python. The
root figure of the artist reports the artists that its changes make stale
(`Figure::report_stale`, `_artist_staleness.watch`). It reports the
innermost artist of each change, or the figure if no artist went stale (a
scale, the subplot parameters, the size). At the end of the outermost
native call of the thread, the binding gives them to
`_artist_staleness.stale_reported`. It sets each one stale as the
matplotlib setters do and runs the callback chain up to the program
callback.

pltrs does not report two kinds of change (`_artist.unreported`): what the
program sets stale itself, and what a mirror gives its part
(`_mirrors.push`). The matplotlib part has no callback. Every change that
makes the figure stale calls its callback at least once. pltrs does not
copy how often matplotlib calls it (11 times for `set_title`, dozens of
times per draw). Setters that leave matplotlib artists fresh also leave
pltrs (Rust) artists fresh. Examples are `gid`, `url`, `picker`,
`rasterized`, `in_layout`, `navigate`, the `wrap` and `bbox` of a text,
and the line and cap styles of a collection.

### Warnings

pltrs (Rust) issues warnings through `pltrs::warnings`, for example when
it casts data to a narrower type to draw it. The extension sends them to
`_api.warn_external`, which `_api` registers with the extension. So they
are matplotlib `UserWarning` warnings. If a
warning filter turns one into an exception, the native call that issued it
raises it, as with a callback exception. Where numpy arithmetic in
matplotlib warns (`divide by zero encountered in log10`, the invalid
tolerance of isclose), pltrs (Rust) does the arithmetic itself. It issues
the same warning in the same case with `warnings::warn_runtime`, as a
`RuntimeWarning`.

### Warnings issued while pltrs (Rust) works

Some matplotlib warnings come from deep inside a draw or a layout, where
pltrs (Rust) cannot call Python. An example is "Glyph 128579
(\N{UPSIDE-DOWN FACE}) missing from font(s) DejaVu Sans.". matplotlib
issues it when a renderer measures an uncached line with a character that
no font has.

pltrs (Rust) queues such events on the thread
(`pltrs_text::take_missing_glyphs`). `font_notes` in `pltrs::text` notes
them where pltrs (Rust) measures a line that its cache does not hold. At
the end of each native call that succeeds, `binding::after_call` runs the
functions that `modules::register` installed. These functions drain the
queues and issue the warnings through package functions registered with
`_native.register`. Here this is `_text_helpers.warn_on_missing_glyph`,
which calls `_api.warn_external`. If the warning filters make a warning an
error, the call raises the error in place of its result.

The font manager log messages about the fonts of a text use the same path.
pltrs (Rust) records each lookup that matplotlib logs about
(`pltrs_text::take_font_lookups`) where matplotlib calls
`_find_fonts_by_props`:

* when `Text._get_layout` takes the font height,
* when a line is measured uncached,
* when a line is drawn (`_prepare_font` in `draw_text`),
* for tick labels, again where an axis places its label
  (`_update_label_position`).

The hook gives each lookup to `_text_helpers.report_font_lookup`. It looks
the fonts up again with the `fontManager` of the program
(`_find_fonts_by_props` or `findfont`). It logs a substituted weight once
per font properties, because `findfont` caches, and a missing family at
every lookup. Two more kinds of lookup use this path:

* **AFM fonts** (`ps.useafm`, `pdf.use14corefonts`, `LookupKind::Afm`).
  pltrs reports every lookup, because only the font manager knows what it
  logs, for example `Failed to find font weight normal for Helvetica, now
  using medium`. The reporter calls `findfont(fontext="afm",
  directory=...)` as `_get_font_afm` does, and `_find_fonts_by_props` for
  the PDF core fonts, as `fontName` does.
* **The deprecated `text.kerning_factor`.** While it is set, pltrs (Rust)
  reports every lookup (`pltrs_text::keep_every_lookup`). The reporter opens
  the fonts as `get_font` does. Thus `FT2Font` warns about
  `_kerning_factor` for each font that `_get_font` did not cache, the Last
  Resort font included.

### Warnings of matplotlib's transform graph

matplotlib's transforms compute lazily, with numpy. Some nodes keep their
matrix until a leaf changes (`BboxTransformFrom.get_matrix`). Some make it
at every call (`CompositeGenericTransform.get_affine`, which is
`transData`). Some are made again at each request (the
`get_patch_transform` of a rectangle). Some leaves are not ordinary
numbers, for example view limits without width or a tick pad of `inf`.
Then the arithmetic warns once for each node computed (`invalid value
encountered in dot`). Thus the caching of matplotlib decides how many
warnings a statement gives.

pltrs (Rust) computes none of these nodes. The package keeps matplotlib's
graph over the leaves of pltrs (Rust), with matplotlib's invalidation (see
Transforms). To give the same warnings, pltrs evaluates this graph where
matplotlib evaluates it, with the same calls in the same order
(`_transform_replay`):

* **The arithmetic warns as in numpy.** `Affine2D.translate`, `scale` and
  `rotate` are matplotlib's numpy code. The node matrices of
  `_native.transforms` (`compose` for `np.dot`, `transform_from`,
  `transform_to`, `transform_between`) issue numpy's warnings for the same
  floating-point exceptions (`pltrs::numeric::numpy_arith`). A product
  warns as OpenBLAS does: its fused sums hide `0 × inf` behind a sum that
  is already NaN. Where numpy's products raise no exceptions, pltrs issues
  none (`warnings::PRODUCTS_WARN`).
* **The nodes cache as in matplotlib.** `CompositeGenericTransform` keeps
  its affine part only while both factors are tame
  (`_native.transforms.tame`).
* **The leaves invalidate as in matplotlib.** A leaf invalidates its nodes
  when its value changes, and NaN equals itself. The view limits invalidate
  at every set, also to the same values (`AxesLeaves::view_sets`), as
  `viewLim.intervalx = ...` does in `set_xlim` and `set_view_interval`.
  Handing over leaves evaluates nothing. Otherwise a node would already be
  computed, and silent, where matplotlib's draw first computes it.
* **Each site evaluates what matplotlib's code evaluates there.**
  `add_patch` and `relim` run the search of `_update_patch_limits` for
  `transData` (`patch_limits_updated`). Plotting methods whose patches
  pltrs (Rust) adds report this (`_transform_trees.patches_added`). A draw
  gives each Axes, background, artist and axis that it reaches to the
  evaluation hook of the figure (`Figure::set_evaluation_hook`,
  `pltrs::transforms::Evaluation`). For example, `Patch.draw` makes its
  transform and takes its affine part. `Line2D.draw` takes the affine part
  twice, for the line and for the markers. `Collection.draw` gives the
  offset transform as a matrix. A polar Axes fits its wedge through
  `transWedge`. Each artist reads its clip path and logs one that is not
  finite. `Axis.draw` places the
  ticks, measures and draws their labels, then measures again to place its
  label. pltrs (Rust) names these labels with their parts and pads. An
  artist that the draw does not reach evaluates nothing. An artist that a
  program draw method draws twice evaluates twice.
* **Only the graph is evaluated, on no data.** Paths and points go through
  empty. pltrs (Rust) maps the data and issues data warnings itself, such
  as for the logarithm of a negative coordinate. The tick positions of
  polar Axes are the exception, because their result depends on settings
  that pltrs (Rust) does not report.
* **Each warning comes once.** If pltrs replays a computation, the
  recomputation of the patch geometry before it (`_unusable.check_ahead`)
  is silent.

**Nothing is replayed while every leaf is tame.** A tame leaf is a finite
number of ordinary size, or a view with some width
(`pltrs::transforms::is_tame`, `is_tame_span`). Other leaves are wild.
A draw checks the figure
leaves once (`FigureState::graph_is_wild`) and the transform of each patch
(`Artist::has_wild_transform`). Only a wild figure or a wild patch reaches
the hook. `add_patch` and `relim` first ask `_native.transforms.replays`,
through `_transform_trees.patch_added` and its kin. These functions load
`_transform_replay` only when they need it. A tame graph raises no
exception, so the skip loses no warning. A leaf that becomes wild
invalidates every node built on it, so the uncomputed nodes also match.

**Measuring is replayed like drawing.** matplotlib's `Axes.get_tightbbox`
evaluates the graph in this order:

1. Each axis (`Axis.get_tightbbox` places the ticks and the axis label,
   then measures them).
2. The title placement (`_update_title_position`). It measures the x axis
   of each Axes that the titles must clear and that has decorations on top.
   Then it measures their y axes for each title with a text. Every draw
   does this too.
3. Each artist that takes part in layout and is not clipped to the Axes.

pltrs (Rust) gives these evaluations to the hook where it takes the tight
box of an Axes (`axes_tight_bbox_at`, `tight_bbox_of_axis`). It lends the
figure to the hook meanwhile (`FigureState::measuring_evaluated`). It gives
the axes as `Evaluation::Axis { measured: true }`, then the title axes,
then `Evaluation::Measured`. For the last, the package finds the artists
itself (`get_default_bbox_extra_artists`). Thus `get_tightbbox` of an Axes,
an axis or a figure evaluates as often as in matplotlib. So do these:

* A tight print. matplotlib draws, measures and draws again. The single
  pltrs draw gives each evaluation twice.
* The layout engines. Constrained layout measures in both passes. Tight
  layout measures once, and again to fit a rectangle.

A measurement checks only the figure size and the leaves and place of the
measured Axes (`FigureState::measurement_evaluated`). `add_patch` and its
kin check the Axes leaves and the transform of the artist
(`evaluates_wild`). The package methods that measure one artist
(`Text.get_window_extent`, `get_window_extent` and `get_tightbbox` of a
legend) check the same way (`_transform_trees.measured`).

Artists that the package draws or measures lazily keep their own count of
draws (`Annotation._positioned_by_draw`, `Quiver._transform_by_draw`). The
replay makes them once per draw. Their getters make them for the draws
that ended.

**Pickled figures keep what they computed.** matplotlib pickles its nodes
with their matrices and validity. After unpickling it computes only the
invalid nodes, and the nodes on `dpi_scale_trans`, which
`Figure.__setstate__` invalidates. The package rebuilds its trees instead
of pickling them. The state of each owner records which nodes were valid
(`_transform_trees.without_trees`). The rebuilt trees compute these nodes
quietly, except the nodes on the resolution (`_restore_validity`).

pltrs does not replay these evaluations:

* The first draw of a tight print, before its measurement. The counts
  agree unless the nodes were invalid before the print.
* What matplotlib's `AxisArtist` evaluates when it measures.

### Errors

Each module file implements `From<pltrs error> for PyErr` with
matplotlib's exception class and, where possible, its message. For
example, `RcError::UnknownKey` becomes `KeyError: '<key> is not a valid rc
parameter (see rcParams.keys() for a list of valid parameters)'`. Tests
match messages with `pytest.raises(..., match=...)`. If a message differs,
fix it in the `From` implementation, or in pltrs (Rust) if its message is
wrong. The package registers exceptions that matplotlib defines in Python,
such as `ImageComparisonFailure` (`_native.register`). The binding raises
them through `binding::registered_error`.

pltrs allocates fallibly what program data can make enormous: a canvas,
an encoded file, or a copy of either (`pltrs_core::alloc`). If the system
refuses memory, or a canvas exceeds `pltrs_raster::MAX_PIXELS`, the error
has the cause `OutOfMemory`. The binding raises `MemoryError`, and the
process continues.

The renderers check their arguments in Python before pltrs (Rust) draws,
as matplotlib's renderers do (`backends/_pdf_arguments.py`,
`_ps_arguments.py`, `_svg_arguments.py`,
`_backend_agg.check_renderer_arguments`). Thus wrong arguments fail with
matplotlib's errors. A vector renderer draws a path collection that is too
small to reuse its paths path by path, through
`RendererBase.draw_path_collection`. The artist draws refuse an object
that is not a renderer (`_artist_subclassing._refuse_renderer`).

pltrs (Rust) draws past a problem and returns all problems
(`DrawErrors`), each with the exception kind of matplotlib
(`DrawErrorKind`). The binding makes draws stop at the first problem
(`pltrs::artist::set_draws_stop_at_first_problem`), as matplotlib's draw
stops at its first exception. It raises a `ValueError` for a value that
cannot be drawn, such as malformed mathtext. Otherwise it raises a
`RuntimeError` that names all problems. `draw_without_rendering` raises
them too.

**If an external program is missing,** pltrs fails where matplotlib fails,
with the same exception:

* Missing TeX for usetex gives `RuntimeError("Failed to process string with
  tex because latex could not be found")` on every path that typesets
  (`TexError::Missing`, `From<TextError>`, the draw error kind `Library`).
  A renderer's `draw_tex` typesets before it reads its other arguments.
* A TeX file that kpathsea cannot find gives the `FileNotFoundError` of
  `dviread`.
* A missing TeX system of the PGF backend gives its `RuntimeError`. The
  `draw` of the PGF canvas starts TeX where matplotlib's draw measures a
  text.
* A missing Ghostscript or `pdftops` for `ps.usedistiller` skips the
  distiller with matplotlib's logged warning (`_try_distill`). pltrs writes
  the file undistilled.

`python/tests/test_allow1.py` checks each case with none of these programs
on `PATH`.

### What a draw did before it failed

matplotlib's draw stops at the first artist that raises, after the earlier
artists logged or warned. pltrs (Rust) reports the failure when the native
call returns, and the package drops the callbacks of a native call after
one raised (`binding::callback`). Thus a failure from a Python painter
would also drop renderer notes, such as the PostScript "does not support
transparency" warning. pltrs gives these notes as work done before the
failure (`pltrs::warnings::as_done_before_a_failure`, run with the failure
set aside).

### Text measured and drawn by the program

matplotlib measures every line through `text._get_text_metrics_with_cache`
and draws every text through `Text.draw`. Its tests replace both to turn
texts into boxes (`monkeypatch.setattr`, the `text_placeholders` fixture).
`_text_overrides` gives pltrs (Rust) two objects once:

* A `pltrs::text::LineMeasurer`. pltrs (Rust) measures every line, tick
  labels included, by calling the module function. The font height comes
  from `FT2Font.get_sfnt_table`, as in `Text._get_layout`.
* A `pltrs::text::TextPainter`. pltrs (Rust) draws every text by calling
  `Text.draw` on a stand-in `Text` in display coordinates. The renderer
  queues the artists that its `Artist.draw` gets, and pltrs (Rust) draws
  them in place of the text.

Each object holds the attribute it stands for (`binding::Watched`: module,
name and original function). pltrs (Rust) reads the attribute at each use
(`in_force`) and uses the object only while the attribute is not the
original. Thus the module and the class stay plain. pltrs uses no module
class or metaclass to watch assignments, because programs would see it in
`type(Text)`. Without replacements, pltrs (Rust)
never calls Python for text.

matplotlib caches line metrics per renderer
(`text._get_text_metrics_function`, a `WeakKeyDictionary` of `lru_cache`s),
and its tests read the caches. pltrs keeps them natively
(`pltrs::text::MetricsCache`, `_text_metrics`). While pltrs (Rust) lays out
or draws with a renderer, `text._measuring_with` installs the cache of that
renderer on the thread. This happens in canvas draws and prints,
`draw_without_rendering`, `tight_layout` and `Text.get_window_extent`.
pltrs (Rust) then finds or adds each line in the cache without a call
into Python. Measuring from Python uses the same cache.

A replaced `_get_text_metrics_with_cache` gets the installed renderer
(`text._layout_renderer`), or a stand-in. Under the PDF, PostScript and
SVG canvases, this is the print renderer (`_print_renderer`), which
measures as its backend does and draws nothing. Thus `fig._get_renderer()`
stays the Agg renderer.

A cache knows how its renderer measures (`pltrs::text::RendererMetrics`):
the `_text_metrics` of `RendererPdf`, `RendererPS`, `RendererSVG` and of the
vector renderer of a `MixedModeRenderer`. While such a cache is installed,
pltrs (Rust) lays out text as that backend does. That is unhinted, through
`TextToPath`, and in AFM metrics if `pdf.use14corefonts` or `ps.useafm`
asks. A measurement without a renderer on such a canvas uses the print
renderer and leaves the figure at 72 dpi (`text._measured_by_print`), like
matplotlib's `_get_renderer`.

matplotlib's `Text.draw` lays out a tick label again. While a cache is
installed, pltrs (Rust) looks the label lines up again at draw time
(`LaidOutText::measure_again`), without a second layout.

matplotlib's renderers measure lines at their own resolution, but place
texts at the figure resolution. Thus a renderer of another resolution
given to `get_window_extent(renderer)` or `get_tightbbox(renderer)` gives
lines at that resolution. `text._measured_by` installs its cache, and pltrs
(Rust) measures lines and `points_to_pixels` pads at its resolution
(`pltrs::artist::set_measured_at`). A `Text` keeps the renderer of its
last measurement until it is drawn, like `Text._renderer`.

`RendererAgg.draw_text` and `draw_tex` draw a line onto the canvas
(`_native.drawing.draw_text`, see "Drawing defined in Python"). Programs
can replace them:

* If a canvas renderer has a different `draw_tex` (on the instance, or
  `RendererBase.draw_tex`), its draws give pltrs (Rust) a
  `pltrs::text::TexPainter`. pltrs calls it for every TeX line, and draws
  its `draw_path` paths in place of the line.
* While `RendererAgg.draw_text` is `RendererBase.draw_text` (a `Watched`
  attribute), raster canvases draw text as paths
  (`pltrs_render::set_texts_as_paths`), as `_draw_text_as_path` does.

Tests also replace `dviread.PsfontsMap.__getitem__`, the TeX font lookup
of usetex. While it is replaced, the font map lookups of pltrs-text
(`pltrs_text::tex::set_font_map_lookup`) call the replacement.

### Drawing defined in Python

matplotlib draws a figure by calling `draw(renderer)` on every artist. Thus
program subclasses, program path effects (`draw_path(renderer, gc, tpath,
affine, rgbFace)`) and program renderers take part in a draw. pltrs (Rust)
draws figures itself and calls this Python code where matplotlib calls it
(`_python_drawing`). For figures without such code, it never calls Python.

* **Drawings.** The renderers that pltrs gives Python are matplotlib's
  classes with native drawing methods (`backends/_native_drawing`).
  `draw_path`, `draw_markers`, `draw_path_collection`, `draw_quad_mesh`,
  `draw_gouraud_triangles`, `draw_image`, `draw_text`, `draw_tex`, the
  groups and the filters each add a step to a `pltrs::artist::Drawing`
  (`_native.drawing`). `Artist.draw` adds the artist, drawn natively
  (`_draw_artist`). Outside a draw, pltrs draws the drawing at once.
* **Artists.** These artists get a painter that holds the object weakly
  (`pltrs::artist::ArtistPainter`, `_native.drawing.set_painter`):
  * an artist whose class takes `draw` from outside the package
    (`_program_classes.takes_from_program`),
  * an artist whose class computes a value that matplotlib's `draw` reads,
    such as `theta1` as a property. Package classes list these values in
    `_computed`, and `_push_computed` gives them to pltrs at draw time.
    pltrs reads them as often and in the same order as matplotlib
    (`Ellipse._recomputed`, `Arc._given_angles`),
  * an image whose class gives its pixels, `make_image`, `get_extent` or
    `get_transform`, drawn by matplotlib's `_ImageBase.draw`.

  After the layout, pltrs (Rust) calls `record_drawing` for each painted
  artist that the draw reaches, in drawing order. The figure can still
  change then. The class `draw` runs with the renderer recording. pltrs
  removes its rasterization wrapper and applies rasterization as
  `allow_rasterization` does. Then pltrs draws the recording in place of
  the artist. `super().draw(renderer)` draws the artist as pltrs draws it.

  An artist drawn from Python uses its given transform as it is at that
  moment (`evaluate_given_transform`). The other transforms are those of
  the laid-out figure (`_transform_trees.sync`). The renderer is the one
  that each format gives matplotlib's draw
  (`FigureCanvas*._program_renderer`). This is the `RendererAgg` of the
  canvas, or a `MixedModeRenderer` around `RendererPdf`, `RendererSVG`,
  `RendererPS` or `RendererPgf`.
  `FigureCanvasBase._program_drawing` installs it for the print
  (`drawing_with`). pltrs records its `start_rasterizing` too. A loose
  artist resolves its transforms in the figure that draws it. A program
  Axes class, such as the `WCSAxes` of astropy, gets a painter the same way
  (`ConstructionMixin._init_axes_state`).

  A class that measures its artists (`get_window_extent`, `get_tightbbox`)
  is measured only where matplotlib's engines and `bbox_inches="tight"`
  measure (`Figure::prepare_layout`). That is before a layout engine runs,
  before a tight box, and before the second pass of constrained layout
  (`FigureState::measure_painters_lent`), parents first. pltrs drops what
  the measuring draws (`measure_artist`).
* **Adding.** matplotlib's `add_*` methods call `set_figure`,
  `set_transform` and the `axes` setter, and its constructors call setters
  such as `set_data`. If a program class overrides them, pltrs calls them
  at the same places (`_program_classes.set_artist_props`). The artist gets
  the `zorder` of its class (`take_class_defaults`).
* **Path effects.** A program effect goes to pltrs (Rust) as the object
  (`("python", effect)`, `pltrs::patheffects::CustomPathEffect`). For every
  path drawn with it, pltrs calls `effect_draw_path`, which runs
  `draw_path` with the renderer recording. pltrs draws the recording in
  place of the path. The effects of an artist share one graphics context.
* **Renderers of the program.** If `Artist.draw` gets a renderer that pltrs
  did not make, pltrs (Rust) draws through its methods
  (`RendererBase._draw_artist`, `_native.drawing.draw_through`,
  `forward_drawing`). This also applies inside a `MixedModeRenderer`.
  pltrs lays out and draws a figure whole, rasterized artists through
  `start_rasterizing`.
* **Graphics contexts of the program.** The canvas renderer can have a
  program `new_gc` (`RendererBase.new_gc = ...`) that returns a
  `GraphicsContextBase` subclass. Then every artist on the raster canvas
  starts from a context that it makes (`pltrs::artist::GcSource`), as
  matplotlib's artists start from `renderer.new_gc()`.
  `RendererAgg._drawing` installs it with `_native.backends.set_program_gc`.
* **3-D projections of the program.** A 2-D patch or collection on 3-D
  axes whose class gives `do_3d_projection` (`Arrow3D`) is drawn through
  Python. Its projection runs first (`mplot3d._program_projection`,
  registered with `_python_drawing.before_drawing`). The returned depth
  orders it among the artists of the axes (`Axes3D::set_program_depth`).

matplotlib draws a print twice when an engine lays it out or when it is
cut to its tight box. pltrs then prepares the painters twice.

The painter archives as the object of the artist. Thus such a figure
pickles as in matplotlib, or fails with the pickle error.

### Projections of programs

`matplotlib.projections.register_projection` accepts any `Axes` subclass.
matplotlib promises such a class little. Its `__init__` calls `set_figure`,
`_set_lim_and_transforms`, `_gen_axes_spines`, `_init_axis` and `clear`,
and its draw calls `draw(renderer)` on every child. A library can override
any of these and add more. The reference case is the `TernaryAxes` of
mpltern. It has custom axes (`taxis`, `laxis`, `raxis`), spines, patch,
transforms of three coordinates, `clear`, `grid`, `tick_params`,
`callbacks` and title placement.

pltrs (Rust) holds an x and a y axis per Axes and nothing of an unknown
projection. One rule covers every case:

* pltrs (Rust) keeps what the native Axes has and asks the class about it
  through hooks.
* What the native Axes does not have stays matplotlib's Python object.
  pltrs (Rust) holds only its place: where it draws, where it is measured,
  and what clear, pickle and copy do to it.

| the class gives | what pltrs does |
|---|---|
| `set_figure`, `_set_lim_and_transforms` | Runs them where `__init__` runs them (`_program.set_up`). The trees are matplotlib's, so added transforms of any dimension (`transProjection`) compute as in matplotlib. |
| `transData`, `get_xaxis_transform`, `get_yaxis_transform`, the text transforms | The hooks of the native x and y axis (`_Adapter`, see "Transforms"). |
| an `XAxis`, `YAxis`, `XTick` subclass named `x` or `y` | The native axis, asked through the hooks (`tick_interval`, `tick_sides`, `tick_label`). |
| an axis with another name (`axis_name = 't'`, listed in `_axis_names`) | A *held axis*: matplotlib's `Axis` in Python (see below). |
| `_gen_axes_spines` with spines that name a side | The native spine of that side, with its interval from `_adjust_location`. While `_init_axis` runs, `ax.spines` already gives these spines, so `spines['tside'].register_axis` finds them. |
| spines with other names (`'tside'`), any path, any transform | Spines with no side, beside the frame, if the class makes several or also spines of the sides (`draw_class_spines`). If the class has its own `_adjust_location`, each draw runs it first and draws `_path` through `get_transform()` (`Spine._adjust_before_draws`). |
| `_gen_axes_patch` | The background outline and the clip. pltrs takes the patch again after a change (`patch.set_xy`). |
| `clear` (or `cla`) | Called where matplotlib's `__init__` and the program call it. `super().clear()` clears the native Axes and every held axis. Then each held axis that no other Axes shares gets `_set_lim(0, 1, auto=True)`. The clear reads `_share<name>` first, so it fails for a class that did not set it. A hidden axis stays hidden, because `Axis.clear` keeps its visibility. |
| `draw`, `get_tightbbox`, `_update_title_position`, `autoscale_view`, `get_data_ratio`, `tick_params`, `grid` | Called where matplotlib calls them (see "Drawing defined in Python", `_program_locators`, `_program_tick_params`). |
| `callbacks` as a property of the class | matplotlib's registry knows only `xlim_changed`, `ylim_changed` and `zlim_changed`, so a class with another axis keeps its own registry. pltrs (Rust) reads the `callbacks` attribute at each change. pltrs connects itself at a clear (`ClearMixin._hear_limit_changes`). |
| custom `_axis_names`, `_shared_axes` | `_axis_map`, `get_children`, `set_axisbelow`, the unit pipeline and `__repr__` go over `_axis_names`. Code that reaches a native axis by name first checks the name (`_AxesBase._native_axis_names`). `sharex` and `sharey` also join the class groups (`_join_class_group`), which `get_shared_x_axes()` and the held axes read. |
| a subclass of `GeoAxes`, `AitoffAxes`, `HammerAxes`, `MollweideAxes` or `LambertAxes` | A program projection (`_program.is_program_geo`): a rectilinear native Axes that matplotlib's `GeoAxes` code makes geographic (see below). |

matplotlib requires three things of a class with a third axis:

* `_share<name>` is set before `__init__`.
* The `callbacks` registry knows `<name>lim_changed`.
* `autoscale_view` takes `scale<name>`.

**A held axis** is an `Axis` whose name is not a native axis of its Axes
(`axis._names_native_axis`). It is matplotlib's `Axis`, ported unchanged
from matplotlib 3.11.2 (`_held_axis.py`), over objects of the package:

* **State.** Its locator and formatter compute natively, as for a stand-in
  axis. `TickHelper._axis_info` asks the axis for its view interval and
  tick space, and the class answers. The `_get_tick` of the class makes its
  `Tick` objects lazily (`majorTicks`, `minorTicks`, as `_LazyTickList`).
  Their lines and labels are ordinary artists with the transforms of their Axes
  (`Tick._take_axes_transforms`), which the tick class can replace. `label`
  and `offsetText` are `Text` objects, and the tick parameters are
  `_major_tick_kw` and `_minor_tick_kw`. The scale, units and converter
  are Python objects. The limits come from the `get_view_interval` and
  `set_view_interval` of the class. `_set_lim` is matplotlib's. It
  processes `<name>lim_changed`, follows the `_shared_axes` group and
  calls `autoscale_view`.
* **Dispatch.** A program axis class derives from the package's `XAxis` or
  `YAxis`. pltrs wraps each method and property of `Axis`, `XAxis` and
  `YAxis` once (`axis._serving_held`). `__init__` first marks a held axis
  `_held`. A call on a held axis goes to matplotlib's function
  (`_held_axis.function`), or else to the classes after `Axis`, such as
  `set_visible` and `get_zorder` of `Artist`. A call on a native axis goes
  to the package function. A property uses the
  `__dict__` of the axis (`_held_axis.read`, `write`), because matplotlib's
  properties are plain attributes. Unused private methods
  (`_copy_tick_props`, `_update_label_position`, `_init`) exist for
  `super()` calls. `_held_axis` loads with the first held axis.
* **Place in the Axes.** `Axis.__init__` asks pltrs (Rust) for a
  `ProgramAxis` (`Axes::add_program_axis`, `_native.axes.add_program_axis`).
  pltrs paints it through the `draw` of the axis and measures it through
  its `get_tightbbox` (`_python_drawing.draw_through_python`). The Rust Axes
  lists its program axes after x and y. It does these things for each:
  * draws it after the spines and before the titles, at its own z-order,
  * leaves it out with `axis("off")`, and out of every draw except a print
    if it is animated,
  * measures it as an axis only while the Axes shows its axes,
  * keeps it through `clear` and archives it with the figure.

  An axis whose name the Axes does not list is a plain artist that nothing
  draws.
* **Measuring.** Before a layout engine or a tight box measures, each
  painted artist with `_before_measured` makes itself ready
  (`_python_drawing.measure_artist`). An Axes with its own title placement
  applies its aspect and places its titles, and pltrs (Rust) measures the
  rest. A held axis has its Axes apply the aspect, then its
  `get_tightbbox` measures it. pltrs first updates the trees, so the second pass of an engine measures at the positions of the
  first. Engines and `Axes.get_tightbbox(for_layout_only=True)` pass
  `for_layout_only=True` to an Axes and its axes
  (`ArtistPainter::measure_for_layout`, `_python_drawing._tight_box`).
  pltrs omits the keyword if a method does not take it.
* **Figure.** After `ax.remove()`, a held axis and its ticks keep their
  figure.
* **Calls, and how often.** The `draw` of the axis runs once per draw of
  the Axes that reaches it. pltrs prepares painters only for drawn
  children, in drawing order (`AxesState::drawn_children`). Thus the
  spines run `_adjust_location` before the axes `draw`, and no axis code
  runs with the axes off. The `draw` is matplotlib's code. It runs
  `_update_ticks` (the locator `__call__`, `format_ticks`, each
  `update_position`), the label boxes, each tick `draw`,
  `_update_label_position`, `_update_offset_text_position`, the label and
  the offset text. `get_tightbbox` runs where an engine or a tight box
  measures, and where the title placement asks.
* **Tick parts.** Tick parts draw by their own `draw`, through their
  transform at draw time. The `_marker` of a line is its `MarkerStyle`
  (`_line_markers.LineMarker`), so `line._marker._transform = t` reaches
  the line. Some transforms have a program `get_matrix` (an
  `Affine2DBase` subclass). pltrs gives such a transform to pltrs (Rust)
  again wherever the text is measured (`_live_transforms.follow_given`).
  matplotlib also computes the matrix at each use.

The background, the clip, the children, the x and y axis, the titles, the
legends and the layout stay native. A held axis costs one call into Python
per draw and one per tick part.

Some library Axes classes read values in the `clear` that `__init__` runs.
pltrs gives them the tick locations of an Axes not yet added
(`Axis::ticklocs_as_is` of a removed Axes) and `_gridOn`.

**A subclass of a geographic class** (the `StereonetAxes` of mplstereonet)
is not drawn by the native geographic Axes. That Axes knows four
projections and no class changes. matplotlib's `GeoAxes` is a plain `Axes`
with a few methods, and the example `custom_projection` shows that the
hooks can draw such a class. Thus `projections/geo.py` keeps matplotlib's
code beside the native code. These methods choose by the class
(`GeoAxes._is_programs`):

* `__init__`, `_init_axis`, `_gen_axes_spines` and `clear`,
* `_get_affine_transform`, which asks for the core transform at resolution
  1 and maps two points,
* `set_longitude_grid`, `set_latitude_grid` and `set_longitude_grid_ends`.

pltrs calls each override where matplotlib calls it, with matplotlib's
arguments. The private attributes `_longitude_cap`, `_center_longitude` and
`_center_latitude` are plain attributes there. On the native Axes they read
what pltrs (Rust) holds (`geo._NativeSetting`).

The hooks of every projection class also do these things:

* A gridline path asks for matplotlib's 180 interpolation steps
  (`AxisPainter::gridline_path`).
* The data transform keeps a `TransformedPath` per line path
  (`_NonAffinePart`, `keeps_paths`). pltrs calls the
  `transform_path_non_affine` and `transform_non_affine` of the class once
  per line until the transform is invalidated.
* A label goes beyond the Axes on a side without a spine
  (`Projection::spines`, as `spines.get("bottom", self.axes)`).
* An Axes whose constructor adds other Axes joins the figure after them
  (`_constructed_native`).
* Pans and zooms map back through the inverse of the program tree
  (`Projection::axes_to_data`, `_Adapter.axes_to_data`). matplotlib inverts
  each affine part of `transData` separately, which changes the last bit.

pltrs does not do these things:

* pltrs (Rust) aligns only the x and y labels. matplotlib's `align_labels`
  also knows no other axis.
* A plain Axes that shares an axis with an Axes of a class with its own
  groups follows its limits. In matplotlib it does not. pltrs (Rust) has
  one group per shared axis.
* The x and y axis of an Axes that left its figure measure nothing.
  matplotlib's axes still measure.

`python/tests/test_proj1.py` and `test_proj2.py` check a three-axis
projection (`python/tests/curve_projection.py`) and a sinusoidal projection
on each geographic class against matplotlib values, with no library.
`cargo xtask diff --corpus mpltern pysmithchart mplstereonet pyrolite` runs
four library projections.

### Methods of matplotlib's classes that programs override

Libraries subclass matplotlib's classes and override what matplotlib's own
code calls on ``self``. ``spec/diff/overrides_census.json`` lists these
overrides for about 200 libraries and corpora.
``spec/diff/overrides_census.py`` makes it. pltrs (Rust) does this work
natively, so each such call is a dispatch. pltrs asks the Python object only
when its class takes the member from the program
(`._program_classes.takes_from_program`, cached per class and member).
Objects of package classes never pay. pltrs asks in these places:

* **Construction.** An Axes class that overrides ``__init__``, ``clear``,
  ``cla`` or ``grid`` (`.figure._CONSTRUCTION_CALLS`) is made by its
  constructor, with the positional arguments of ``add_subplot``
  (`.axes._construction.given_arguments`). A clear calls its ``grid``. A
  class that overrides ``_set_lim_and_transforms``, ``_gen_axes_spines``,
  ``_init_axis``, ``_gen_axes_patch`` or the axis transforms is a program
  projection, whatever its ``name``
  (`.projections._program._PROJECTION_HOOKS`). After unpickling, it keeps
  its spines.
* **Measuring.** An artist whose class measures itself
  (``get_window_extent``, ``get_tightbbox``) gets a painter for layouts and
  tight prints, unless it is clipped to its Axes. pltrs asks an Axes with
  ``bbox_extra_artists=None``.
* **Transforms** of the program map arrays in one call, through
  ``transform_non_affine`` and then the affine part
  (``_native.transforms.apply``).
* **Ticks.** A locator with its own ``tick_values`` ticks through it where
  matplotlib's ``__call__`` does.
* **Color mapping.** pltrs calls a program norm with its values alone,
  unless pltrs clips.
* **Scales** of the program keep their Python object natively
  (``PyScale``), and a figure pickle holds it. After unpickling, pltrs asks
  ``scale._program_scale`` for its functions again.
* **Adding.** pltrs calls an overridden ``add_*`` method (``add_line``,
  ``add_patch``, ``add_collection``, ``add_image``, ``_add_text``,
  ``add_table``) with each artist that the plotting methods make, with
  matplotlib's ``autolim`` (`.axes._program_adds`). The package method adds
  nothing more. An override that does not call it leaves the artist out.
* **Constructors' setters.** The `.Text` constructor calls ``set_text`` and
  each overridden setter of its explicit keywords, given or not. The
  `.Legend` constructor calls ``set_loc``. Other constructors call the
  overridden setters that matplotlib's constructors call, with matplotlib's
  arguments (`._program_classes.replay_setters`). Examples are
  ``Polygon.set_xy`` and the ``set_verts``, ``set_segments`` and
  ``set_transform`` of collections. pltrs finds these setters once per class
  (``Artist._program_setters``). The set is empty for package classes,
  identified by module name.
* **Texts.** A `.Text` class can answer the getters that matplotlib's draw
  and measuring read (``get_text``, ``get_rotation``, ``get_color``,
  ``get_wrap``, ``_get_wrapped_text``, ``get_unitless_position``). pltrs
  (Rust) holds the answers meanwhile (`.Text._as_class_reads`).
* **Plotting methods made of plotting methods.** Some matplotlib plotting
  methods call others:
  * ``errorbar`` calls ``vlines`` and ``hlines``.
  * ``bxp``, ``stem``, the spectra and ``ecdf`` call ``plot``. ``stem``
    also calls ``vlines``.
  * ``hist`` calls ``bar`` or ``fill``. ``grouped_bar`` calls ``bar``.
  * ``violin`` calls ``fill_between(x)``, ``vlines`` and ``hlines``.
    ``stackplot`` calls ``fill_between``.
  * ``pie`` and ``pie_label`` call ``text``. ``bar_label`` calls
    ``annotate``.

  If a class overrides an inner method
  (`._program_classes.takes_any_from_program`), the outer method runs
  matplotlib's code (the ``_*_reference`` modules of `.axes._plotting`).
* **Prints cut to a window** (a tight box or ``bbox_inches``). During the
  print, the figure has the window size (``get_size_inches``,
  ``bbox_inches``, ``bbox``), and ``transFigure`` moves by the window
  corner, as after ``adjust_bbox``.
* **Spines replaced.** After ``ax.spines['bottom'] = spine``, the ticks
  keep the transform of the old spine until they are made again.
* **Linear spines.** In a plain Axes, a spine class that overrides
  ``_adjust_location``, ``get_patch_transform``, ``get_spine_transform``,
  ``_ensure_position_is_set``, ``get_path``, ``get_transform`` or
  ``get_window_extent`` lays out the spine in Python. pltrs (Rust) draws the
  resulting path in display pixels (`Spine._push_layout`).
* **Picking** goes through the ``get_children`` of a class, as in
  ``Artist.pick``.
* **Prints.** Before a print that an engine lays out or that is cut to its
  tight box, matplotlib makes an extra draw. The ``draw`` of a figure class
  runs in it too. A program layout engine runs once per
  such print.
* **Affine transforms** with their own ``transform_affine``, ``transform``
  or ``transform_point`` map single points, for example to place a text.
  `._live_transforms` gives pltrs (Rust) such a transform as a callback
  map. Paths and compositions go through its matrix. pltrs reads
  ``get_matrix`` when it takes the transform and where a text is measured
  (`follow_given`), not at every draw.
* **Axis transforms** of a program projection (``get_xaxis_transform``)
  are taken again at each clear, not at each draw, because matplotlib's
  ticks keep theirs.
* **axes_grid1 sizes** of the program get the print renderer, or else the
  last canvas renderer. They pickle as their ``get_size``.

`python/tests/test_gen5.py` and `test_gen6.py` check each case. ``cargo
xtask diff --generated overrides`` runs the family
(``spec/diff/generators/overrides*.py``).

Libraries also replace methods and functions by assignment: on a class
(``Axes.scatter``), on one object (``axis._update_ticks``) or on a module
(``scale.scale_factory``). The census is ``spec/diff/patches_census.json``
(``patches_census.py``). ``cargo xtask diff --generated patches`` runs the
family.

* A replacement on a class is a class member, and `takes_from_program`
  reads it.
* pltrs sees a replacement on an object at assignment
  (`.Artist.__setattr__`). If an axis gets ``_update_ticks``, the program
  tick updater ticks it (`._tick_updates.follow_program_update`) where
  matplotlib calls the method (``UpdateSite.Matplotlib``). pltrs draws the
  ticks that it returns. Where pltrs needs ticks for other reasons, it
  computes them without the replacement.
* pltrs calls a module function where matplotlib calls it. A replaced
  ``scale_factory`` makes the scales of each new axis and each clear
  (`.axis.Axis`, `.axes._clear`).

`python/tests/test_gen7.py` checks these replacements.

### pyplot's state

pyplot keeps its figure registry in Python. `_pylab_helpers.Gcf` maps
numbers to managers, which hold canvases and `Figure` objects of any
`FigureClass`, with their Python attributes. pltrs keeps matplotlib's
module unchanged, because tests use it directly. pltrs (Python) does not
use the `pltrs::pyplot` registry. pltrs (Rust) keeps the current Axes of
a figure, and `Figure.gca` calls `_figure.gca`.

### Backends, canvases and printing

The backends are matplotlib's. `backends.backend_registry` names them.
`pyplot.switch_backend` loads `backends/backend_<name>.py` and builds
`new_figure_manager` and `show` from its `FigureCanvas` (the `_Backend`
export machinery). `pyplot.figure` makes managers through it.
`rcParams['backend']` starts as `rcsetup._auto_backend_sentinel` or as
`MPLBACKEND`. When pyplot first needs a backend, it tries these in
order:

1. the framework that already runs,
2. the pltrs windows (`macosx` on macOS, `pltrs` on other systems),
3. Qt, GTK, Tk and wx,
4. `agg`. On Linux without a display, every interactive backend refuses.

* **The pltrs windows** (`backends/backend_pltrs.py`, the `pltrs`
  backend, and `backend_macosx` on macOS) are a GUI toolkit backend over
  the windows of pltrs (Rust) (`_native.gui`, a
  `pltrs::gui::WindowHost`). The registry knows `pltrs` but does not
  list it among matplotlib's built-in names.

  A window shows the raster canvas of its figure above a toolbar. It
  reports input, resizes, repaints, toolbar clicks, close requests,
  chosen files and timers as tuples. `backends/_pltrs_loop.py` gives
  them to the canvas as matplotlib events, or to the timer or the file
  chooser. Navigation, keys and callbacks stay matplotlib's
  (`NavigationToolbar2`, or the tool manager). The subplot configuration
  tool is a pltrs dialog (`SubplotToolPltrs`), as in the Qt backend. It
  does the `"subplots"` events of the host with matplotlib calls
  (`subplots_adjust`, `tight_layout`, `ConstrainedLayoutEngine.set`).
  See the subplot tool of `pltrs::gui`.

  The loop blocks until something happens, so an idle window costs
  nothing. Input, the next timer, a call queued by another thread (see
  "Threads"), an idle draw, a stop request (`_pltrs_loop.wake`) and a
  signal wake it. For signals, the Python signal handler writes to the
  wake pipe of the host (`signal.set_wakeup_fd`), as matplotlib's Qt
  backend does. Thus Ctrl+C interrupts at once. Afterwards, pltrs puts
  back the descriptor of the program. The wait runs without the GIL
  (`binding::blocking`, through the wait hook of `pltrs_platform`). The
  IPython input hook and the Jupyter kernel loop wait in a thread. The
  framework counts as running when the host exists
  (`cbook._get_running_interactive_framework`). For tests, a headless
  host (`_pltrs_loop.connect_headless`) opens windows off screen.

  A program gets the same input events as from matplotlib's QtAgg
  windows, event for event. The code is the canvas input of
  `pltrs::gui`, shared with the `FigureWindow` of pltrs (Rust). For
  example, key names are Qt's, a motion that does not move sends no
  event, and a leave waits until the last button is released. Every
  event carries its `guiEvent` (`backend_pltrs.WindowEvent`). Two
  intended exceptions follow matplotlib's GTK and Tk backends: dead keys
  compose, and Shift+Tab reaches the canvas.

  `python/tests/test_gui_events.py` checks this on a private Xvfb, on
  X11 and on a nested weston. The scenarios of `python/tests/gui_events`
  feed input through XTEST. The test compares every `mpl_connect` event,
  the widget reports and the pixels with goldens of matplotlib under
  QtAgg. Record the goldens with `python -m gui_events --record`. From
  `python/tests`, `python -m gui_events` runs both side by side.
* **Tk** (`backends/_backend_tk.py`, `backend_tkagg.py`) is matplotlib's
  code. It blits through `_tkagg`, the pltrs version of matplotlib's C
  helper, which gives Tk the pixels as PNG data.
* **Qt, GTK 3 and 4, wx** are matplotlib's modules, changed only in
  their imports. Their canvases paint the pixels of the Agg canvas
  through the buffer of the `copy_from_bbox` region
  (`BufferRegion.__buffer__`). Without the toolkit, they raise
  matplotlib's `ImportError`. The Qt backends raise it after `qt_compat`
  selects a binding.

  pltrs has no Cairo. The `*cairo` backends draw with the raster in the
  Cairo graphics state (`backends/_cairo_canvas.py`, their mixin).
  `_native.backends.drawing_like_cairo` runs their draws in
  `pltrs::artist::GraphicsState::Cairo`, where lines with no join style
  get mitred joins, not round joins. In all other points they show the
  pixels of the `*agg` backends.

  wx is not thread-safe, so wx canvases call wx only from the main
  thread (`_FigureCanvasWxBase._on_main_thread`). A draw, blit,
  `draw_idle` or cursor change from another thread goes to the main loop
  (`wx.CallAfter`).
* **Cairo** (`backends/backend_cairo.py`) is matplotlib's module and
  needs pycairo or cairocffi. `RendererCairo` and `GraphicsContextCairo`
  draw onto a Cairo context of the program (see "Drawing defined in
  Python"). `FigureCanvasCairo`, the canvas of the `cairo` backend, is
  the Agg canvas in the Cairo graphics state. It prints PDF, PostScript
  and SVG with the pltrs (Rust) writers at 72 dpi, as matplotlib's Cairo
  surfaces do.
* **WebAgg** (`backend_webagg.py`) runs the web server of pltrs (Rust)
  (`_native.web`, a `pltrs::web::WebRelay`) in place of Tornado. It
  serves matplotlib's pages and files. An asyncio loop gives the
  WebSocket messages and downloads to matplotlib's figure managers
  (`backend_webagg_core`) and runs their timers. The URL prefix comes
  from `WebAggApplication.initialize` (`WebOptions::url_prefix`).

  Without Tornado, matplotlib raises
  `RuntimeError("The WebAgg backend requires Tornado.")`. pltrs works,
  and this difference is intended. Programs only select the module
  (`MPLBACKEND=webagg`) or subclass `backend_webagg_core`, and none
  relies on the error. `backend_webagg.tornado` gives the Tornado
  package if it is installed (`_lazy_imports.getter`). **nbAgg** is
  matplotlib's, over the kernel comms.
* **Jupyter's inline backend**: The IPython kernel selects
  `module://matplotlib_inline.backend_inline`, which imports
  `matplotlib`. That import gives pltrs if pltrs is loaded as
  `matplotlib`, or if its `matplotlib` distribution is installed. If
  not, the registry loads the pltrs port (`backend_inline.py`), which
  registers the kernel formatters for pltrs figures.

A canvas draws and prints through the native handle of the figure
(`_native.backends`):

* `FigureCanvasAgg.draw` draws onto the raster canvas that pltrs (Rust)
  keeps for each figure. `buffer_rgba`, `copy_from_bbox` and
  `restore_region` work on it. `RendererAgg` stands for it, and
  `Artist.draw` with it draws the artist onto that canvas.
  `Axes.draw_artist` and `Figure.draw_artist` blit this way
  (`axes/_blitting.py`, `_figure_interaction.py`).
* `FigureCanvasAgg` has no `blit`, as in matplotlib. Toolkit canvases
  blit with the widget method. The pltrs windows record the region. The
  `draw` of a vector canvas lays out the figure without output, as
  `draw_without_rendering` does.
* `Figure.savefig` calls `FigureCanvasBase.print_figure`. This resolves
  the keywords (format, dpi, colors, `bbox_inches`), changes to the
  canvas of the format or of `backend=`, and calls its `print_<format>`.
  The built-in print methods render with pltrs (Rust) (`_print_native`)
  and emit the `draw_event` that matplotlib's `Figure.draw` emits.
* The area to save travels as `bbox_inches_restore`, and `transparent`
  as the `_transparent` of the canvas. pltrs (Rust) refuses `metadata`
  for a format that cannot hold it, as matplotlib does.
* pltrs (Rust) encodes the raster formats that matplotlib writes with
  Pillow. GIF, AVIF and any `pil_kwargs` other than a JPEG `quality` are
  the exceptions, because matplotlib gets their bytes from Pillow or
  libavif. pltrs
  (Rust) draws their pixels (`_native.backends.print_rgba`), and
  `image.imsave` saves them, as matplotlib's `_print_pil` does.
* Canvas callbacks belong to the figure (`figure._canvas_callbacks`), so
  they stay when the canvas changes.
* The renderer that pltrs gives to Python is `RendererAgg`, also from
  `fig._get_renderer()` under the vector backends. Between draws, its
  drawing methods draw onto the raster canvas of the figure at once (see
  "Drawing defined in Python").

### rcParams

The `rc` module of pltrs (Rust) is the source of truth:

* `rcParams[key] = value` writes the value as rc-file text: `True`,
  `None`, numbers as Python prints them, sequences joined by commas, a
  `Cycler` as its repr. pltrs (Rust) validates it as matplotlib does.
* pltrs (Rust) gives floats, ints, bools, strings and lists of them as
  typed values. It gives other types, such as colors and cyclers, as rc
  text.
* Each `RcParams` other than `rcParams` (`rcParamsDefault`, copies,
  `RcParams({...})`) holds its own native `RcParams`, with only its
  keys.
* Python keeps `backend`, because Python code chooses the backend.
* Iteration leaves out the `pltrs.*` parameters, so code sees
  matplotlib's keys. You can still read and set them by name.
* `rc_context` saves and restores the current pltrs (Rust) parameters.

pltrs keeps matplotlib's `rcsetup`, with its validators and its table
`_params_list`. `RcParams._get` applies the validator to the rc text
that pltrs (Rust) holds. Thus `rcParams[key]` gives matplotlib's Python
value: a `Cycler`, a sketch as a tuple, a color as matplotlib keeps it.

pltrs (Rust) normalizes values, for example `--` for `'dashed'`. Thus
an `RcParams` remembers each given value with its rc text
(`RcParams._given`). While pltrs (Rust) still holds that text, `_get`
validates the given value, and matplotlib's spelling comes back. The
defaults start with matplotlib's spellings (`_default_spellings`), and
`rc_context` restores the remembered values. Assignment does not run
the validators. pltrs takes `cycler` from the `cycler` package.

The validator of the marker rcParams (`lines.marker`, `scatter.marker`,
`boxplot.*.marker`) takes any string, but the pltrs (Rust) type cannot
hold every string. Python keeps such a value and reads it back as given,
and pltrs (Rust) keeps its previous value (`RcParams._refused`).
matplotlib fails with the `MarkerStyle` error where an artist takes the
marker. `Line2D()` and `plot` without a marker check it with one set
lookup (`markers._check_rc`). matplotlib then cannot make Axes, because
ticks are such lines. pltrs still makes them.

The `style` package is matplotlib's, over the bundled sheets of pltrs
(Rust) (`_native.rcsetup.bundled_styles`). `style.library` maps names to
`RcParams`. User sheets come from `USER_LIBRARY_PATHS`
(`get_configdir()/stylelib`). `matplotlib._init_tests` adds
`_classic_test`. `RcParams._update_from` applies a library sheet by a
native copy of its rc text. pltrs (Rust) reads rc files
(`_native.rcsetup.read_file`) and reports the lines that matplotlib
logs.

### The configuration directory

pltrs (Rust) finds the configuration and cache directories
(`pltrs::config`) from `PLTRSCONFIGDIR`, or else the XDG directories.
pltrs (Python) also works unchanged in an environment set up for
matplotlib:

* If `PLTRSCONFIGDIR` is unset, it reads `MPLCONFIGDIR`.
* After `pltrsrc` at each place, it reads matplotlib's rc files:
  `matplotlibrc` in the working and configuration directories, and
  `MATPLOTLIBRC`.

`config::honour_matplotlib_config_dir` turns this on. `modules::register`
calls it before any code finds the directories. Thus a `matplotlibrc`
beside a program styles its figures under both. If pltrs cannot make or
write the directory, it uses a temporary one. The import then logs
matplotlib's warning with the variable that named the directory
(`_log_temporary_config_dir`). The suite runner sets `MPLCONFIGDIR` and
removes the `PLTRSCONFIGDIR` that cargo sets.

### The font catalogue

pltrs (Rust) keeps the catalogue of the system fonts in its cache
directory as `fontlist-v2.pltrs`, as matplotlib keeps
`fontlist-v390.json` (`pltrs::fonts::keep_catalogue`,
`pltrs_font::FontDatabase::add_system_fonts_catalogued`):

* The text engine reads it on first use, not at import.
* pltrs reads a font file again if it is new, or if its size or
  modification time changed.
* pltrs builds the catalogue again if it is missing, corrupt, or of
  another version or format.
* pltrs writes it atomically. If it cannot, it still uses the fonts, and
  the `font_manager` logger warns as matplotlib's `json_dump` does.
* Fonts from `fontManager.addfont` stay for the process.
* `ttflist`, `findfont` and the fallback chains are those of a process
  that read every font.

pltrs opens a font file on first use. It reads glyph programs (`glyf`,
the CFF `CharStrings`) glyph by glyph as it draws, and bitmaps and
variations only on request. Thus a large CJK font costs the memory of
the drawn glyphs, not of its file.

If `font.enable_last_resort` is off, a character can be in no font of
the chain. pltrs draws it as the missing glyph of the first font, but
does not measure it. As in `FT2Font.set_text`, the width, height and
descent of the line leave it out (`FontChain::layout`).

### Grids, layout engines, legends

pltrs (Rust) and its layout engines tell grids apart by identity. Thus
a `GridSpec` holds its native grid for its whole life, and a grid
indexed many times places all its axes on one native grid.
`gridspec._wrap` finds the Python object of a native grid that pltrs
(Rust) made. `fig.subplots` and `add_subplot(2, 2, 1)` reuse grids, as
`_check_gridspec_exists` does. The margins (`gs.left`, ...) live in the
native grid. `add_subplot` resolves its arguments with
`SubplotSpec._from_subplot_args` and adds the axes on the native block.
`subplot_mosaic` is matplotlib's and uses `add_subplot` on nested grids.

Layout engines (`layout_engine`) are matplotlib's classes. An engine
knows its figures. A figure gives pltrs (Rust) the description of its
engine (`_native_spec`) when the engine or a parameter changes. The
layout methods of figures (`tight_layout`, `align_*`, `suptitle`, the
super-labels, `get_tightbbox`) are the mixins of `_figure_layout`, over
`_native.figure_layout`. As in matplotlib, constrained layout leaves out
a `GridSpec` made without a figure.

pltrs (Rust) solves the grids of constrained layout.
`ConstrainedLayoutEngine.execute` returns them in its private
`layoutgrids`, as `_layoutgrid.LayoutGrid` mirrors that keep nothing
alive (`Figure::execute_layout`, `pltrs::layout::SolvedGrid`). Ticks
read between draws (`get_ticklocs`, `get_ticklabels`) are computed
without the layout engine, as in matplotlib (`Axes::ticks`).

pltrs (Rust) gets a description of the axes locators of axes_grid1,
insets and colorbars (`_place`) and places their axes at every layout.
Other locators are Python, which cannot run during the layout:

* a callable `locator(ax, renderer)`,
* a `Divider.set_locator`,
* a secondary axis located by a transform other than `transData` or
  `transAxes`.

The root figure keeps what they place (`axes/_program_locators`).
Before every layout of a draw, pltrs (Rust) runs the layout hook
(`Figure::set_layout_hook`) with no borrow of the figure held. The hook
calls the locators and places each axes where its locator says, until
the next draw.

Grids and engines of program classes (such as plotnine's):

* A grid that derives only from `GridSpecBase` gets its own native grid
  on first use.
* An overridden `get_subplot_params` gives the margins for
  `get_grid_positions`, `SubplotSpec.get_position` and the native block
  of a new axes.
* An overridden `get_grid_positions` places the blocks itself.
* An engine class with its own `execute` is a placeholder for pltrs
  (Rust), with its `adjust_compatible` and `colorbar_gridspec`. The
  layout hook runs its `execute(fig)` before the locators.
* An engine with a parameter that pltrs (Rust) cannot hold (a string, a
  list, a rectangle without four numbers, `_layout_checks`) is also a
  placeholder. Its `execute` raises matplotlib's errors at the same
  steps, before pltrs (Rust) lays out the figure. A `ValueError` leaves
  the layout unchanged, as in matplotlib.

`Legend(parent, handles, labels, **kwargs)` makes the native legend of
its parent at once. matplotlib's argument handling runs in Python
(`_parse_legend_args`, handler maps by handle and type). Each entry
reaches pltrs (Rust) as an artist handle, a tuple with its
`HandlerTuple` layout, or the native handle of a container. A loose line
or rectangle becomes a proxy. `_native.legend.candidates` lists what
`_get_legend_handles` lists.

`_legend_box`, the first child of the legend, is made of `offsetbox`
objects from the box of pltrs (Rust) (`_native.legend.legend_box`). The
box is laid out for the legend (`OffsetBox._laid_out_for`), and program
settings on it reach the legend. Its offset is the `_findoffset` of the
legend. `set_offset` pins it (`Legend::set_box_offset`), as
`DraggableLegend` does during a drag. Setting the location unpins it. A
figure legend (`Legend(fig, ...)`) is drawn when it is in `fig.legends`
(`_figure_artists.LegendList`), where `Figure.legend` puts it.

### Pickling

pltrs (Rust) holds everything drawn, and pickles must load in other
processes (`test_pickle_load_from_subprocess`). Thus a figure pickles as
an archive of its native state. The Python objects around it pickle as
usual. Do not record and replay the calls that built a figure. That
misses what pltrs (Rust) changes itself (layout, autoscaling) and
artists changed through their native objects.

* **The archive of pltrs (Rust)** (`pltrs_core::archive`): A type
  implements `Archive` (`encode`, `decode`) with `archive_struct!` or
  `archive_enum!` next to its definition. The macro lists the fields, so
  a new field does not compile until it is archived. Do not use a
  proc-macro derive, because proc macros need a host C toolchain, and
  the musl build avoids one.

  - A trait object archives its concrete type under a tag
    (`archive_kinds!(dyn Locator { "auto" => AutoLocator, ... })`, next
    to the trait).
  - An `Rc` reached twice reads back as one value.
  - A figure archives whole (`Figure::to_archive`,
    `RawHandle::archive_figure`), with removed artists under their ids.
    It leaves out canvas, callbacks and timers. Draggable artists
    connect again to the callbacks of the figure read back.
  - Reading never panics on malformed bytes.
  - The spec test `figure_archive` checks that every figure case read
    back draws the same pixels.
* **Externals**: The archive refers to what it cannot hold, and the
  program archives that beside the bytes. `ProgramKinds`
  (`modules/archive.rs`) archives binding types behind pltrs (Rust)
  traits as their Python objects, such as a Python `Formatter` or a
  `Norm` subclass. A closure archives as its origin (`ProgramFn`).
  `archive::tick_formatter`, `data_formatter` and `label_format` make
  the closures of Python callables and record the callable and how it
  is called. A Rust closure without an origin raises
  `TypeError: cannot pickle ...`, as a lambda does in matplotlib.
  Functions that pltrs makes itself are data.
* **The package** (`pltrs/_pickling.py`): Every native object pickles
  through `copyreg`.

  - An artist handle pickles as the `_FigureState` of its figure plus
    the artist id. A native value pickles as its own archive.
  - A `_FigureState` carries the figure archive. Unpickled, it is first
    an empty native figure that the handles can reach. Then
    `RawHandle::restore_figure` fills it in place. Thus the objects of a
    pickle can refer to each other in any order.
  - Artists pickle their `__dict__`.
  - `Figure` uses matplotlib's `__getstate__` and `__setstate__`: no
    canvas, and `_restore_to_pylab` puts it back into pyplot.
    `_pickling.restore_figure` makes the unpickled state.
    `_FigureState.when_restored` finishes it after the native state.
  - `copy.deepcopy` uses the same path.
* **Transforms of the trees**: The trees do not pickle. A tree transform
  that an artist got (`ax.transData` as its `_given_transform`) pickles
  as a reference to its owner (`_pickling.TreeReference`). On restore,
  the artist takes the rebuilt transform, so both are one object again,
  as in matplotlib.
* **Shallow copies**: `copy.copy` of a line, patch, text, collection or
  image gets its own native artist (`RawHandle::detached_copy`,
  `_artist_copying.detach_shallow_copy`). The copy is outside the tree,
  as a removed artist is. Thus a change to the copy leaves the original
  unchanged. Artists that hold others (Axes, legends) keep sharing them.
* **Mappables read back still follow their norm.** matplotlib pickles
  no callbacks, but an unpickled image reads its norm when it draws. In
  pltrs, the native artist holds the mapping. Thus
  `ColorizingArtist.__setstate__` registers the artist with its
  colorizer, and the colorizer with its norm, in
  `colors._follow_unheard` (weak, by identity). Both `_changed` methods
  tell them (`_tell_unheard`). matplotlib's unpickled colorbar has lost
  its callback and stays as drawn. Thus pltrs tells the artist inside
  `colorbar.leave_untold(True)`, so the pltrs (Rust) colorbars ignore
  that change.

### Imports inside the package

The suite and the `matplotlib` distribution load `python/pltrs` under the
name `matplotlib`. Thus:

* Modules of the package import each other relatively
  (`from . import _handles`, `from ..figure import Figure`), never as
  `pltrs`.
* The toolkits (`pltrs/mpl_toolkits`) and `pylab.py` load as top-level
  packages in the suite. They import pltrs by name
  (`from pltrs import colors`). `python/mpl_suite/pltrs_alias.py` and
  `_pltrs_matplotlib_alias` in the distribution resolve that name to the
  modules loaded as `matplotlib`.
* Import the native module as `from ._native import lines as _lines`.
  Its submodules are attributes.
* Importing pltrs loads the modules that matplotlib's import loads.
  Of the other pltrs modules, it loads only what the class definitions
  need. If a module uses a helper module only inside functions for rare
  cases, bind it with `_lazy_imports.module`. pltrs imports it on first
  use. `python/tests/test_perf7_import.py` holds
  matplotlib's lists and a ceiling on the count of pltrs modules.
  `benchmarks/import_cost.py` measures the cost of an import.
* At import, `get_data_path()` only finds and logs the data directory.
  pltrs writes the files when code next asks for the path, because
  writing them maps the extension pages around the bundled files.
* Code made from source text at import (the pyplot wrappers, the patch
  style constructors) is compiled once and kept with the kwdoc tables
  (`_docstring.compiled`).

### Distributions

`pyproject.toml` builds pltrs (Python) with the PEP 517 backend of the
tree (`python/backend/pltrs_backend`). It uses only the Python standard
library, so `pip install .` needs only cargo. It builds two
distributions:

* `pltrs`, the default, holds the release build of `pltrs-python` and
  the package.

  - The wheel is tagged `cp312-abi3-<platform>`. The backend reads the
    platform from the built library, never from the build Python
    (`_tags`). On Linux, it checks against the auditwheel policies. On
    macOS, it reads the oldest macOS of each slice. On Windows, it reads
    the architecture and `python3.dll`.
  - The backend checks the wheel as an index would (`_audit`).
  - `-C glibc=2.17` links with Zig against an old glibc (`_linker`), for
    manylinux2014.
  - `RELEASING.md` tells how to build, test and publish release wheels.
  - Linux wheels install the desktop entry and icons under the `share`
    directory of the prefix (`python/share`), because GNOME takes window
    icons from there. `crates/pltrs/examples/install_desktop_entry.rs`
    writes these files. `python/tests/test_desktop_entry.py` keeps them
    equal to what `backend_pltrs.install_desktop_entry` writes.
* `matplotlib` (`-C distribution=matplotlib`) is `python/alias`, in pure
  Python.

  - `_pltrs_matplotlib_alias`, a meta path finder, loads the pltrs
    sources under matplotlib's names, as `pltrs_alias.py` does. Thus
    pltrs objects name matplotlib's modules (`type(fig).__module__`,
    reprs, pickles), which libraries read.
  - The pltrs names (`pltrs.x`, which the toolkits import) are aliases
    of those modules. If a program imports pltrs first, it keeps the
    pltrs names, and matplotlib's names are the aliases.
  - The `pylab` module is matplotlib's own.
  - pip sees it as `matplotlib`, so it satisfies each requirement for
    matplotlib. It openly replaces a real matplotlib, and the reverse.

`cargo xtask ecosystem` runs pandas and seaborn through the alias
(`python/ecosystem`). It compares the structure of each figure with
`expected.json`. Record that file with `ECOSYSTEM_RECORD=1` under
matplotlib.

`cargo xtask ecosystem suite` runs the tests of each library that reach
matplotlib (`xtask/src/ecosystem/libraries.rs`). It works as
`mpl-suite` does, with the same outcome files and report:

* `ecosystem setup` downloads the sdists, or the release tag archives,
  into `target/refs/` and checks their SHA-256.
* `--reference` runs in `target/ecosystem/reference-venv`. The pltrs
  mode runs through the alias in `target/ecosystem/venv`, which has no
  matplotlib. A library whose pins clash gets
  `target/ecosystem/venvs/<library>`.
* The core metadata of the `matplotlib` distribution lies beside the
  alias, for libraries that read its version with `importlib.metadata`.
* Each library runs in a site laid out as its source tree, with its
  sdist `pyproject.toml`, so its pytest configuration applies.
* The outcomes are in `spec/ecosystem/<mode>.json`.

The libraries need more than matplotlib's suite does. Put this code
with the code of its kind. Examples:

* `sticky_edges` lists whose changes reach pltrs (Rust),
* `_get_lines` and `_get_patches_for_fill`, with shared property cycles
  (`axes/_prop_cycles.py`, `Axes::share_prop_cycle`),
* artists made outside a figure as legend handles
  (`LegendHandle::of_artist`),
* new ticks that copy the first tick, as `_copy_tick_props` does
  (`Axis::make_ticks`),
* a converter for each shared axis (`get_converter` returns
  `Axis::own_converter`),
* changes to `colors._colors_full_map` (`pltrs::colors::set_color_name`),
* `zoneinfo.ZoneInfo` with its `TZif` data, from the system or `tzdata`
  (`TimeZone::Info`),
* stand-in axes of tick helpers (`TickHelper._axis_info`).

The image comparisons need baselines of the same matplotlib.
`ecosystem setup` makes the baselines of plotnine, astropy and
mplfinance again under matplotlib 3.11.2, in the reference venv, from
the pinned versions alone (`xtask/src/ecosystem/baselines.rs`). They go
to `target/ecosystem/baselines`. Never commit them. The stored hashes of
astropy measure PNG bytes, so setup uses the pytest-mpl option
`--mpl-generate-path`. For plotnine and mplfinance, setup runs the tests
until no saved image is missing, as plotnine's
`tools/update_baseline_images.py` does. The suites compare with their
own tolerances (`--mpl --mpl-baseline-path` for astropy). These
comparisons need, for example, `TickStyle::label_across`,
`AnchoredOffsetbox::set_child_offset` (a child placed where the
program's `get_offset` says), `DrawnArtist` and
`axes/_program_tick_params.py`.

`ecosystem setup` also fetches test data that the sdists lack into
`target/ecosystem/data/<library>` and checks its SHA-256
(`Library::data`, `xtask/src/ecosystem/data.rs`). The tests find it by
a variable (`CARTOPY_DATA_DIR`) or a link.

The runs are offline (`Network::Refused`). A proxy refuses every HTTP
and HTTPS connection, so a test that fetches more fails in both modes
alike. `python/ecosystem/plugin/ecosystem_order.py` sorts the tests of
each file by id, because pytest-xdist refuses an order that changes
between processes.

These libraries also need projections and artists from the program.
Examples:

* a spine of no side (`SpineArtist::of_no_side`),
* the `autoscale_view` of an Axes class, run when its data limits
  change (`_program_locators.py`),
* transform-givers (`_as_mpl_transform`) and projection objects
  (`_as_mpl_axes`),
* private registers: the `_axes` of an artist (`_artist_private.py`)
  and the `_axstack` of a figure (`_axes_stack.py`),
* arrays changed in place (`_own_state`, such as
  `Poly3DCollection._facecolor3d`),
* `MaxNLocator._raw_ticks`, where a subclass changes its steps.

### Type information

pltrs has matplotlib's types, so a type checker reads a matplotlib
program the same way under pltrs. `py.typed` and matplotlib's stubs lie
beside the modules they describe. `python/tools/matplotlib_stubs.py`
writes them from the matplotlib source tree. Never write them by hand.

matplotlib's stubs import absolutely
(`from matplotlib.text import Text`). Type checkers read absolute and
relative imports differently. For example, pyright reads
`from . import x` in a stub as a re-export. Thus each distribution has
its own copy:

* `python/pltrs/**/*.pyi` import the pltrs modules.
* `python/alias/matplotlib/**/*.pyi` are matplotlib's stubs as it
  installs them.

The alias also installs these files, which Python never imports:

* stubs of matplotlib's modules without one (`units`, `dates`, the
  backends). They are matplotlib's code without function bodies, but
  keep what methods assign on their instance.
* the pltrs module beside each stub, under matplotlib's name. Checkers
  look for the source of a stub, and pyright warns without one.
* the pltrs toolkit modules beside the toolkit loaders, whose
  `__init__.pyi` are matplotlib's code (`Project.checked_sources`).

The pltrs toolkits (`pltrs.mpl_toolkits`) have no stubs. `py.typed`
covers them, because PEP 561 has no marker for an untyped part. pyright
reads their code, as for matplotlib. mypy reads their code too. For the
matplotlib toolkits, mypy uses `Any` and reports `import-untyped`. Do not
add stubs with only a module `__getattr__`. They silence pyright, which
is a larger difference.

matplotlib keeps some instance attributes as plain attributes
(`Wedge.r`, `ContourSet.zmin`). If pltrs (Rust) holds one, it is a
property whose setter has matplotlib's effect:

* If matplotlib's draw reads it, the setter gives it to pltrs (Rust)
  (`RegularPolygon.xy`, `Arc.theta1`, `AxesImage.origin`, `axison`).
* If only a later method reads it, or nothing, the setter keeps it on
  the object (`_values.assign`).

A patch assigned to a part that pltrs (Rust) draws (`legendPatch`,
`arrow_patch`) gives the part its look and becomes its object
(`_handles.adopt`, `_annotation_arrow.adopt`). An assigned `None` is
kept as the problem that the next draw fails on (`_unusable`).

The pltrs `pyplot.py` makes its wrappers from source text, so it has no
inline types. Thus `pyplot.pyi` is matplotlib's module without
docstrings and function bodies. Its `__all__` lists every public name,
because mypy exports the imports of a stub only through `__all__`.

`cargo xtask diff --types` (`spec/diff/stubs.py`) checks the stubs
against pltrs at run time with `mypy.stubtest`, as matplotlib's CI does.
It uses matplotlib's allowlist and the lines its `tools/stubtest.py`
derives. Each error that the matplotlib run does not report is a
difference in the public names of pltrs. A native attribute that the
stubs declare is a property with a setter, or a descriptor that steps
aside (`_transform_trees.TreeAttribute`, `_offsetbox_trees.TreeSetting`).

### Jupyter

`cargo xtask notebooks` runs the notebooks of `spec/notebooks` in
Jupyter kernels (nbclient, ipykernel). It runs them once with real
matplotlib and once with pltrs as matplotlib (`python/alias`). It
compares every output and the comm messages of ipympl and nbAgg
(`xtask/src/notebooks`). Images must stay within matplotlib's RMS
tolerance. The driver (`execute.py`) acts as the browser and sends the
messages that the cell metadata lists. The module names and the `pltrs`
backend in backend lists differ by design.

`cargo xtask notebooks suite` runs the matplotlib tests of IPython,
matplotlib-inline and ipympl, and matplotlib's notebook tests, under
both.

The code that Jupyter relies on lives where matplotlib keeps it:

* pyplot connects to the IPython `post_execute` event
  (`install_repl_displayhook`, from `switch_backend`, `ion` and `ioff`).
  It draws stale figures again after every cell and comm message.
* The Axes observers of a figure hear of every Axes added.
* pltrs keeps matplotlib's staleness side effects that the widget
  backends turn into messages. For example, vector canvases leave their
  figure at 72 dpi after a draw.
* A Jupyter kernel runs the pltrs windows between its messages
  (`_pltrs_loop._kernel_loop`, `%matplotlib pltrs`).

### Sphinx

The Sphinx extensions in `sphinxext` (the plot directive, `mathmpl`,
`figure-mpl`, the roles) are matplotlib's modules, changed only in their
imports. Keep every other line equal to matplotlib's, because tracebacks
in build warnings show these lines. The extensions use only the public
API of pltrs. Thus a difference in a documentation build is a difference
in pltrs. Fix it in pltrs.

`python/tests/test_sphx1.py` builds `python/tests/sphinx_project` under
matplotlib and under pltrs as matplotlib. The project uses every option
and configuration value of the extensions. The test compares the
warnings, files, HTML and pictures (`sphinx_builds.py`). matplotlib's
`test_sphinxext.py` runs in the suite.

The plot directive resets the rcParams with `rc_file_defaults` before
each plot, which gives the parameters of the look (`rcParamsOrig`).
Thus a documentation build draws in the modern look, unless
`PLTRS_LOOK=matplotlib` is set. sphinx-gallery resets with
`rcdefaults`, so it draws in the matplotlib look.

### Apple silicon

pltrs gives the results that numpy and matplotlib give on the same
machine, not the same results on every machine. On arm64 macOS, the
reference is the PyPI wheels: matplotlib 3.11.2
(`matplotlib-3.11.2-cp314-cp314-macosx_11_0_arm64.whl`, SHA-256
`1a3040b2…5e2863`), contourpy 1.4.0 (`…072a702e…f3c23d`), numpy 2.5.4
and Pillow 12.3.0. They differ from the x86-64 wheels in four ways.
pltrs follows these differences on arm64 macOS only. A macOS x86-64
build follows x86-64, because Rosetta runs it with the x86-64 wheels.

* **Fused products.** clang compiled the wheels. Its default
  (`-ffp-contract=on`) contracts a product and the sum or difference that
  uses it into one fused multiply-add, if both are in one expression.
  GCC's x86-64 wheels round the product first. The last bit differs, and
  it can decide a triangulation, a side of an edge or collinearity.

  `pltrs_core::numeric::FUSES_PRODUCTS` is true for
  `cfg(all(target_os = "macos", target_arch = "aarch64"))`. Then
  `numeric::fused(a, b, c)` is `a.mul_add(b, c)`, else `a * b + c`.
  `sum_of_products`, `difference_of_products` and `fused_f32` use it.
  Write each contracted expression of a port through these functions, in
  clang's operand order. The rule is syntactic. clang fuses the left
  operand of a sum or difference if it is a product, else the right one:
  - `a * b + c * d` is `fused(a, b, c * d)`
  - `c - a * b` is `fused(-a, b, c)`
  - `x += a * b` is `x = fused(a, b, x)`

  clang does not fuse a product that is kept in a variable or converted.
  It also does not fuse a product from an overloaded operator, such as
  the `XY` arithmetic of `_tri` in `TriContourGenerator::interp`. A
  product by a power of two is exact. pltrs fuses in these places:
  - qhull's geometry for the Delaunay triangulation (`tri/delaunay`),
    `_tri` (`XY::cross_z`, the plane coefficients) and the crossing
    interpolation of contourpy.
  - Agg and matplotlib's path code (`path` in `pltrs-core`). This
    includes the affine transform (`Affine2D::agg_transform_point`,
    `agg_then`, `agg_inverted`), curves, the stroker, simplification,
    sketching, clipping and Gouraud triangles. matplotlib's Python
    transforms use numpy and are not fused.
  - the image resampler in `pltrs-raster`
  - numpy's fill of `arange` and its `interp` (`pltrs::numeric`)
  - HarfBuzz's interpolation of `trak` tracking. Nothing else in
    `ft2font` changes a value.
* **Accelerate.** numpy's wheel links Apple's BLAS, which rounds small
  products in its own way (`numeric::ACCELERATE_BLAS`). A matrix product
  takes four columns at a time, fuses the products of each row in pairs,
  and fuses the rest in order. A matrix-vector product and a dot product
  sum rounded products, four of them in pairs. `Affine2D::then` (3×3)
  and `mplot3d::math` (4×4) follow this rule. pltrs does not follow
  larger shapes or LAPACK's inverse.
* **Sorting.** numpy has a SIMD `argsort` only on x86. Elsewhere, equal
  values come out in the order of numpy's introsort
  (`numeric::argsort::generic`).
* **Conversions.** On arm64, a float that does not fit in an integer
  saturates. On x86-64, it gives the smallest integer (`numeric::np_int`).
  `jpeg::decode::idct` in `pltrs-codec` narrows the sums of corrupt JPEG
  blocks as the Neon routine of libjpeg-turbo does.

`python/tools/fused_sites.py` finds the fused expressions again for a
new release. Run it on an arm64 Mac with Xcode's tools:

* `fused_sites.py wheel FILE.whl` shows the wheel and its SHA-256, and
  counts the fused instructions of each function. For 3.11.2: `_qhull`
  285, `_tri` 22, `_path` 235, `_image` 339, `_backend_agg` 353,
  `ft2font` 145, contourpy 64. A count includes inlined code.
* `fused_sites.py sources ...` compiles the sources with `-O3 -g` and
  prints the function, line and expression of each fused instruction.

Ignore fused instructions in code that only prints, in code for another
dimension and in code behind options that matplotlib does not set.
pltrs follows the PyPI wheels, not conda-forge, Homebrew or source
builds.

The goldens of the spec are matplotlib's results on Linux x86-64. Most
results of matplotlib on the Mac stay within the tolerance of pltrs'
check (values within 1e-9, images within an RMS of 1). Where they do
not, `spec/goldens/platforms/aarch64-apple-darwin` holds the platform's
goldens. See `CONTRIBUTING.md` and `spec/reference/spec_support/golden.py`.
Other files for this platform:

* The bit-exact fixture tests of `pltrs-core` and `pltrs-raster` have
  fixtures in `tests/fixtures/aarch64-apple-darwin`. The same
  `generate.py`, run on the Mac, writes them.
* Smoke tests that pin such a value hold both values
  (`python/tests/test_platform_arithmetic.py`).
* The trapezoid map shuffles its edges in libc++'s order
  (`tri/finder/shuffle.rs`).

`spec/diff/allowlist.txt` lists what pltrs does not follow, with the
proof:

* Agg's rasterizer reaches its cell limit on saturated coordinates.
* numpy cannot allocate what a saturated date asks for.
* The sign of rounding noise differs in numpy's arm64 pocketfft.

### aarch64 Linux

Docker on an Apple-silicon Mac, Graviton and the Raspberry Pi run
aarch64 Linux. Its reference is the `manylinux_2_28_aarch64` wheels of
matplotlib 3.11.2, contourpy 1.4.0 and numpy 2.5.4. GCC 14.2.1, the
compiler of the `quay.io/pypa/manylinux_2_28_aarch64` image, built them.
matplotlib has no musl aarch64 wheel. A source build there also uses
GCC, so musl follows the same rules.

GCC contracts with other rules than clang. Thus pltrs names the
contraction per platform: `pltrs_core::numeric::Contraction` (`Unfused`,
`Clang`, `Gcc`, the constant `CONTRACTION`). `FUSES_PRODUCTS` is
`CONTRACTION.fuses()`. GCC differs from clang in these ways:

* **Across statements and inlined operators.** GCC's default for C++ and
  GNU C (`-ffp-contract=fast`) fuses wherever a product meets its sum
  after inlining.
  - `_path.affine_transform` keeps its products in variables
    (`t0 = sx * x; t1 = shx * y; t = t0 + t1 + tx`). GCC fuses the first.
    Thus `Affine2D::transform_point` follows `CONTRACTION`
    (`transform_point_as`). Under GCC it equals Agg's
    `trans_affine::transform` (`agg_transform_point`).
  - GCC fuses the `XY` arithmetic of `_tri`: the tricontour crossing
    (`tri::contour::interp_point`) and the trapezoid map's box
    (`tri::finder::expanded`).
* **Another operand.** GCC's `widening_mul` pass chooses the fused
  product, and the choice depends on the function that the code is
  inlined into:
  - In `Triangulation::calculate_plane_coefficients`, the x and y
    components of the cross product fuse their second product, and z its
    first. The out-of-line `XYZ::cross` fuses each first product
    (`tri::triangulation::plane_as`).
  - Agg's `calc_intersection` fuses the second product of its numerator
    where `_draw_gouraud_triangle` dilates a triangle
    (`outline::calc_intersection_fusing`, `render::gouraud` in
    `pltrs-raster`).

  `numeric::FusedProduct` names the product that a port fuses
  (`Contraction::difference_of_products`).
* **Not in ISO C.** GCC does not contract ISO C, and numpy's C is C11.
  Thus numpy's `arange` and `interp` are fused on arm64 macOS only
  (`numeric::fused_iso_c`). On every aarch64 system, numpy fuses in its
  SIMD kernels (`tanh`, float32 `sin` and `cos`, complex `multiply`,
  einsum). Complex products use `mlab::Complex::mul_at`.
* **The same as clang.** qhull, the contourpy interpolation,
  `XY::cross_z`, the dot product of the planes and Agg's
  `trans_affine::transform` fuse as under clang.

`python/tools/fused_sites.py gimple SOURCE...` finds these sites. Run it
in the image above. It compiles matplotlib's sources with the wheels'
flags (`-O3`, `-fvisibility=hidden`). It prints GCC's `.FMA`, `.FMS`,
`.FNMA` and `.FNMS` statements per line and function, with the debug
binds that name their operands. An exact emulation (`Fraction`, one
rounding per fused operation) confirms each reading against the wheel.
The tests of each site check the patterns of all three compilers
(`Contraction::fused`, `plane_as`, `transform_point_as`, `interp_point`,
`expanded`).

The arm64 casts (`numeric::np_int`) and the Neon IDCT also apply here.
Accelerate does not, because numpy links OpenBLAS here. The goldens are
in `spec/goldens/platforms/aarch64-unknown-linux-gnu/` (`PLATFORM` in
`pltrs-spec`). The fixtures of `pltrs-core` and `pltrs-raster` are in
`tests/fixtures/aarch64-unknown-linux-gnu/`. pltrs does not follow these
differences:

* Gouraud shading. The colors differ for a cause that is not a fused
  product. A rebuild with `-ffp-contract=off` and `-fsigned-char` still
  differs. `agg_reference` has a tolerance for `gouraud_clip_path`.
* the extrema of curves (`np.roots`, LAPACK) and OpenBLAS's kernels
* probably, the complex products of numpy's FFT. One value of
  `mlab.csd` in the `number_types` sweep is one ulp off.

The release workflow's containers compare the drawings of
`python/tools/drop_in.py` with matplotlib's here, pixel for pixel
(`check_dist.py --compare-drawings`).

### Pythons and dependency versions

pltrs gives matplotlib's results on CPython 3.12 to 3.14, with
dependency versions from the lower bounds in `pyproject.toml` to the
current releases. The release workflow runs the smoke tests on them.

Some values come from the interpreter or a dependency, for example
Python's error messages, numpy's reprs, promotion and warnings, and
python-dateutil's zone files. pltrs gives what matplotlib gives with
that version. Smoke tests do not pin such a value. They get it from
`python/tests/build_facts.py`, which asks the environment.

At import, pltrs (Python) tells pltrs (Rust) what numpy does
(`pltrs::warnings::set_products_warn`: whether `np.dot` and `@` warn).
pltrs (Rust) asks python-dateutil for its zones through Python
(`_date_locators._dateutil_keeps_offsets`), because version 2.7 rounds
offsets to whole minutes.

### Processor levels

numpy's x86-64 wheels hold three copies of their hot loops: `X86_V2`
(the baseline, SSE4.2), `X86_V3` (AVX2 with FMA) and `X86_V4`
(AVX-512). At import, numpy picks the best copy the processor runs. The
copies give different results. Thus pltrs computes at numpy's level,
`pltrs_core::numeric::CpuLevel` (`Baseline`, `V3`, `V4`), resolved once
per process.

* **Where the level comes from: the numpy that is installed decides.**
  Before pltrs computes anything, `pltrs/__init__.py` tells
  `_native.numeric.follow_numpy` whether numpy runs its `X86_V3` and
  `X86_V4` copies. numpy runs a copy if two things are true:
  - The processor has the group (`__cpu_features__`, which
    `NPY_DISABLE_CPU_FEATURES` empties).
  - numpy holds a copy (`__cpu_baseline__`, `__cpu_dispatch__`).

  numpy before 2.4 has no groups, and pltrs reads the features `AVX2`
  and `AVX512_SKX` instead. numpy's Windows wheel has no `X86_V4` copy.

  pltrs (Rust) alone asks the processor (`is_x86_feature_detected!`). If
  `PLTRS_CPU_LEVEL` names a level (`v2`, `v3`, `v4`), it uses that level.
  Other architectures are `Baseline`. A kernel reads the level once per
  call. For tests, `argsort_at` takes the level as an argument.
* **What follows it: the sort.** At the baseline, numpy's unstable
  `argsort` uses numpy's introsort. At `X86_V3` and `X86_V4`, it uses
  Intel's x86-simd-sort for registers of 4 or 8 keys.
  `pltrs::numeric::argsort` ports both widths. Above 256 values, they
  order equal values differently. This changes the drawing order of 3-D
  points and faces of equal depth, the labels a contour keeps, and other
  results sorted by `argsort`. `crates/pltrs/tools/argsort_digests.py`
  holds the proof against numpy. With `--check`, it checks a processor's
  level with numpy alone.
* **The sort by numpy's release: numpy sorts the long arrays itself.**
  The order of equal values also changed in numpy 2.0 and 2.3. numpy
  1.25 and 1.26 have no AVX2 sort. pltrs (Rust) replays numpy 2.3 to
  2.5. All differences between releases are
  in arrays of more than 64 values. Thus pltrs (Python) lets numpy sort
  these arrays. `pltrs/__init__` registers `numpy.ndarray.argsort` as
  `"argsort"`, and `pltrs::numeric::argsort` calls it for arrays longer
  than `LONGEST_REPLAYED` (64, in `argsort/numpy.rs`). This is also
  faster. The call goes to numpy's C method and runs no Python code.
  numpy releases the GIL while it sorts, as under matplotlib
  (`python/tests/test_threads.py`).

  pltrs replays shorter arrays, because the replay costs less than the
  call. At the first short array with equal values, numpy sorts four
  seeded arrays, once per process:
  1. If the replay at the processor's level gives numpy's order, pltrs
     continues with it.
  2. If numpy's introsort gives numpy's order (numpy 1 with AVX2), pltrs
     replays the introsort.
  3. If neither does, numpy also sorts the short arrays.

  Arrays with NaN go to the C++ library's `std::sort`, so pltrs tests
  and routes them separately. `_native.numeric.argsort_routes()` shows
  the decisions. pltrs (Rust) alone orders equal values as numpy 2.5
  does, which recorded the goldens.
* **What does not follow it: the maths.** At `X86_V4` on Linux, numpy
  uses Intel's SVML for `exp`, `log`, `log2`, `log10`, `exp2`, `expm1`,
  `log1p`, `power`, `cbrt`, `tan` and the inverse trigonometric and
  hyperbolic functions. Up to 71% of the results differ from the C
  library's in the last bit. pltrs computes with the C library at every
  level, as numpy does at the other levels and on other platforms. This
  is within the tolerance of the spec (1e-9) and the differential tester
  (1e-6). The SVML float64 kernels (`__svml_*8_ha`) have an "INTEL
  CONFIDENTIAL" notice. Do not port any part of them.
* **Where one bit decides: numpy's own values.** One bit matters where a
  rounding feeds a discrete choice. This is the case for the power to
  which an `AsinhLocator` rounds a tick and for the boundaries of a
  `LogNorm` colorbar. There, pltrs (Rust) gets the values through
  `pltrs_core::numeric::Ufunc::map`. pltrs (Python) puts numpy's ufuncs
  behind it at import (`_native.numeric.use_ufuncs`). Thus pltrs makes
  matplotlib's choice whatever numpy runs. `AsinhLocator.tick_values`
  and `LogNorm.inverse` call the same ufuncs as matplotlib, as often.
  pltrs (Rust) alone uses the C library, so the spec waives the three
  `AsinhLocator` probes for it at `X86_V4`.
* **numpy's BLAS follows the processor too, and is not followed.**
  OpenBLAS picks its kernels by the processor (`SkylakeX`, `Haswell`).
  This changes the last bits of long dot products, large matrix products
  and LAPACK's results, but not the 3×3 and 4×4 products that pltrs
  replays. The spec waives one probe at the lower levels. A reference run
  held to a level must also set `OPENBLAS_CORETYPE`.
* **The baseline.** Without AVX2, numpy's complex multiply rounds both
  products. With AVX2 it fuses one. `pltrs::mlab::Complex::mul_at`
  follows both (`psd`, `csd`, `cohere`). float32 `exp`, `log`, `sin`
  and `cos` come from the C library.
* **aarch64.** numpy's aarch64 wheel dispatches from ASIMD to `ASIMDHP`,
  `ASIMDDP`, `ASIMDFHM` and `SVE`, but all copies give the same values.
  Its `argsort` is the generic introsort, so pltrs computes at
  `Baseline`. pltrs does not follow OpenBLAS's kernels there.

The goldens of the spec are recorded at `X86_V4`. Where matplotlib at a
lower level fails pltrs' check, `spec/goldens/platforms/x86_64-v3` or
`x86_64-v2` holds the golden (see CONTRIBUTING.md). The spec looks these up
after a target's goldens and before the common goldens. On an AVX-512
machine, `cargo xtask spec --level v3` and `python/tests/test_cpu1.py`
run the lower levels.

### Windows

matplotlib's `win_amd64` wheel is built with MSVC against the UCRT.
pltrs computes as this wheel does with both toolchains. An MSVC build
calls the UCRT's maths library. A GNU build links it too, because
MinGW's `sin` and `pow` differ from numpy's in the last bit. The build
script of `pltrs-core` writes the import library of
`api-ms-win-crt-math-l1-1-0.dll`.

C's `long` has 32 bits on Windows. This affects FreeType's sizes
(`Size::checked`) and HarfBuzz's feature values (`Feature::from_str`).
The wheel's `_tri` was built with MSVC 14.51. Its `std::shuffle` gives
the order of the trapezoid map (`shuffle_msvc`, checked against the
wheel).

numpy's Windows wheel has no `X86_V4` copy. Thus pltrs computes at
`X86_V3` at most (`CpuLevel::detect`). x86-simd-sort falls back to
`std::sort` for arrays with NaN and for partitions that stall. On
Windows this is MSVC's STL, and `numeric::argsort` sorts as it does
(`argsort/msvc.rs`). The digests of numpy's Windows wheel
(`digests-windows.txt`) check pltrs (Rust) at every length. In pltrs
(Python), this sort applies only to replayed arrays (64 values or fewer)
with NaN.

The goldens for Windows are in
`spec/goldens/platforms/x86_64-pc-windows-msvc`. Both builds on Windows
read them. The spec compares Ghostscript's console program and its
arguments as `gs` (`ghostscript` in `pltrs-spec`). The reference run
writes dates in the C locale (`%c`), not in matplotlib's en_US.UTF-8.

pltrs (Python) reads `SOURCE_DATE_EPOCH` with Python's `datetime`, as
matplotlib's writers do. On Windows, the UCRT's `gmtime` refuses a time
before 1970 with `OSError`. pltrs (Rust) writes such a date if it gets
one directly.

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

   Arguments convert through `FromPy`: `Text` through `artist_handle!`,
   `f64` as `float()` does. Results convert through `IntoPy`. Errors of
   pltrs (Rust) convert with `?` through the module's `From` impls. If
   pltrs (Rust) lacks what matplotlib does, add it there first. If
   outputs can change, run the output identity check.
2. **Python** (`python/pltrs/text.py`): write the method with
   matplotlib's signature and docstring. The signature handling stays
   here.

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

   If plotting methods take the property as a keyword, add it to the
   module's keyword table (`lines::LineKwargs` for lines). If matplotlib
   has an alias, add it to `define_aliases`.
3. **Check**: run `cargo xtask python test`, then the test files
   concerned, for example `cargo xtask mpl-suite test_text`.

A new matplotlib *module* is a new `modules/<name>.rs`, its line in
`modules/mod.rs`, and a new `python/pltrs/<name>.py`.

## Running the suite and reading its results

```sh
cargo xtask python setup                 # once: .venv-pltrs, and pytest-xdist in .venv
cargo xtask mpl-suite --reference        # real matplotlib (rarely needed)
cargo xtask mpl-suite                    # pltrs: builds the extension (release), runs, reports
cargo xtask mpl-suite test_colors        # only the files whose path contains a pattern
cargo xtask mpl-suite --failures         # also every failing test, with its reason
cargo xtask mpl-suite --imports          # every missing import of each test file
```

* **Where it runs.** `target/mpl-suite/<mode>/site/` holds the package
  under test in the layout of matplotlib's source tree, with the sdist's
  test packages (`baseline_images`, data) copied in. `site/matplotlib/`
  links the package's entries (installed matplotlib, or `python/pltrs`)
  and has a real `tests/`. The toolkits have the same layout. Test ids
  read `matplotlib/tests/test_x.py::test` in both modes. The reference
  venv stays unchanged. Its wheel has no baseline images, so the site
  also gives it the sdist's test data for `test_getattr.py`. In pltrs
  mode, `sitecustomize.py` (`pltrs_alias.py`) makes `pltrs` the alias. A toolkit that pltrs lacks
  gets a placeholder `__init__.py`.
* **Environment.** The runner uses pytest-xdist with 8 workers
  (`PLTRS_SUITE_WORKERS` sets another number) and pytest-timeout at 600 s
  per test. It sets `MPLCONFIGDIR` under the mode's directory and leaves
  `PLTRSCONFIGDIR` unset. It uses matplotlib's pytest options
  (`--import-mode=importlib`) and removes an old font list cache.

  The committed outcomes were recorded with TinyTeX, Ghostscript
  (`~/.local/gs/bin`) and Inkscape on `PATH`. Run both modes in the same
  way. `testing.compare` converts PDF and EPS files with Ghostscript and
  SVG files with Inkscape. `python/tools/suite-tools.sh install`
  installs Inkscape, the fonts that tests name and missing TeX packages
  into `~/.local/mpl-suite`.
  The runner gives both modes the variables of
  `~/.local/mpl-suite/environment` (`XDG_DATA_HOME`, `TEXMFHOME`,
  `TEXMFVAR`, `TEXMFCONFIG`). It tells which tools a run lacks. Those
  tests then skip in both modes.
* **Outcomes.** `spec/mpl-suite/<mode>.json` holds the counts, the
  collection errors and the outcome of each test id (`passed`, `failed`,
  `error`, `skipped`, `xfailed`, `xpassed`), one per line. Commit
  `pltrs.json` with your work. A run with patterns replaces only
  the entries of its files. The reasons of failures go to
  `target/mpl-suite/<mode>/details.json`, and pytest's output to
  `pytest.log` beside it.
* **The report.** The report shows, in this order:
  1. the counts of each run
  2. per test file: the tests that pass under matplotlib, how many of
     them pass under pltrs, the share and the main blocker
  3. the total share, which is the measure
  4. the tests that the reference skips and pltrs does not. They must
     skip in the same way.
  5. the blockers, largest first, with the tests and files they hold
     back
* **GUI toolkits.** The GUI backend tests need a display, the toolkits
  and their bindings. The tests of dates, units and plotting need pandas.
  `python/requirements-suite.txt` pins pandas, tornado and the Qt
  bindings, and `cargo xtask python setup` installs them into both venvs.
  `python/tools/toolkits.sh install` unpacks GTK 3 and 4, wxPython,
  cairo and Qt's X libraries into `~/.local/gtk`, without root.

  On Linux, the runner gives both modes the same environment:
  - the toolkits' paths (`toolkits.sh env`), after the runner's entries
    on `PYTHONPATH`
  - `$DISPLAY`, or its own Xvfb with `-noreset` on a free display. The
    runner stops the Xvfb by its process id at the end.
  - no `$WAYLAND_DISPLAY`
  - `LC_ALL=C.UTF-8`, so that the outcome of `test_use_locale` does not
    depend on the host or on earlier Qt or GTK tests

  Qt picks its xcb plugin itself (`QT_QPA_PLATFORM` stays unset). If the
  plugin lacks a library, the runner runs Qt offscreen and says so. The
  tests of missing parts skip, and the report counts them as not
  passing. A run of some files keeps the whole suite's order, because a
  Qt test makes a later GTK test in its worker skip. The command
  `toolkits.sh run COMMAND` runs a command with the toolkits. The smoke
  tests of `cargo xtask python test` need it
  (`python/tests/test_backend_gtk_wx.py`).
* **Collection errors hide the next gap.** A test file that fails to
  import shows only its first missing import. `--imports` lists all of
  them, per file and in total, weighted by the reference-passing tests
  that they block.

## Choosing what to work on

Work by test file. The suite reports per file, and the file's missing
imports (`--imports`) are what its tests need first:

1. Pick a file, or a group of files that need the same missing module.
2. Make it import.
3. Make its tests pass.
4. Run its pattern and commit the new `pltrs.json`.

The blocker list is the plan: work on the largest blockers first. Every
test that passes under matplotlib now passes under pltrs, so the suite
guards what is done. New work comes from the other measures: the
libraries' suites, the notebooks, the differential tester, `--docs`,
`--types` and `--private`.

## Shared files

Most work changes only its own module's files: `modules/<name>.rs` and
`python/pltrs/<name>.py`. The files below are shared. Keep your changes
there small, and add instead of change:

* `crates/pltrs-python/src/modules/mod.rs`: one `mod` line and one
  `register` line per module. In a merge conflict, keep both lines.
* `python/pltrs/__init__.py`, `pyplot.py`, `axes/_axes.py`, `figure.py`,
  `artist.py`: add methods and functions, and keep the existing ones in
  place.
* `binding/`: the core. Add a conversion or a helper where it belongs,
  with its tests. Everything depends on the core, so change existing
  behavior only on purpose.
* `spec/mpl-suite/pltrs.json`: never resolve a conflict by hand. Take
  either side and run `cargo xtask mpl-suite` again after the merge.

## Conventions for the Python code

* Write idiomatic Python with matplotlib's names and signatures. Write
  comments only where the code cannot explain itself.
* Docstrings are matplotlib's. Copy them into the source with the
  decorators and snippets that interpolate them (`_docstring.interpd`,
  `Substitution`, `copy`, `kwarg_doc`, the `%(Class:kwdoc)s` tables).
  Then `help()`, `inspect` and Sphinx read what they read in matplotlib.
  If matplotlib's callable has none (``# docstring inherited``), ours has
  none either. Put what pltrs does differently in comments, not in the
  docstring. `cargo xtask diff --docs` compares them with matplotlib's. A
  module that uses `%(Line2D:kwdoc)s` imports the module of that class.
* pltrs keeps the docstring tables of the package's classes in the cache
  directory (`_docstring._KwdocTables`) until a file of the package
  changes. `_docstring.cleandoc` is a faster `inspect.cleandoc`.
* Use matplotlib's class layout. If matplotlib's class defines a method,
  ours does too, even if it only calls the shared one (``def
  draw(self, renderer): super().draw(renderer)``). Then `__qualname__`,
  `__module__` and ``help`` show the class. A shared mixin, or a mixin
  before the documented base in the MRO, has no docstring. A comment
  says what it does. If it hides a constructor's docstring, set the
  docstring (``__init__.__doc__ = Axes.__init__.__doc__``).
* Programs import some names from matplotlib's modules that come from
  their imports (``np``, ``mpl``, sibling modules, re-exports). Import
  these at the end of the module, after the comment "What matplotlib's
  module also has from its imports". `cargo xtask diff --docs` lists the
  missing ones.
* Do not duplicate code. Put shared behavior in a base class (`Artist`)
  or a helper module (`_handles`, `cbook`).
* Keep code taken from matplotlib (`_api`, `_pylab_helpers`, the testing
  decorators) close to the original, and say so in its module docstring.
  It is under matplotlib's license (`LICENSES/matplotlib.txt`,
  `THIRD_PARTY.md`).
* pltrs requires only numpy, `cycler` (`rcsetup` returns `Cycler`
  objects), Pillow, `python-dateutil` (for `dates`) and `packaging` (for
  `parse_version` and `_get_executable_info`, imported at first call).
* pltrs (Rust) replaces matplotlib's other requirements (contourpy,
  kiwisolver, fontTools, pyparsing). pltrs does the same whether they
  are installed or not. A quad contour set refuses each `nchunk` that
  contourpy refuses, with contourpy's error (`_contour_chunks`). Only
  one object can come from these packages, a contour set's
  `_contour_generator`:
  - With contourpy, it is the generator that matplotlib makes, with the
    same arguments (`_contour_chunks.generator_arguments`). pltrs makes
    it at first use and shares it with contour sets made from that set
    (`contour(cs, ...)`). pltrs (Rust) still computes and draws every
    contour.
  - Without contourpy, it is `contour._ContourGenerator`, which contours
    the field again with pltrs (Rust). A program that reads contourpy's
    object depends on contourpy.
  - A `TriContourSet` uses the generator of pltrs' `_tri` module.
* `python/tools/check_dist.py --install` installs a wheel alone. It fails
  on an import of a package that pltrs does not require. The exceptions
  are packages that matplotlib imports as optional, and packages in the
  tool's `OPTIONAL`.
* The pltrs venv also holds what matplotlib's *test files* import at
  module level (`pyparsing`, `contourpy`), so that pytest can collect
  them.
* Unimplemented keywords raise `NotImplementedError("pltrs: ...")`.
