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
also pass under pltrs (`cargo xtask mpl-suite`). It must reach 100%.

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
    registry.rs                 Python objects registered for native code
    threads.rs                  every thread holding the GIL uses every object
    home.rs                     other threads' calls on windows, run by the main thread
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
  tests/test_smoke.py           `cargo xtask python test`
  backend/pltrs_backend/        the PEP 517 build backend (standard library only)
  alias/                        the `matplotlib` distribution: matplotlib's names for pltrs
  ecosystem/                    `cargo xtask ecosystem`: pandas and seaborn on pltrs
                                (and `fetch.py`, which downloads their sdists)
  tools/toolkits.sh             GTK 3 and 4, PyGObject, pycairo, wxPython without root
  requirements-pltrs.txt        the pltrs venv (pinned)
  requirements-reference.txt    what the reference venv gets on top
  requirements-ecosystem.txt    the ecosystem venvs (pandas, seaborn, scipy, pytest)
  requirements-ecosystem-reference.txt  what the ecosystem's reference venv adds (matplotlib)
pyproject.toml                  `pip install .`: the backend and the distributions
spec/mpl-suite/
  reference.json                outcomes under real matplotlib (committed)
  pltrs.json                    outcomes under pltrs (committed)
spec/ecosystem/
  reference.json, pltrs.json    the same for pandas' and seaborn's suites
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
  `home`'s wait for the main thread releases it, touching nothing), and
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
the suite skips here for lack of a display and toolkits.

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

