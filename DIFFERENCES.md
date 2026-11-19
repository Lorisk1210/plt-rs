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
  directories; on Windows `%LOCALAPPDATA%\pltrs`, where matplotlib uses
  `%LOCALAPPDATA%\matplotlib`, or `%USERPROFILE%\.matplotlib` where that
  exists), and its rc file is `pltrsrc`; `MPLCONFIGDIR` and
  `matplotlibrc` files are read too.
* `get_data_path()` is pltrs' data directory in its configuration
  directory, where it writes the fonts and files it bundles, and
  `matplotlib_fname()` names its `pltrsrc` there when you have no rc file
  of your own. Paths built from them (the bundled fonts `findfont()`
  finds, `get_sample_data()`, the `stylelib` directory) are there too, and
  messages naming such a file name it there.
* The `mpl_toolkits` packages live inside pltrs' package, so an
  `ImportError` from them names files there.
* `font_manager.fontManager.ttflist`, `findSystemFonts()` and
  `get_font_names()` list the same fonts in another order (matplotlib's
  comes from a set of paths), so code that picks among equally good fonts
  by walking the list can pick another one; `findfont()` chooses the same
  fonts as matplotlib.
* pltrs does not require fontTools. Where fontTools is installed,
  `backend_ps.fontTools` and `dviread.fontTools` are its modules, as in
  matplotlib; without it, they are missing.
* On Linux with glibc 2.17 (CentOS 7; versions up to 2.27 were not
  measured, 2.28 and later are not affected), a figure saved or drawn
  from inside a callback (a tick formatter or an event handler that saves
  its own figure) leaves less room on the stack than elsewhere. It works,
  but a runaway recursion of that kind is more likely to end the process
  with a crash than with matplotlib's `RecursionError`.

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
  same, and fontTools' "'created' timestamp seems very low" warnings,
  which matplotlib's subsetting gives, do not come.
* A PDF saved with `image.composite_image` off draws its images in
  another order of operators, to the same page.
* `PdfPages.savefig` after `close()` leaves the 16-byte header pltrs
  wrote at its opening, where matplotlib writes a whole page into the
  closed file.
* In SVG and PostScript text a zero can carry the other sign (`-0` where
  matplotlib writes `0`, or the reverse), and the ids matplotlib hashes
  from that text (of markers and clip paths) differ with it. A rasterized
  artist in an SVG can be cropped one pixel column wider. The drawing is
  the same in both cases.
* In PostScript and EPS files, the coordinates of mathtext's rule boxes
  (fraction bars, square-root overlines) can differ from matplotlib's in
  their last digit; the drawing is the same.
* On Windows, an SVG, PostScript or EPS file saved to a path ends its
  lines with `\n`, where matplotlib writes it through a text file whose
  lines end with `\r\n` there. The document is the same otherwise, and
  one written to a file object or a buffer is the same as matplotlib's.
* A `datetime` without a time zone given as a PDF's `CreationDate` (or
  `ModDate`) is written as the moment in UTC (`D:20010203030506Z`), where
  matplotlib writes the clock time with the local offset
  (`D:20010203040506+02'00'`). On Windows such a date before 1970 raises
  `OSError` under pltrs, where matplotlib writes it.
* A pickled figure holds pltrs' own archive of it, of another length
  than matplotlib's. pltrs loads the pickles pltrs writes, but pickles do
  not cross between pltrs and matplotlib in either direction (see "Not
  built").

## Classes and types

* A figure's `artists`, `images`, `lines`, `patches`, `texts` and
  `legends`, and an Axes' `child_axes`, are list subclasses that add and
  remove what you put in or take out. `isinstance(fig.lines, list)` holds
  and they compare equal to matplotlib's lists; `type(fig.lines) is list`
  does not. So are an artist's `sticky_edges.x` and `.y`.
* The classes of matplotlib's C++ modules (`ft2font.FT2Font`, `FT2Image`,
  the `_tri` classes, `BufferRegion`) are Python classes: `pybind11_object`
  is not among their bases. `FT2Font.set_text` with
  `LoadFlags.TARGET_LIGHT` lays the glyphs out with other advances than
  matplotlib's FreeType build (1856 against 1947 for "Hello" at 12 points
  and 72 dpi).
