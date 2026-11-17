# Differences from matplotlib

pltrs (Python) is meant to run code written for matplotlib 3.11.2
unchanged and give the same figures, values and errors. This file lists,
by topic, every place where it is known not to. Some differences are on
purpose (pltrs' own look and windows, the name it writes into files);
some are matplotlib's bugs that pltrs does not repeat; and some are
things not built yet, at the end.

The test harness keeps a list of every intended difference it measures
(`spec/diff/*allowlist.txt`, `spec/reference/not_applicable.txt`, the
probe waivers in `crates/pltrs-spec`), each entry with its proof. Each
entry names the section of this file it belongs to, and
`python/tests/test_differences.py` checks that it does: nothing a list
excuses is missing here.

## The look of figures

* **Under the name `matplotlib`, figures have matplotlib's look**; under
  the name `pltrs`, pltrs' modern look (its `pltrs-modern` style: Inter
  type, constrained layout, its own colour cycle, thicker lines, no top
  or right spine). The look is fixed by the name under which the package
  is first imported in a process: a program that imports `pltrs` first
  keeps the modern look even when a library it uses imports `matplotlib`
  afterwards, and one that imports `matplotlib` first keeps matplotlib's.
  `PLTRS_LOOK=matplotlib` (or `modern`) in the environment, or a look
  chosen in code, overrides the name. `rcdefaults()`, `rcParamsDefault`
  and `style.use("default")` give matplotlib's defaults in either look.
* `style.available` and `style.library` list two more sheets,
  `pltrs-modern` and `pltrs-modern-dark`. A program that draws one
  figure per style draws two more. These two sheets ask for semibold
  titles, which the bundled DejaVu fonts lack, so the font manager logs
  its substitution for them.
* Under matplotlib's look, the font manager leaves pltrs' bundled Inter
  fonts out of `fontManager.ttflist` and so out of font matching, but
  their files lie in the data directory's `fonts/ttf`. Under the modern
  look, Inter's five faces are among the fonts besides matplotlib's
  DejaVu and STIX fonts.
* `rcParams` holds pltrs' own `pltrs.*` parameters besides matplotlib's;
  their defaults leave matplotlib's look unchanged.
* With a display, the window backend chosen by default is pltrs' own
  (see "Windows and backends").

## Windows and backends

* With a display, the interactive backend chosen by default is pltrs'
  own windows (`pltrs`, or `macosx` on macOS), ahead of Qt, GTK, Tk and
  wx. `plt.get_current_fig_manager().window` is then pltrs' window, not
  a Qt or Tk widget. Set `MPLBACKEND=qtagg` (or another backend) to get
  matplotlib's choice.
* `backend_registry.list_all()` lists `pltrs` after matplotlib's
  backends, and so do the errors of `use()` and `validate_backend()`
  that name the valid backends.
* pltrs' windows give a program matplotlib's QtAgg input events, but
  compose dead keys and pass Shift+Tab to the canvas, as matplotlib's GTK
  and Tk backends do. On Wayland, where no program learns where the
  pointer goes outside its window, leaving the window is reported where
  the pointer was last.
* `backend_webagg` imports and serves without Tornado, where matplotlib
  raises `RuntimeError`.

## Installation, configuration and data files

* pltrs reads its configuration from `PLTRSCONFIGDIR` (else the XDG
  directories), and its rc file is `pltrsrc`; `MPLCONFIGDIR` and
  `matplotlibrc` files are read too.
* `get_data_path()` is pltrs' data directory in its configuration
  directory, where it writes the fonts and files it bundles, and
  `matplotlib_fname()` names its `pltrsrc` there when you have no rc file
  of your own. Paths built from them (the bundled fonts `findfont()`
  finds, `get_sample_data()`, the `stylelib` directory) are there too, and
  messages naming such a file name it there.
* The `mpl_toolkits` packages live inside pltrs' package, so an
  `ImportError` from them names files there.
* pltrs does not require fontTools. Where fontTools is installed,
  `backend_ps.fontTools` and `dviread.fontTools` are its modules, as in
  matplotlib; without it, they are missing.