pltrs (Python) draws in pltrs' modern look by default, as pltrs (Rust)
does: the `pltrs-modern` style sheet, Inter, constrained layout (see
`pltrs::look`). `PLTRS_LOOK=matplotlib` (or `PLTRS_FIGURE_LOOK`) selects
matplotlib's look, in which the output is matplotlib's. The extension
does not choose: `modules::register` only starts the parameters
from the look pltrs (Rust) resolves from the environment. `rcParams` then
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
rect)`, `Legend(ax, ...)`): `fig.add_axes(ax)` then adds them back.

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
`(row, column)` key in its table. A cell made by its constructor
(`Cell(xy, width, height)`) lives in a loose table without an axes,
until `table[row, col] = cell` copies it into a table and makes the
object that table's cell.

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
to the next draw), contour sets and stream plots autoscale when made, and
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
matplotlib.

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
the interval of a spine that of an overridden `Spine._adjust_location`.
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
their tick helpers. Any other converter is the program's: its axis keeps
it in Python, `_update_axisinfo` installs its `AxisInfo` natively
(`_tick_helpers.use_axis_info`, `Axis::take_axis_info`), and lines made
from its values keep them (`_xorig`) to convert them again when the unit
changes. An artist made outside any axes with values in units converts
them when it joins one (`_convert_units_for`).

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

An exception a callback raises waits in the frame of the native call
running (`binding/callback.rs`). Where matplotlib's Python code catches
one that pltrs (Rust) runs the program's code for, pltrs (Rust) asks
the binding through `pltrs::layout::failures` (installed by
`modules/figure_layout.rs`), which takes an exception of the class asked
out of the frame (`binding::forgive`): a layout engine's pass stops where
the program's code failed, as matplotlib's does; `Figure.draw` passes
over a `ValueError` of that pass and draws without the rest of the
layout; and measuring an axis for a layout measures it again in full
after a `TypeError` (matplotlib's `_get_tightbbox_for_layout_only`).

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
* **Mirrors** (`_axis_artists`, `_native.axis` `mirrors`): the ticks
  (`Tick` with `label1`, `label2`, `tick1line`, `tick2line`, `gridline`),
  the axis label and the offset text are loose `Text` and `Line2D`
  artists that a native `sync` fills from the axis when they are handed
  out and that every setter pushes back (`push`): what one tick has of its
  own is what differs from the style of its level, kept as pltrs (Rust)'s
  per-tick properties (`Axis::set_tick_label_props`).

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
is refused. Values in units plotted on 3-D axes pass as they are
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

pltrs (Rust)'s callback traits cannot fail the Python way, so when the callable
raises, the exception is kept in the frame of the native call that is
running, the callback returns a fallback (`None` from `call`), later
callbacks of that call are skipped, and the native function raises the
kept exception when pltrs (Rust) returns. Each native call has its own frame, so
a native function a callback calls cannot pick up its caller's exception.
A callback outside any native call turns its exception into a
`RuntimeWarning`.

Python subclasses that override drawing or computation (a `Formatter`,
`Locator`, `Transform`, `Artist` subclass) follow the same route: the
native side is given the Python object as a callable or as an object
whose methods a Rust adapter calls through `Callback`. A Rust value that
must refer back to Python objects that can reach it (an event callback
holding its figure) must hold them weakly, or the cycle runs through Rust
where Python's garbage collector cannot see it.

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

Other code pltrs (Rust) runs while it lays a figure out or draws it with
the figure held (a transform, a norm, the text measurer) finds the figure
in use: taken out while it is laid out, borrowed for reading while it is
drawn. matplotlib's may still change it, so a
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

A draw stops where the program's code it runs before painting fails, as
matplotlib's draw stops at an exception: a layout hook or draw hook of the
package's that raises (`DrawHook::stopping`; the projection of 3-D
offsets that do not broadcast, `_locate_by_program`) leaves the figure
laid out but its artists undrawn (a `Line3D` keeps the data it had).

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

Problems met while drawing are not raised by the artist meeting them:
pltrs (Rust) draws the rest and returns them all (`DrawErrors`), each with
the kind of exception matplotlib raises for it (`DrawErrorKind`). The
binding raises the first one as matplotlib would have, stopping there: a
`ValueError` with its message for a value of the program's that cannot be
drawn (malformed mathtext, an infinite line through two identical
points), else a `RuntimeError` naming them all. `draw_without_rendering`
draws with a renderer that paints nothing, so it raises them too.

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
found.

### Text measured and drawn by the program

matplotlib measures every line of text through the module function
`text._get_text_metrics_with_cache` and draws every text through
`Text.draw`; its tests replace both (`monkeypatch.setattr`, the
`text_placeholders` fixture) to turn texts into boxes. `_text_overrides`
makes such replacements take effect: the `text` module's class and
`Text`'s metaclass notice when either attribute is replaced or restored,
and hand pltrs (Rust) a `pltrs::text::LineMeasurer` (every line it lays
out, tick labels included, is measured by calling the module function;
the font height metrics come from `FT2Font.get_sfnt_table` as in
`Text._get_layout`) or a `pltrs::text::TextPainter` (every text it draws
is drawn by calling `Text.draw` on a stand-in `Text` in display
coordinates, with a renderer whose `Artist.draw` queues artists that
pltrs (Rust) then draws in the text's place). Without replacements pltrs
(Rust) never calls Python for text.

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
the renderer laying out (`text._layout_renderer`): under the PDF canvas that
is a `RendererPdf` (`FigureCanvasPdf._print_renderer`), which measures as
the PDF backend does but draws nothing, so `fig._get_renderer()` stays the
Agg renderer drawing onto the raster canvas. matplotlib's `Text.draw` lays
a tick label out again where it draws it; while a cache is installed, pltrs
(Rust) looks the label's lines up in it again where it draws the label
(`LaidOutText::measure_again`), so the cache sees the same lookups without
the cost of a second layout.

The renderer's text drawing can be replaced too. `RendererAgg.draw_text`
and `draw_tex` draw a line onto the canvas (`_native.drawing.draw_text`,
see "Drawing defined in Python").
While a canvas's renderer has a `draw_tex` other than its own (assigned to
the instance, or `RendererBase.draw_tex`), its draws hand pltrs (Rust) a
`pltrs::text::TexPainter`: every TeX line is drawn by calling it, and the
paths it draws with `draw_path` are queued and drawn by pltrs in the
line's place. While `RendererAgg.draw_text` is `RendererBase.draw_text`
(the class watches it), raster canvases draw text as paths
(`pltrs_render::set_texts_as_paths`), as `_draw_text_as_path` does.

usetex looks TeX fonts up through `dviread.PsfontsMap.__getitem__`, which
tests replace: `PsfontsMap`'s metaclass notices it, and pltrs-text's font
map lookups (`pltrs_text::tex::set_font_map_lookup`) call the replacement
while it is in force.

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
  `Ellipse._recomputed`, `Arc._given_angles`), gets a painter
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
  resolves its transforms in the figure drawing it.
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

A painted artist's object is pickled with its figure (the painter is
archived as the object), so a figure with one pickles as matplotlib's
does, or fails with pickle's error for a class pickle cannot find.

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
  (whatever `FigureCanvasAgg.draw` and `blit` put there) above a toolbar
  of the backend's buttons, and reports input, resizes, toolbar clicks,
  close requests, chosen files and timers as tuples, which
  `backends/_pltrs_loop.py` hands to the canvas (matplotlib's events,
  in matplotlib's coordinates), the timer or the file chooser waiting.
  Navigation, keys and callbacks stay matplotlib's (`NavigationToolbar2`
  or the tool manager over pltrs' toolbar). The loop runs in short
  slices, between which signals arrive (Ctrl+C) and the calls other
  threads queued for the windows run (see "Threads"); its framework
  counts as running once the host is
  made (`cbook._get_running_interactive_framework`). A headless host
  (`_pltrs_loop.connect_headless`) opens windows off screen and takes
  injected input, for tests.
* **Tk** (`backends/_backend_tk.py`, `backend_tkagg.py`, matplotlib's)
  blits through `_tkagg`, pltrs' counterpart of matplotlib's C helper,
  which hands Tk the pixels with its photo commands, as PNG data.
* **Qt, GTK 3 and 4, wx** are matplotlib's modules, kept as they are but
  for their imports; their canvases paint the Agg canvas' pixels through
  the buffer of `copy_from_bbox`'s region (`BufferRegion.__buffer__`).
  Without the toolkit they raise matplotlib's `ImportError` (the Qt
  backends after its binding selection, `qt_compat`). pltrs has no Cairo:
  the `*cairo` backends draw with the raster too, so their widgets show
  the pixels the `*agg` ones show (matplotlib's own Cairo variants differ
  from its Agg ones by their text and antialiasing).
* **Cairo** (`backends/backend_cairo.py`) is matplotlib's module, which
  needs pycairo or cairocffi as matplotlib's does: `RendererCairo` and
  `GraphicsContextCairo` draw onto a Cairo context of the program's (a
  figure drawn with one goes through their methods, see "Drawing defined
  in Python"), while the `cairo` backend's canvas, `FigureCanvasCairo`,
  is the Agg canvas, printing PDF, PostScript and SVG with pltrs (Rust)'s
  writers at 72 dpi as matplotlib's Cairo surfaces are.
* **WebAgg** (`backend_webagg.py`) runs pltrs (Rust)'s web server
  (`_native.web`, a `pltrs::web::WebRelay`) instead of Tornado: it serves
  matplotlib's pages and files and hands the WebSockets' messages and the
  downloads to matplotlib's figure managers (`backend_webagg_core`), from
  an asyncio loop that also runs their timers, under the URL prefix
  `WebAggApplication.initialize` is given (`WebOptions::url_prefix`).
  **nbAgg** is matplotlib's, over the kernel's comms.
* **Jupyter's inline backend**: IPython's kernel selects
  `module://matplotlib_inline.backend_inline`, whose module imports
  `matplotlib`; unless that gives pltrs (pltrs loaded as `matplotlib`,
  or its `matplotlib` distribution installed), the registry loads pltrs'
  port of it instead (`backend_inline.py`), which registers the kernel's
  formatters for pltrs' figures.

