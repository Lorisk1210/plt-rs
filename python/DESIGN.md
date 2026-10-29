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
    registry.rs                 Python objects registered for native code
    init.rs                     the module definition
  src/modules/                  one file per matplotlib module
    mod.rs                      registration list (shared, see below)
    artist.rs axes.rs colors.rs figure.rs lines.rs rcsetup.rs ticker.rs ...
python/
  pltrs/                        the package; mirrors matplotlib's module tree
    __init__.py                 rcParams, RcParams, rc, rc_context, rcdefaults, use
    _native.abi3.so             built by `cargo xtask python` (gitignored)
    _api/                       matplotlib's _api, kept as it is
    _handles.py                 one Python object per pltrs (Rust) artist
    _pylab_helpers.py           pyplot's figure registry (Gcf)
    artist.py figure.py axes/ axis.py lines.py text.py patches.py
    collections.py container.py legend.py colors.py ticker.py units.py
    backend_bases.py cbook.py ft2font.py pyplot.py pylab.py style/
    testing/                    setup, decorators, compare, conftest, exceptions
    mpl_toolkits/               the toolkits (imported as pltrs.mpl_toolkits.*)
  mpl_suite/                    the suite runner's Python side (not in pltrs)
    mpl_suite_report.py         pytest plugin recording outcomes
    pltrs_alias.py              sitecustomize: `pltrs` aliases what loads as `matplotlib`
    import_gaps.py              `--imports`: every missing import of each test file
  tests/test_smoke.py           `cargo xtask python test`
  requirements-pltrs.txt        the pltrs venv (pinned)
  requirements-reference.txt    what the reference venv gets on top
spec/mpl-suite/
  reference.json                outcomes under real matplotlib (committed)
  pltrs.json                    outcomes under pltrs (committed)
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
  owns a copy (`numpy.frombuffer` over a `bytearray`, reshaped). No numpy
  C API.
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

### The native object and its threads

A native object holds one `'static` Rust value in a `RefCell`, the thread
that made it, and, for artist handles, the artist's `RawHandle`.

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

pltrs (Rust) handles share their figure through an `Rc`, so they are not
`Send`, while Python lets any thread holding the GIL reach any object. A
native object therefore refuses use from a thread other than its maker's
(`RuntimeError: this pltrs object belongs to another thread`), and one
deallocated on another thread leaks its value rather than dropping it
there. pltrs (Rust)'s rcParams are per thread too. pltrs is used from one
thread, as matplotlib's GUI backends already require; the handful of
matplotlib tests using threads with figures will need a plan of their own.

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
does not choose: `modules::register` only starts the thread's parameters
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
collections, texts, images, custom artists): a *loose* artist
(`Line2D::loose`, `Patch::loose`, `Collection::loose`, `Text::loose`,
`Custom::loose`) lives in a holder of its own, a figure state that is no
figure, so that its handle works as any handle does and its properties
can be read and set before it is added. `Axes::adopt_artist(artist,
AddAs::Line)` (matplotlib's `add_line`; `AddAs` names the `add_*`
method, which decides the data limits and the `_child<n>` label) and
`Figure::adopt_artist(artist, clip)` move it into the figure under a new
id, give it the default transform (`transData`, `transSubfigure`) unless
it has one, and return the handle there; an artist removed from the same
figure comes back under its own id.

In Python, a constructor makes the native loose artist with the `new`
functions of its module (`_lines.new(x, y, kwargs)`, `_text.new(x, y,
text)`, `_patches.new_rectangle(...)` and a `new_<shape>` per patch
class, `_collections.new_line_collection(...)` and a `new_<kind>` per
collection class; `Artist()` itself makes a plain artist that draws
nothing, `_artist.new()`) and calls `Artist.__init__`, which sets
`_figure` to None. Its getters and setters are then those of an added
artist. The `add_*` methods (`axes/_base.py`, `Figure.add_artist`) call
`_native.artist.adopt_into_axes` / `adopt_into_figure` (through
`Artist._add_to_axes` / `_add_to_figure`, which a class overrides when it
joins differently), and `_handles.adopt` gives the object the returned
handle and registers it in the figure. The returned handle is untyped (a
`RawHandle` native object), which the `artist_handle!` conversions accept
(see "The native object and its threads"). The loose handle is not used
again: its holder is empty. An artist's `axes` is the one pltrs (Rust)
has it in.