## Files pltrs writes

* PDF, SVG and PGF files name pltrs as the software that wrote them (the
  PDF `Creator` and `Producer`, the SVG's `dc:title` of its `cc:Agent`,
  the PGF header), and PNG files carry pltrs' `Software` text chunk.
* PNG bytes are pltrs' own DEFLATE encoding of the same pixels, so the
  files, and the base64 text or the ids of SVG images hashed from them,
  differ from matplotlib's while the pixels are the same. A PDF with
  `pdf.fonttype` 42 embeds the same glyphs in a differently packed
  TrueType subset.
* WebP files are written losslessly: pltrs has no lossy WebP encoder,
  where matplotlib saves through Pillow's lossy default. The pixels of
  pltrs' file are the canvas' own; what a program gives in `pil_kwargs`
  goes through Pillow under both.
* With `ps.fonttype` 42 a PostScript file embeds the whole font where
  matplotlib embeds a subset of it: the file is larger and draws the
  same.
* In SVG and PostScript text a zero can carry the other sign (`-0` where
  matplotlib writes `0`, or the reverse), and the ids matplotlib hashes
  from that text (of markers and clip paths) differ with it. A rasterized
  artist in an SVG can be cropped one pixel column wider. The drawing is
  the same in both cases.
* A pickled figure holds pltrs' own archive of it, of another length
  than matplotlib's. pltrs loads the pickles pltrs writes, but pickles do
  not cross between pltrs and matplotlib in either direction (see "Not
  built").

## Classes and types

* A figure's `artists`, `images`, `lines`, `patches`, `texts` and
  `legends`, and an Axes' `child_axes`, are list subclasses that add and
  remove what you put in or take out. `isinstance(fig.lines, list)` holds
  and they compare equal to matplotlib's lists; `type(fig.lines) is list`
  does not.
* The classes of matplotlib's C++ modules (`ft2font.FT2Font`, `FT2Image`,
  the `_tri` classes, `BufferRegion`) are Python classes: `pybind11_object`
  is not among their bases.
* matplotlib's type stubs ship with pltrs. A few of matplotlib's instance
  attributes are properties or descriptors on pltrs' classes (the
  figure's and Axes' transforms, built on first read; a colour bar's
  `vmin`, `vmax`; a contour set's `cvalues`): reading and assigning them
  on an object behaves as matplotlib's, but `mypy.stubtest` and a read
  off the class itself see the descriptor.

## Where matplotlib fails and pltrs does not

Where matplotlib 3.11.2 raises, crashes or draws something wrong because
of a bug of its own, pltrs draws or answers:

* `BoxStyle.register` (matplotlib reads a `_Base` that no longer exists);
  an annotation whose arrow starts from a text at a non-finite position
  (matplotlib lets a `StopIteration` escape `savefig`); a negative
  `FT2Image` size (a crash in matplotlib); a box style whose box is
  smaller than one tooth (`roundtooth`).
* A categorical axis pickles (matplotlib's holds an `itertools.count`,
  which Python 3.14 refuses to pickle), so code that falls back when the
  pickle fails takes another branch.
* A `PathCollection` made directly, or a `PolyCollection` or
  `LineCollection` given offsets, keeps limits in data coordinates; in
  matplotlib its data limits come out in display pixels and the Axes
  shows nothing.
* A callback that clears its Axes, or removes their text, while they are
  drawn; a NaN in a constrained layout's `rect`; an Agg canvas that
  overflowed its cell limit once (matplotlib's keeps failing every later
  draw).
* A callback that redraws the figure while it is being drawn (a
  formatter, locator, norm or `xlim_changed` callback calling
  `canvas.draw()` or `draw_idle()`): matplotlib paints the figure twice
  into the same image, the second time over a half-finished first;
  pltrs draws it once, as the program's final state says.
* Values matplotlib leaves to chance: the colour of a NaN hue in
  `hsv_to_rgb` (uninitialised memory in matplotlib), the order of NaN
  ticks from `AsinhLocator`. A path with an undefined code fails the
  draw with `ValueError` where matplotlib's `KeyError` comes from a
  table lookup.