A canvas draws and prints through the figure's native handle
(`_native.backends`):

* `FigureCanvasAgg.draw` draws the figure onto the raster canvas pltrs
  (Rust) keeps per figure; `buffer_rgba`, `copy_from_bbox`,
  `restore_region` and `blit` work on it, and `RendererAgg` stands for
  it: `Artist.draw` with it draws the artist onto that canvas, which is
  how `Axes.draw_artist` and `Figure.draw_artist` blit (`axes/_blitting.py`,
  `_figure_interaction.py`). The vector canvases' `draw` lays the figure
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
  transparency are Pillow's), AVIF (which pltrs (Rust) does not encode)
  and any `pil_kwargs` but a JPEG `quality`: those are drawn by pltrs
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
is unset (`config::honour_matplotlib_config_dir`, which `modules::register`
calls before anything looks the directories up), so that it works in
matplotlib's environment unchanged. When the directory cannot be made or
written to, a temporary one stands in, and importing the package logs
matplotlib's warning (`_log_temporary_config_dir`), naming the variable
that named the directory. The suite runner sets `MPLCONFIGDIR` and
removes the `PLTRSCONFIGDIR` cargo sets.

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

### Imports inside the package

The suite loads `python/pltrs` under the name `matplotlib`, so:

* modules of the package import each other relatively
  (`from . import _handles`, `from ..figure import Figure`), never as
  `pltrs`;