* The drawing methods of pltrs' renderers (`RendererAgg.draw_markers`,
  `draw_path_collection` and the like) are pltrs' own functions, so code
  comparing them with `RendererBase`'s finds other functions.
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
  smaller than one tooth (`roundtooth`); `rasterized=True` among an
  annotation's `arrowprops` (matplotlib raises `AttributeError`; pltrs
  draws the arrow and warns that it ignores the rasterization).
* A categorical axis pickles (matplotlib's holds an `itertools.count`,
  which Python 3.14 refuses to pickle), so code that falls back when the
  pickle fails takes another branch.
* A callback that clears its Axes, or removes their text, while they are
  drawn; a NaN in a constrained layout's `rect`; an Agg canvas that
  overflowed its cell limit once (matplotlib's keeps failing every later
  draw); a box plot of an all-masked array (matplotlib raises
  `IndexError`; pltrs draws).
* The first draw of a 3-D Axes that the figure does not list (made with
  `Axes3D(fig)` and drawn by the program) paints its background in the
  squared box; matplotlib's first draw paints it in the unsquared box
  and only its later draws match.
* A callback that redraws the figure while it is being drawn (a
  formatter, locator, norm or `xlim_changed` callback calling
  `canvas.draw()` or `draw_idle()`): matplotlib paints the figure twice
  into the same image, the second time over a half-finished first;
  pltrs draws it once, as the program's final state says.
* `streamplot` given a norm without limits (`norm=PowerNorm(2)`):
  matplotlib scales the norm to the colour of the first arrow alone, so
  every line and arrow takes one colour and the colorbar spans a sliver;
  pltrs scales it to all the colour values. Under each library the lines
  agree with its colorbar.
* Values matplotlib leaves to chance: the colour of a NaN hue in
  `hsv_to_rgb` (uninitialised memory in matplotlib), the order of NaN
  ticks from `AsinhLocator`. A path with an undefined code fails the
  draw with `ValueError` where matplotlib's `KeyError` comes from a
  table lookup.

## Values pltrs computes otherwise

* Numbers handed to your code are of matplotlib's types, with these
  exceptions: arrays given to a transform's `transform_non_affine`, a
  scale's functions or a norm are contiguous copies where matplotlib
  hands column views (the values and writability are the same); a path
  effect's `draw_path` gets the face colour as four floats where
  matplotlib passes the patch's colour as given; a hexbin's
  `reduce_C_function` gets arrays where matplotlib gives lists when
  `marginals=True`; and `RendererPdf.get_text_width_height_descent` of
  mathtext returns Python floats where matplotlib's are numpy floats.
* Some values differ in their last bits (contour label positions, by one
  unit in the last place), or are the same object under one library and
  an equal copy under the other: the strings and floats an artist was
  given and hands back (a label, a text, a title, line widths, marker
  sizes, an aspect) are equal copies under pltrs, and
  `rcParams["path.effects"]` after `rcParams.update(rcParams.copy())` is
  still the list the program held under pltrs, a new equal list under
  matplotlib.
* In 3-D: `do_3d_projection()` called by a program before any draw
  projects in the current view, where matplotlib projects in the matrix
  `add_patch` left; a `Line3D` subclass' `draw` already sees the coming
  projection in `_x` and `_y`; a `Path3DCollection` made directly reads
  unshaded face colours before its first draw (the picture is the
  same).
* A `PathCollection` made directly, or a `PolyCollection` or
  `LineCollection` given offsets, gets data limits in display pixels in
  both (so the Axes shows little of it), but pltrs takes them at another
  moment of the layout: the limits come out a few pixels apart (x 376.5
  to 4743.5 against 372.7 to 4747.3 for points from 1 to 9).