## Values that differ between any two runs

A program that reads the clock, a memory address, a random salt or a
`uuid`, or that iterates a set whose order follows hashing, gives other
values in every run, under matplotlib as under pltrs: the times a program
prints or plots, `id()`s, SVG ids salted without `svg.hashsalt`, HTML ids
of `to_jshtml()`, signal handler numbers of GTK, the last bits of
kiwisolver's solutions, and which of several equally good fonts a
font-lookup warning names. The test harness checks each such program by
running it twice under matplotlib.

## Your subclasses and callbacks

pltrs calls the methods your subclasses override (locators, formatters,
transforms, scales, artists' getters and `draw`) wherever their result
matters, and draws and measures what they return, but it does most of its
work natively and calls them another number of times than matplotlib:

* An offset box of yours inside a box pltrs lays out is measured once per
  layout (matplotlib asks again in each parent's `get_bbox`,
  `get_offset` and `draw`), and a projection's axis transforms are asked
  once per side rather than once per tick.
* Overrides of private members (`_set_lim_and_transforms`,
  `_update_transScale`, ...), getters such as `get_transform` or
  `get_path`, `pchanged`, `RendererBase.new_gc` and `PathCollection.__init__`
  (which `scatter` does not call) are called more or fewer times, and
  some receive floats where matplotlib passes the ints you gave
  (`Locator.nonsingular`, `limit_range_for_scale`).
* A `Figure.draw` that never calls the base class still prints the
  figure (matplotlib prints a blank canvas), and a `FancyArrowPatch.draw`
  that never calls the base keeps another dpi correction.
* A transform class whose `transform_affine` disagrees with its own
  `get_matrix` maps everything through the matrix.

The libraries known to override these members (MetPy, mpltern,
ultraplot, cartopy, mne, arviz, probscale, mplcursors, pytransform3d)
draw the same figures under both.

## Private attributes

matplotlib's private names that other libraries read or write work as
matplotlib's, with these exceptions:

* Values pltrs computes at every layout read differently before the first
  draw (an Axes' `_position`, boxes' `_points`, a mesh's `_paths`, an
  annotation arrow's `_posA_posB`, a surface's `_facecolor3d`).
* Holders are pltrs' own classes answering the same protocol
  (`_shared_axes`, `_twinned_axes`, `_align_label_groups`: `cbook.Grouper`'s;
  a table's `_cells`: a dict's; an Axes' `_children`: a list's).
* Integers matplotlib's code made are floats (`_loc`, `_base`, a geo
  Axes' centre), `_tight` is False where matplotlib keeps None, and
  `_renderer`, `_clippath`, `_locs` and `_remove_method` are of other
  types.
* Writes pltrs does not draw from, or pickle: `Axes3D._dist` (holoviews'
  `distance` option has no effect), a colour bar's `_values` (assigning
  it fails: pass `values=` to `colorbar`), a legend's `_fontsize`, a
  path assigned to a colour bar's outline spine or to a line's `_path`,
  a figure's `_transform`, and `Legend._offset`, `AutoDateLocator._freq`,
  a `GridSpec`'s `_nrows`/`_ncols` and a formatter's `_format`, which a
  pickle does not carry.
* Helpers with no state in pltrs are missing: a fill-between collection's
  `_bbox`, a polar spine's `_center` and `_width`, a 3-D collection's
  `_faces`, a contour set's `_process_args`, an axis artist's
  `_update_ticks`, a 3-D patch's `_segment3d` (see "Not built").
* `TransformNode._parents` is keyed by `id()`s, and the invalidation
  flags `_invalid` and the `Affine2D._inverted` cache are pltrs' own.

## Warnings and error messages

* For nonsense input (infinite or NaN coordinates, degenerate shapes,
  dates in a 3-D projection), pltrs does not give the `RuntimeWarning`s
  numpy gives when matplotlib computes with them (`invalid value
  encountered`, `divide by zero`, `overflow`), and where two values are
  wrong it may complain about the other one first.
* `set_resample("False")` is taken as true, where matplotlib's draw
  refuses the string.