Artists that own others are made in their figure instead, as removed
artists, since matplotlib's constructors take their parent (`Axes(fig,
rect)`, `Legend(ax, ...)`): `fig.add_axes(ax)` then adds them back.

### Transforms

`transforms` is matplotlib's Python transform tree, computing with
`_native.transforms`. The transforms of the layout (`ax.transData`,
`ax.transAxes`, `fig.transFigure`, `subfig.transSubfigure`,
`fig.dpi_scale_trans`) are `_live_transforms.LiveTransform`s: each holds
a pltrs (Rust) `Transform` (`Transform::Data(axes)`, ...) that the figure
resolves in its current layout whenever points go through it, so they
follow limits, scales and layout engines without matplotlib's
invalidation; `ax.bbox` and `fig.bbox` are the boxes they map the unit
square to. An axes or figure makes each once, so `line.get_transform() is
ax.transData` holds. matplotlib's classes compose with them as with any
transform. A transform crosses to pltrs (Rust) (`Artist.set_transform`,
the `transform` keyword of the line properties) as the nested tuples of
`_native.transforms.native` (`_live_transforms.to_native`: composites,
blends, `ScaledTranslation`s and affine maps around live transforms),
and comes back from `describe` as matplotlib's objects around the live
transforms of the axes and figures it names (`from_native`). Python
transforms of other kinds (non-affine ones of their own) raise
`NotImplementedError`.

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
side a new copy from `changed()`.

### Callbacks and subclassing

Python code runs inside pltrs (Rust) through `Callback`: the native function
takes the callable, and the pltrs (Rust) closure or trait object holds it
(`Callback` is `Send`, as pltrs (Rust) requires of what runs during layout and
drawing). `ticker.rs` shows the pattern: the Python formatter of an axis
becomes a pltrs (Rust) formatter calling its methods as matplotlib's
`Axis` does (`Formatter._label_axis`: `format_ticks`, then `get_offset`),
handing it pltrs (Rust)'s description of the axis being drawn, which its
`_axis_info()` returns meanwhile.

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

pltrs (Rust)'s rule that code run while a figure is laid out or drawn cannot
use the figure's handles (it would re-borrow the figure) applies to
Python callbacks too: a native call made from such a callback that touches
the same figure panics, which arrives as the `RuntimeError` above.

### Warnings

pltrs (Rust) issues its warnings through `pltrs::warnings` (data cast to
a narrower type to be drawn, say). The extension sends them to
`_api.warn_external`, which `_api` registers, so they are matplotlib's
`UserWarning`s; one that the warning filters turn into an exception
raises from the native call that issued it, as a callback's exception
does.

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
export machinery), and `pyplot.figure` makes managers through it. The
GUI-toolkit backends (Qt, GTK, Tk, wx, macOS) are not there: loading one
raises `ImportError`, as matplotlib does without the toolkit.

A canvas draws and prints through the figure's native handle
(`_native.backends`):

* `FigureCanvasAgg.draw` draws the figure onto the raster canvas pltrs
  (Rust) keeps per figure; `buffer_rgba`, `copy_from_bbox`,
  `restore_region` and `blit` work on it, and `RendererAgg` stands for
  it.
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
* Canvas callbacks belong to the figure (`figure._canvas_callbacks`), so
  that they survive a change of canvas.

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
changes (an engine knows the figures it is set on).

A `Legend(parent, handles, labels, **kwargs)` makes the native legend of
its parent at once: matplotlib's argument handling (`_parse_legend_args`,
handler maps by handle and type) runs in Python, and the entries reach
pltrs (Rust) explicitly: an artist's handle (a loose line or rectangle
becomes a proxy drawn as it is now), a tuple with its `HandlerTuple`
layout, or the native handle of a container found on the axes
(`_native.legend.candidates`, which lists what matplotlib's
`_get_legend_handles` lists).

### Pickling

Decided, not done yet: a figure pickles as a serialisation of its native
state, since pltrs (Rust) holds everything drawn and pickles must load in
another process (`test_pickle_load_from_subprocess`). pltrs (Rust) will
get a versioned binary archive of a figure (`pltrs::figure::archive`):
every artist type encodes and decodes its state, artist ids included, so
that the tree, the transforms and the shared axes come back as they were.
`Figure.__reduce__` then returns `(_figure_from_archive, (bytes, state))`,
where `state` holds the Python side the archive cannot: the `__dict__`
of each artist object (without `_native` and `_figure`) by artist id and
the figure's own attributes. Unpickling rebuilds the native figure from
the bytes, wraps every artist id with the class it had, restores the
attributes, and re-registers the figure with pyplot as matplotlib's
`Figure.__setstate__` does (`restore_to_pylab`). Axes and other artists
pickle through their figure (`(_artist_of_figure, (figure, id))`).
Recording the calls that built a figure and replaying them was ruled
out: it cannot capture changes pltrs (Rust) makes itself (layout,
autoscaling) nor artists changed through their natives.

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
  test, `MPLCONFIGDIR` under the mode's directory, matplotlib's pytest
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
  Pillow, nothing else. The pltrs venv also holds what matplotlib's *test
  files* import at module level (`packaging`, `pyparsing`,
  `python-dateutil`, `contourpy`), so that those files can be collected;
  pltrs itself does not import them.
* Unimplemented keywords raise `NotImplementedError("pltrs: ...")`.