* `ax.dataLim` of shapes on log axes keeps coordinates the scale cannot
  take, which matplotlib's `get_datalim` drops; the view mostly agrees.
* Read before the first draw, a table cell's text extents differ.
* `get_offsets()` of a scatter given `None` among its points holds `None`
  where matplotlib's holds `nan`; both draw the other points.
* A mapped scatter whose alphas do not fit its points after a draw reads
  its edge colours as its face colours.
* A filled contour set given hatches after it was made reads back, after
  a draw, the face colours of all its bands from `get_facecolor()`, where
  matplotlib's reads back the colour of the band it drew last; the
  figure is the same.
* `ax.gridlines.get_paths()` of an `axisartist` Axes with a curvilinear
  grid is empty; matplotlib's holds the grid lines, which both draw.
* Objects held from before a draw keep the state of their read: a 3-D
  axis label shows the draw's rotation and alignment only once read again
  through `axis.label`, a 3-D tick's marker and line style read back as
  drawn for ticks outside the last draw's view, and a floating axis'
  line path is the one of its first read. The axis of an Axes removed
  from its figure answers no figure (it measures as before).
* An `AuxTransformBox` whose children were added under different aux
  transforms places them all with the last one; matplotlib keeps one per
  child.
* A plain Axes sharing an axis with an Axes whose class keeps sharing
  groups of its own follows that Axes' limits; in matplotlib it does not.
* A text at a position in a unit library's units (astropy, pint) is
  converted when it joins its Axes, where matplotlib converts it when it
  draws. On an Axes whose axes have no units yet, that gives both axes
  the library's converter early, and limits or data given in units
  afterwards no longer set the axes' units (nor the labels they bring):
  give the Axes its data or limits first. A text made by `ax.text` also
  keeps its place when an axis' units are changed later, where
  matplotlib's moves with them. Annotations behave as matplotlib's.

## Values that differ between any two runs

A program that reads the clock, a memory address, a random salt or a
`uuid`, or that iterates a set whose order follows hashing, gives other
values in every run, under matplotlib as under pltrs: the times a program
prints or plots, `id()`s, SVG ids salted without `svg.hashsalt`, HTML ids
of `to_jshtml()`, signal handler numbers of GTK, the last bits of
kiwisolver's solutions, and which of several equally good fonts a
font-lookup warning names. An array made with `np.empty` holds whatever
memory held until the program writes it, so elements it never wrote read
otherwise. The test harness checks each such program by running it twice
under matplotlib.

## Your subclasses and callbacks