* Without TeX, a usetex figure fails with matplotlib's `RuntimeError`,
  but where an earlier run left typeset texts in the cache, the
  `FileNotFoundError` for the missing font metrics may name another
  font than matplotlib's.
* The "findfont: Failed to find font weight ..." warnings are logged a
  different number of times in some programs (pltrs looks up math and
  fallback fonts in another order); the fonts drawn are the same.

## Numbers on other machines

pltrs follows what numpy and matplotlib's wheels compute on the machine it
runs on: numpy's processor level (the order `argsort` leaves ties in),
fused multiply-adds on arm64, Apple's Accelerate. Not followed, and seen
in last bits or in ill-conditioned results only:

* numpy's OpenBLAS kernels, which differ by processor (long sums, LAPACK);
* numpy's SVML maths on Linux with AVX-512, in pltrs (Rust) alone;
* on Apple silicon, Accelerate's products of shapes other than 3×3 and
  4×4 and its LAPACK, and the sign of rounding noise in numpy's FFT;
* on arm64, Agg's handling of infinitely wide strokes (matplotlib fails
  with "Exceeded cell block limit"; pltrs leaves such corners out), and
  numpy's saturating conversion of infinite dates;
* on aarch64 Linux, Gouraud-shaded colours.

Some messages and warnings differ with the system too: matplotlib's
"building the font cache" warning on a Mac (pltrs reads the system's
fonts in milliseconds), the order of fonts named for a missing glyph, a
file system that ignores case.

## Not built

* **Pickles do not cross** between pltrs and matplotlib: a figure pickled
  by matplotlib does not load under pltrs, nor the other way round.
* **`matplotlib.sphinxext` ships no modules**: a Sphinx project using the
  plot directive (`.. plot::`) or `mathmpl` fails at its configuration.
* **3-D patches of your own**: a subclass of `Patch3D` gets no
  `_segment3d`, and its own `draw` is not called; qiskit's Bloch sphere
  draws its vectors without arrowheads.
* **usetex on axis artists**: the labels and tick labels of
  `mpl_toolkits.axisartist` set to usetex are typeset without TeX (and
  draw where TeX is missing, where matplotlib raises).
* **TeX through a PGF renderer called directly**: `RendererPgf.draw_tex`
  runs TeX on the string, where matplotlib's writes it into the picture
  for the document's TeX to typeset (saving a figure as PGF is not
  concerned).
* **ultraplot's mathtext fonts** route some glyphs to Computer Modern
  through its own font class, glyph by glyph; pltrs typesets the formula
  with the font the class names.
* **Arrays changed in place** after reading them from a few getters (some
  of 3-D artists' and patches' state, a step line's path) are read back
  changed but not drawn changed, or the other way round, before the
  first draw.
* **Polygon markers** with a number of sides that is not a positive
  integer: `MarkerStyle((4.5, 0))` has 5 corners where matplotlib's has
  6, and `MarkerStyle((-3, 0))` fails where matplotlib makes an empty
  marker.
* **Shared axes across Axes classes**: a plain Axes sharing an axis with
  an Axes whose class keeps sharing groups of its own follows its limits
  (in matplotlib it does not), and the axes of an Axes removed from its
  figure measure nothing.
* **musl (Alpine)**: a draw started inside a draw (a callback saving its
  figure) in a thread other than the main one raises `RecursionError`.

## pltrs (Rust)

matplotlib's own tests that the conformance spec cannot port to pltrs
(Rust) are listed in `spec/reference/not_applicable.txt`: they test
Python-only machinery (keyword aliases, `data=`, the backend registry's
Python modules, pandas objects, Python warnings) that pltrs (Rust)
expresses with types. Under pltrs (Python), each of them that
matplotlib's own suite runs gives matplotlib's outcome there. Likewise, the probe
waivers where pltrs (Rust) refuses what matplotlib accepts by accident
(negative or fractional polygon sides, undefined path codes, unsorted
boundaries, colours outside 0–1) concern pltrs (Rust)'s API: pltrs
(Python) gives matplotlib's answer there.