* the toolkits (`pltrs/mpl_toolkits`) and `pylab.py`, which the suite loads
  as top-level packages, import pltrs by name (`from pltrs import colors`);
  `python/mpl_suite/pltrs_alias.py` makes that name resolve to the
  modules loaded as `matplotlib`;
* the native module is `from ._native import lines as _lines` (its
  submodules are attributes).

### Distributions

`pyproject.toml` builds pltrs (Python) with the tree's own PEP 517
backend (`python/backend/pltrs_backend`), written in Python's standard
library alone so that `pip install .` needs nothing but cargo. It builds
two distributions:

* `pltrs` (the default): the release build of `pltrs-python` and the
  package, a wheel tagged `cp312-abi3-<platform>` whose platform the
  backend reads from the library built (`_tags`: the glibc versions of
  its ELF symbols on Linux, the deployment target on macOS, the
  architecture on Windows), never from the Python running the build;
* `matplotlib` (`-C distribution=matplotlib`): `python/alias`, pure
  Python. Its modules make `matplotlib`, `mpl_toolkits.<kit>` and
  `pylab` the modules of pltrs (`_pltrs_matplotlib_alias`: a meta path
  finder, the reverse of the suite's `pltrs_alias.py`), so that code
  importing matplotlib by name runs on pltrs. It is named `matplotlib`
  for pip: what requires matplotlib is satisfied by it, and it replaces
  a real matplotlib (or is replaced by one) openly.

`cargo xtask ecosystem` runs pandas and seaborn on the tree through the
alias (`python/ecosystem`), comparing each figure's structure with what
real matplotlib recorded (`expected.json`, recorded with
`ECOSYSTEM_RECORD=1` under matplotlib). `cargo xtask ecosystem suite`
runs the libraries' own suites, pandas' plotting tests and seaborn's
`tests/`, from their sdists (`ecosystem setup` downloads them into
`target/refs/`, with the data files the sdists leave out, checking their
SHA-256), as `mpl-suite` runs matplotlib's and with its outcome files and
report: in a venv with real matplotlib (`--reference`,
`target/ecosystem/reference-venv`) and in one without it, through the
alias (`target/ecosystem/venv`, installed with matplotlib excluded from
the resolution). Each library runs in a site of its own laid out as its
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
* **Environment.** pytest-xdist with 8 workers, pytest-timeout at 600 s a
  test, `MPLCONFIGDIR` under the mode's directory (and no
  `PLTRSCONFIGDIR`, so that pltrs reads `MPLCONFIGDIR`), matplotlib's pytest
  options (`--import-mode=importlib`). The committed outcomes were
  recorded with TinyTeX on `PATH`
  (`export PATH="$HOME/.TinyTeX/bin/x86_64-linux:$PATH"`), which runs the
  usetex tests: run both modes the same way. Ghostscript is installed in
  `~/.local/gs/bin` (put it on `PATH` too): `testing.compare` converts PDF
  and EPS files to PNG with it, as matplotlib's does, so those image
  comparisons run in both modes; there is no Inkscape, so SVG comparisons
  skip with matplotlib's message.
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
* **GUI toolkits.** Without a display the suite skips the GUI backends'
  tests (`test_backends_interactive.py`, `test_backend_gtk3.py`, `test_backend_tk.py`,
  ...), and without pycairo the Cairo backend's. Tk needs only tkinter;
  GTK 3 and 4 with PyGObject and pycairo, and wxPython, come from
  `python/tools/toolkits.sh install`, which unpacks Debian's packages
  into `~/.local/gtk` and builds the bindings there for the venvs'
  Python, without root. `toolkits.sh run COMMAND` runs a command with
  them (library, typelib and data paths, the bindings on `PYTHONPATH`,
  which the runner keeps behind its own entries), so with an X server
  (`Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &`):

  ```sh
  export DISPLAY=:117
  python/tools/toolkits.sh run cargo xtask mpl-suite --reference \
      test_backend_gtk3 test_backends_interactive test_getattr test_backend_cairo
  python/tools/toolkits.sh run cargo xtask mpl-suite \
      test_backend_gtk3 test_backends_interactive test_getattr test_backend_cairo
  python/tools/toolkits.sh run cargo xtask python test
  ```

  runs those tests in both modes, and the smoke tests of the real
  toolkits (`python/tests/test_backend_gtk_wx.py`). The committed
  outcomes are recorded without toolkits or display: put
  `spec/mpl-suite/*.json` back (`git checkout spec/mpl-suite`) after such
  a run.