pltrs calls the methods your subclasses override (locators, formatters,
transforms, scales, norms, renderers, offset boxes, artists' getters and
`draw`) and the callbacks you set wherever their result matters, and
draws and measures what they return, but it does most of its work
natively and calls them another number of times than matplotlib; the
figure is the same:

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
* A renderer of your own (a `RendererBase` subclass, or methods replaced
  on `RendererAgg`) receives matplotlib's drawing calls method by method,
  but not always with the same graphics-context setters, arguments and
  number of calls (3-D and axis-artist texts come with `mtext=None`, a
  colour bar's solids as single paths).
* Callbacks a program sets on an artist's staleness (`stale_callback`)
  run another number of times, and parts matplotlib keeps no callback
  for (an Axes' background, axis labels) still mark the figure stale.
* An `arc_spine` of a program on a side of a rectilinear Axes is not
  re-spanned over the view when drawn, and a y label is not moved beyond
  the tick-label boxes that a program's `_get_ticklabel_bboxes` gives.

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
  `_bbox`, a polar spine's `_center` and `_width`, a contour set's
  `_process_args`, an axis artist's `_update_ticks`.
* What a 3-D projection leaves on an artist (`_vzs`, `_facecolors2d` and
  the like) is computed when first read: a collection added to an Axes
  that was already drawn reads such values before its own first
  projection, where matplotlib has none yet, and a collection converted
  to 3-D after it was added comes last among the Axes' children.
* `TransformNode._parents` is keyed by `id()`s, and the invalidation
  flags `_invalid` and the `Affine2D._inverted` cache are pltrs' own.

## Warnings and error messages

* For nonsense input (infinite or NaN coordinates, degenerate shapes,
  dates in a 3-D projection), pltrs does not give the `RuntimeWarning`s
  numpy gives when matplotlib computes with them (`invalid value
  encountered`, `divide by zero`, `overflow`; a hexbin of points holding
  `None` or NaN), and where two values are wrong it may complain about
  the other one first. Some of those it gives name another line of your
  program than matplotlib's, or are shown once more. `axvline(np.ma.masked)` warns "converting a masked
  element to nan" once, where matplotlib warns four times.
* pltrs draws some data matplotlib refuses: `None` among `bar` heights
  (matplotlib raises `TypeError`), `datetime64` values whose converter
  the program removed from `matplotlib.units.registry` (matplotlib raises
  numpy's `DTypePromotionError`), patch coordinates of a type that
  converts with `float()` but has no arithmetic, and 3-D state a program
  set by hand that matplotlib's draw refuses (a converted polygon
  collection's `_codes3d` changed before `set_verts`, `_axlim_clip`
  assigned on a contour set).
* A geographic Axes refused by `sharex` (`TypeError`, in both) is left
  unjoined; matplotlib's has joined the groups before it refuses.
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
* on aarch64 Linux, Gouraud-shaded colours, and the last bit of some
  values of numpy's FFT (`mlab.csd`), likely the complex products numpy
  fuses there.

Some messages and warnings differ with the system too: matplotlib's
"building the font cache" warning on a Mac (pltrs reads the system's
fonts in milliseconds), the order of fonts named for a missing glyph, a
file system that ignores case.

## Not built

What does not work at all, each checked by running it:

* **Pickles do not cross** between pltrs and matplotlib: a figure pickled
  by matplotlib does not load under pltrs, nor the other way round.
* **Methods assigned on a 3-D axis or Axes** (`draw_pane`, `draw_grid`,
  `get_proj`) are never called, and `_get_axis_line_edge_points` and
  `axis3d._tick_update_position` are missing.
* **usetex on axis artists**: the labels and tick labels of
  `mpl_toolkits.axisartist` set to usetex are typeset without TeX (and
  draw where TeX is missing, where matplotlib raises).
* **TeX through a PGF renderer called directly**: `RendererPgf.draw_tex`
  runs TeX on the string, where matplotlib's writes it into the picture
  for the document's TeX to typeset (saving a figure as PGF is not
  concerned).
* **`gid`, `url` and `snap` given through an annotation's `arrowprops`**
  are accepted but not kept: the arrow has no id or link in an SVG.
* **Some state changed in place is not drawn changed**: the vertices of
  a step line's path (`line._path.vertices` of a `drawstyle="steps"`
  line), the matrix of `fig.dpi_scale_trans`, the points of an Axes'
  `bbox`, and a colour given as a list or an array (`plot(color=c)`, then
  `c[0] = 1.0`), which pltrs takes when it is given and matplotlib when it
  draws. The arrays artists hand out otherwise are their own, as in
  matplotlib, and changes to them are drawn.
* **Polygon markers** with a number of sides that is not a positive
  integer: `MarkerStyle((4.5, 0))` has 5 corners where matplotlib's has
  6, and `MarkerStyle((-3, 0))` fails where matplotlib makes an empty
  marker.
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

On Windows, a `SOURCE_DATE_EPOCH` the system's clock functions cannot
convert (before 12:00 on 31 December 1969, or after 21:59:59 on 19
January 3001) makes matplotlib's PDF, PostScript and SVG writers raise
the `OSError` of Python's `datetime.fromtimestamp`; pltrs (Rust) writes
the date. pltrs (Python) raises as matplotlib does.