* **Collection errors hide the next gap.** A test file that fails to
  import shows only its first missing import; `--imports` lists all of
  them, per file and in total, weighted by the reference-passing tests of
  the files needing them.

## Choosing what to work on

Work by test file: a file is the unit the suite reports, and the file's
missing imports (`--imports`) are what its tests need first. Pick a file
(or a group sharing a missing module), make it import, then make its tests
pass, then run its pattern and commit the new `pltrs.json`. The blocker
list, sorted by tests held back, is the plan; large files with one missing
module (`test_mlab.py`: 2540 tests behind `matplotlib.mlab`) come first.

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
  either side and rerun `cargo xtask mpl-suite` (the full run takes under
  a minute) after merging.

## Conventions for the Python code

* Idiomatic Python, matplotlib's names and signatures; docstrings on
  every public module, class and function (matplotlib's first line, or a
  short one of our own), comments only where the code cannot speak.
* No duplication: shared behaviour lives in a base class (`Artist`) or a
  helper module (`_handles`, `cbook`), as in matplotlib.
* Code taken from matplotlib (pure-Python modules such as `_api`,
  `_pylab_helpers`, the testing decorators) is kept close to the original
  and says so in its module docstring; it is under matplotlib's licence
  (`LICENSES/matplotlib.txt`, `THIRD_PARTY.md`).
* pltrs imports numpy, `cycler` (in `rcsetup`, whose validators return
  `Cycler` objects as matplotlib's do) and, where matplotlib does,
  Pillow and `python-dateutil` (`dates` parses date strings and resolves
  time zone names with it), nothing else. The pltrs venv also holds what
  matplotlib's *test files* import at module level (`packaging`,
  `pyparsing`, `contourpy`), so that those files can be collected;
  pltrs itself does not import them, but for one optional import:
  where contourpy is importable, a contour set's `_contour_generator` is
  the contourpy generator matplotlib would make, with the same arguments
  (`_native.contour.generator_arguments`), made when first asked for and
  shared with the contour sets made from it (`contour(cs, ...)`); pltrs
  (Rust) still computes and draws every contour. Without contourpy it is
  pltrs' own `contour._ContourGenerator`, which holds the field pltrs
  (Rust) contours again. A `TriContourSet`'s is the `_tri` module's
  generator (pltrs' `_tri`, matplotlib's C++ extension over pltrs (Rust)'s
  triangulation).
* Unimplemented keywords raise `NotImplementedError("pltrs: ...")`.
