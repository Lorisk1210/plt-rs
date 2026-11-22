# Differences from matplotlib

pltrs (Python) runs code written for matplotlib 3.11.2 and gives the same
figures, values and errors. This file lists every known exception, by
topic. Some differences are intended, for example the modern look, the
pltrs windows and the software name in files. Some are matplotlib bugs
that pltrs does not repeat. The section "Not built" lists what pltrs does
not do yet.

The test harness keeps lists of the intended differences that it measures.
Each entry has its proof. The lists are:

* `spec/diff/*allowlist.txt`
* `spec/reference/not_applicable.txt`
* the probe waivers in `crates/pltrs-spec`

Each entry names the section of this file that it belongs to.
`python/tests/test_differences.py` checks this. Thus every difference that
a list allows is in this file.

## The look of figures

* If a program imports `matplotlib` first, pltrs uses the matplotlib look.
  If a program imports `pltrs` first, pltrs uses the modern look. The first
  import in a process decides. A library that imports `matplotlib` later
  does not change the look.
* The modern look is the `pltrs-modern` style. It uses the Inter font,
  constrained layout, a different color cycle and thicker lines. It has no
  top or right spine.
* `PLTRS_LOOK=matplotlib` (or `modern`) in the environment overrides the
  import name. A look that the program chooses in code also overrides it.
* In both looks, `rcdefaults()`, `rcParamsDefault` and
  `style.use("default")` give the matplotlib defaults.
* `style.available` and `style.library` list two more style sheets:
  `pltrs-modern` and `pltrs-modern-dark`. A program that draws one figure
  per style draws two more figures.
* The `pltrs-modern` and `pltrs-modern-dark` sheets ask for semibold
  titles. The bundled DejaVu fonts have no semibold face, so the font
  manager logs a substitution.
* In the matplotlib look, `fontManager.ttflist` does not contain the
  bundled Inter fonts, so font matching does not find them. Their files
  are in the `fonts/ttf` folder of the data directory.
* In the modern look, `fontManager.ttflist` contains the five Inter faces
  in addition to the DejaVu and STIX fonts of matplotlib.
* `rcParams` contains the pltrs-only `pltrs.*` parameters in addition to
  the matplotlib parameters. Their defaults do not change the matplotlib
  look.
* If a display is available, the default window backend is the pltrs
  backend. See "Windows and backends".

## Windows and backends

* If a display is available, the default interactive backend is the pltrs
  windows: `pltrs`, or `macosx` on macOS. pltrs chooses it before Qt, GTK,
  Tk and wx. `plt.get_current_fig_manager().window` is then a pltrs
  window, not a Qt or Tk widget. To get the matplotlib choice, set
  `MPLBACKEND=qtagg` or another backend.
* `backend_registry.list_all()` lists `pltrs` after the matplotlib
  backends. The errors of `use()` and `validate_backend()` that name the
  valid backends also list it.
* The pltrs windows give a program the input events of the matplotlib
  QtAgg backend, with two exceptions. They compose dead keys, and they
  pass Shift+Tab to the canvas. The matplotlib GTK and Tk backends do the
  same.
* On Wayland, a program does not learn where the pointer goes outside its
  window. When the pointer leaves the window, pltrs reports the last
  position of the pointer inside the window.
* `backend_webagg` imports and serves without Tornado. Without Tornado,
  matplotlib raises `RuntimeError`.

## Installation, configuration and data files

* pltrs reads its configuration from `PLTRSCONFIGDIR`. If that variable is
  not set, pltrs uses the XDG directories. On Windows, pltrs uses
  `%LOCALAPPDATA%\pltrs`. There, matplotlib uses
  `%LOCALAPPDATA%\matplotlib`, or `%USERPROFILE%\.matplotlib` if that
  exists.
* The rc file of pltrs is `pltrsrc`. pltrs also reads `MPLCONFIGDIR` and
  `matplotlibrc` files.
* `get_data_path()` returns the pltrs data directory. This directory is in
  the configuration directory. pltrs writes its bundled fonts and files
  into it.
* If you have no rc file, `matplotlib_fname()` returns the `pltrsrc` file
  in the data directory.
* Paths built from the data directory are also there. Examples are the
  bundled fonts that `findfont()` finds, `get_sample_data()` and the
  `stylelib` directory. Messages that name such a file give that path.
* The `mpl_toolkits` packages are inside the pltrs package. Thus an
  `ImportError` from them names files there.
* `font_manager.fontManager.ttflist`, `findSystemFonts()` and
  `get_font_names()` list the same fonts in a different order. matplotlib
  builds its list from a set of paths. Code that walks the list to pick
  among equally good fonts can pick a different font. `findfont()`
  chooses the same fonts as matplotlib.
* pltrs does not require fontTools. If fontTools is installed,
  `backend_ps.fontTools` and `dviread.fontTools` are its modules, as in
  matplotlib. Without fontTools, these attributes are missing.
* On Linux with glibc 2.17 (CentOS 7), a figure saved or drawn inside a
  callback has less room on the stack. An example is a tick formatter or
  an event handler that saves its own figure. The save works. But a
  runaway recursion of that kind can crash the process instead of raising
  `RecursionError` as in matplotlib. glibc 2.28 and later is not
  affected. Versions 2.18 to 2.27 are not measured.

## Files pltrs writes

* PDF, SVG and PGF files name pltrs as the software that wrote them. The
  name is in the PDF `Creator` and `Producer`, in the `dc:title` of the
  SVG `cc:Agent` and in the PGF header.
* PNG files carry a `Software` text chunk that names pltrs.
* pltrs encodes PNG files with its own DEFLATE encoder. The pixels are the
  same, but the file bytes differ. Thus the base64 text of SVG images and
  the SVG image ids hashed from these bytes also differ.
* With `pdf.fonttype` 42, a PDF embeds the same glyphs in a TrueType
  subset that pltrs packs differently.
* pltrs writes WebP files losslessly because it has no lossy WebP
  encoder. matplotlib saves through the lossy default of Pillow. The
  pixels of the pltrs file are the pixels of the canvas. In both
  libraries, the `pil_kwargs` of a program go to Pillow.
* With `ps.fonttype` 42, a PostScript file embeds the whole font.
  matplotlib embeds a subset. The pltrs file is larger and draws the same.
* The font subsetting of matplotlib gives the fontTools warning "'created'
  timestamp seems very low". pltrs does not give this warning.
* A PDF saved with `image.composite_image` off draws its images with a
  different order of operators. The page is the same.
* `PdfPages.savefig` after `close()` leaves only the 16-byte header that
  pltrs wrote when it opened the file. matplotlib writes a whole page into
  the closed file.
* In SVG and PostScript text, a zero can carry the other sign. pltrs can
  write `-0` where matplotlib writes `0`, or the reverse. matplotlib
  hashes the ids of markers and clip paths from that text, so these ids
  also differ. The drawing is the same.
* A rasterized artist in an SVG can be cropped one pixel column wider. The
  drawing is the same.
* In PostScript and EPS files, the coordinates of mathtext rule boxes can
  differ in the last digit. Rule boxes are fraction bars and square-root
  overlines. The drawing is the same.
* On Windows, an SVG, PostScript or EPS file saved to a path ends its
  lines with `\n`. matplotlib writes it through a text file, so its lines
  end with `\r\n` there. The rest of the document is the same. A document
  written to a file object or a buffer is the same as in matplotlib.
* If the `CreationDate` or `ModDate` of a PDF is a `datetime` without a
  time zone, pltrs writes the moment in UTC (`D:20010203030506Z`).
  matplotlib writes the clock time with the local offset
  (`D:20010203040506+02'00'`).
* On Windows, such a date before 1970 raises `OSError` in pltrs.
  matplotlib writes the date.
* A pickled figure holds a pltrs archive of the figure. Its length differs
  from a matplotlib pickle. pltrs loads the pickles that pltrs writes.
  Pickles do not cross between pltrs and matplotlib in either direction.
  See "Not built".

## Classes and types

* These attributes are list subclasses: the `artists`, `images`, `lines`,
  `patches`, `texts` and `legends` of a figure, and the `child_axes` of an
  Axes. If you put an artist in such a list or take one out, pltrs adds
  or removes the artist. `isinstance(fig.lines, list)` is true, and the
  lists compare equal to the matplotlib lists. `type(fig.lines) is list`
  is false.
* The `sticky_edges.x` and `.y` of an artist are also list subclasses.
* The classes of the matplotlib C++ modules are Python classes in pltrs.
  This applies to `ft2font.FT2Font`, `FT2Image`, the `_tri` classes and
  `BufferRegion`. `pybind11_object` is not among their bases.
* `FT2Font.set_text` with `LoadFlags.TARGET_LIGHT` lays out the glyphs
  with different advances than the FreeType build of matplotlib. For
  "Hello" at 12 points and 72 dpi, pltrs gives 1856 and matplotlib gives
  1947.
* The drawing methods of the pltrs renderers are pltrs functions. Examples
  are `RendererAgg.draw_markers` and `draw_path_collection`. Code that
  compares them with the methods of `RendererBase` finds different
  functions.
* pltrs ships the type stubs of matplotlib. A few matplotlib instance
  attributes are properties or descriptors on pltrs classes. Examples are
  the transforms of a figure and an Axes, the `vmin` and `vmax` of a color
  bar, and the `cvalues` of a contour set. pltrs builds these transforms
  on first read.
* Reading and assigning these attributes on an object behaves as in
  matplotlib. But `mypy.stubtest` and a read from the class itself see the
  descriptor.

## Where matplotlib fails and pltrs does not

In these cases, matplotlib 3.11.2 raises, crashes or draws something wrong
because of a bug. pltrs draws or gives an answer.

* `BoxStyle.register`. matplotlib reads a `_Base` class that no longer
  exists.
* An annotation whose arrow starts from a text at a non-finite position.
  matplotlib lets a `StopIteration` escape from `savefig`.
* A negative `FT2Image` size. matplotlib crashes.
* A box style whose box is smaller than one tooth (`roundtooth`).
* `rasterized=True` in the `arrowprops` of an annotation. matplotlib
  raises `AttributeError`. pltrs draws the arrow and warns that it ignores
  the rasterization.
* A categorical axis. pltrs can pickle it. The matplotlib axis holds an
  `itertools.count`, which Python 3.14 refuses to pickle. Thus code that
  falls back when the pickle fails takes a different branch.
* A callback that clears its Axes, or removes their text, while pltrs
  draws them.
* A NaN in the `rect` of a constrained layout.
* An Agg canvas that overflowed its cell limit once. matplotlib keeps
  failing every later draw.
* A box plot of an all-masked array. matplotlib raises `IndexError`.
  pltrs draws the plot.
* The first draw of a 3-D Axes that the figure does not list. An example
  is an Axes made with `Axes3D(fig)` and drawn by the program. pltrs
  paints the background in the squared box. The first draw of matplotlib
  paints it in the unsquared box. Only its later draws are the same.
* A callback that redraws the figure while the figure draws. Examples are
  a formatter, locator, norm or `xlim_changed` callback that calls
  `canvas.draw()` or `draw_idle()`. matplotlib paints the figure twice
  into the same image. The second paint goes over a half-finished first
  paint. pltrs draws the figure once, in the final state of the program.
* `streamplot` with a norm without limits, such as `norm=PowerNorm(2)`.
  matplotlib scales the norm to the color of the first arrow only. Thus
  every line and arrow gets one color, and the colorbar spans a narrow
  range. pltrs scales the norm to all color values. In each library, the
  lines agree with the colorbar.
* A patch that an Axes draws through a curving transform, such as a bar of
  a polar Axes. In pltrs, `contains()` and picking hit the patch where it
  is drawn, up to its arcs. matplotlib splits the outline into paths that
  lose the interpolation steps. Thus matplotlib tests the straight chords
  between the corners. It misses the part of the bar beyond them.
* Values that matplotlib leaves to chance. One example is the color of a
  NaN hue in `hsv_to_rgb`, which comes from uninitialized memory in
  matplotlib. Another example is the order of NaN ticks from
  `AsinhLocator`.
* A path with an undefined code. pltrs fails the draw with `ValueError`.
  In matplotlib, a `KeyError` comes from a table lookup.
* A twin that plots dates, made with `twinx()`. In matplotlib, the twin
  takes over the tick helpers of the shared x axis. A locator that the
  program set, such as `MaxNLocator`, stays. But matplotlib replaces the
  date formatter with one that reads a locator that never ran. Thus every
  label shows the year ("2020").
* With the same twin under constrained layout, the matplotlib shared
  limits take the margins of the twin. They do not take the margins that
  the program set (`margins(x=0)`). pltrs keeps the formatter and the
  margins of the program.
* Coordinates in a transform made from the Axes transform, such as
  `Affine2D() + ax.transAxes`. pltrs does not take them as data. While
  the view has its default limits, matplotlib autoscales to them.
  matplotlib takes the transform for `transData` because the matrix is
  the same.
* An Axes whose twin was removed. pltrs can adjust its box to an aspect.
  matplotlib keeps refusing it, as for a twinned Axes.

## Values pltrs computes otherwise

* The numbers that pltrs gives to your code have the matplotlib types.
  The next four items are the exceptions.
* A transform's `transform_non_affine`, the functions of a scale, and a
  norm receive contiguous copies of arrays. matplotlib gives them column
  views. The values and the writability are the same.
* The `draw_path` method of a path effect receives the face color as four
  floats. matplotlib passes the color of the patch as the program gave it.
* With `marginals=True`, the `reduce_C_function` of a hexbin receives
  arrays. matplotlib gives it lists.
* For mathtext, `RendererPdf.get_text_width_height_descent` returns
  Python floats. matplotlib returns numpy floats.
* Some values differ in their last bits. For example, contour label
  positions can differ by one unit in the last place.
* Some values are the same object in one library and an equal copy in the
  other. Strings and floats that an artist receives and returns are equal
  copies in pltrs. Examples are a label, a text, a title, line widths,
  marker sizes and an aspect.
* After `rcParams.update(rcParams.copy())`, `rcParams["path.effects"]` is
  still the list that the program held in pltrs. In matplotlib, it is a
  new equal list.
* The data limits of a patch in a blended transform (`axvspan`, `axhspan`)
  are its coordinates as given. matplotlib converts them to display
  coordinates and back, so its limits can differ in the last bit. This
  can move a tick that falls on half a pixel to the next pixel.
* If a program calls `do_3d_projection()` before any draw, pltrs projects
  in the current view. matplotlib projects in the matrix that `add_patch`
  left.
* The `draw` method of a `Line3D` subclass already sees the coming
  projection in `_x` and `_y`.
* A `Path3DCollection` made directly reads unshaded face colors before its
  first draw. The picture is the same.
* Some collections get data limits in display pixels in both libraries.
  These are a `PathCollection` made directly, and a `PolyCollection` or
  `LineCollection` with offsets. Thus the Axes shows little of the
  collection.
* For such collections, pltrs takes the limits at a different moment of
  the layout, so they differ by a few pixels. For points from 1 to 9,
  pltrs gives x from 376.5 to 4743.5. matplotlib gives 372.7 to 4747.3.
* On log axes, `ax.dataLim` of shapes keeps coordinates that the scale
  cannot take. The `get_datalim` of matplotlib drops them. The view is
  mostly the same.
* Before the first draw, the text extents of a table cell differ.
* The `arrow_patch.get_window_extent()` of an annotation differs before
  the first draw of the annotation. It also differs after a move without
  a new draw. pltrs returns the place where it will draw the arrow.
  matplotlib returns the arrow as it was last placed. Before any draw,
  that is its unplaced path from (0, 0) to (1, 1).
* If a scatter gets `None` among its points, its `get_offsets()` holds
  `None` there. matplotlib holds `nan`. Both draw the other points.
* A scatter read back from a pickle holds NaN under the mask of its
  offsets. matplotlib keeps the values that the mask hides.
* A color-mapped scatter can have alphas that do not fit its points. After
  a draw, its edge colors read back as its face colors.
* A filled contour set can get hatches after it was made. After a draw,
  its `get_facecolor()` returns the face colors of all its bands.
  matplotlib returns the color of the band that it drew last. The figure
  is the same.
* In an `axisartist` Axes with a curvilinear grid,
  `ax.gridlines.get_paths()` is empty. matplotlib returns the grid lines.
  Both libraries draw the grid lines.
* Objects that a program got before a draw keep the state they had when
  the program read them. For example, a 3-D axis label shows the rotation
  and alignment of the draw only after the program reads it again through
  `axis.label`.
* For ticks outside the view of the last draw, the marker and line style
  of a 3-D tick read back as drawn.
* The line path of a floating axis stays the path of its first read.
* The axis of an Axes that was removed from its figure returns no figure.
  It still measures as before.
* An `AuxTransformBox` can get children under different aux transforms.
  pltrs places all of them with the last transform. matplotlib keeps one
  transform per child.
* Some Axes classes keep separate sharing groups. A plain Axes that shares
  an axis with such an Axes follows the limits of that Axes. In
  matplotlib, it does not.
* A text can have a position in the units of a unit library (astropy,
  pint). pltrs converts the position when the text joins its Axes.
  matplotlib converts it when it draws.
* If the axes of the Axes have no units yet, that early conversion gives
  both axes the converter of the library. Data or limits given in units
  later then do not set the units of the axes or their labels. Give the
  Axes its data or limits first.
* A text made by `ax.text` keeps its place if the units of an axis change
  later. In matplotlib, the text moves with them. Annotations behave as in
  matplotlib.

## Values that differ between any two runs

Some programs give different values in every run, in matplotlib and in
pltrs. This happens when a program reads the clock, a memory address, a
random salt or a `uuid`. It also happens when a program iterates a set
whose order follows hashing. Examples:

* the times that a program prints or plots
* `id()` values
* SVG ids salted without `svg.hashsalt`
* the HTML ids of `to_jshtml()`
* the signal handler numbers of GTK
* the last bits of kiwisolver solutions
* the font that a font-lookup warning names, among several equally good
  fonts

An array made with `np.empty` holds old memory until the program writes
it. Elements that the program never writes can differ. The test harness
checks each such program by running it twice in matplotlib.

## Your subclasses and callbacks

pltrs calls the methods that your subclasses override and the callbacks
that you set, where their result matters. Such methods are on locators,
formatters, transforms, scales, norms, renderers and offset boxes, and on
the getters and `draw` of artists. pltrs draws and measures what they
return. But pltrs does most of its work natively. Thus it calls them a
different number of times than matplotlib. The figure is the same.

* pltrs measures an offset box of yours once per layout if it is inside a
  box that pltrs lays out. matplotlib asks again in `get_bbox`,
  `get_offset` and `draw` of each parent.
* pltrs asks the axis transforms of a projection once per side, not once
  per tick.
* pltrs calls overrides of these members more or fewer times: private
  members (`_set_lim_and_transforms`, `_update_transScale`, ...), getters
  such as `get_transform` or `get_path`, `pchanged`,
  `RendererBase.new_gc` and `PathCollection.__init__`. `scatter` does not
  call `PathCollection.__init__`.
* Some overrides receive floats where matplotlib passes the ints that you
  gave. These are `Locator.nonsingular` and `limit_range_for_scale`.
* A `Figure.draw` that never calls the base class still prints the figure.
  matplotlib prints a blank canvas.
* A `FancyArrowPatch.draw` can never call the base class. Then matplotlib
  keeps its first dpi correction of 1. pltrs takes the dpi correction at
  the last draw of the figure.
* If the `transform_affine` of your transform class disagrees with its
  `get_matrix`, pltrs maps all points through the matrix.
* A renderer of yours receives the matplotlib drawing calls method by
  method. Such a renderer is a `RendererBase` subclass, or methods
  replaced on `RendererAgg`.
* The graphics-context setters, the arguments and the number of these
  calls are not always the same. For example, 3-D and axis-artist texts
  come with `mtext=None`. The solids of a color bar come as single paths.
* Callbacks that a program sets on the staleness of an artist
  (`stale_callback`) run a different number of times.
* Some parts have no stale callback in matplotlib, for example the
  background of an Axes and the axis labels. In pltrs, they still mark the
  figure stale.
* pltrs does not re-span an `arc_spine` of a program over the view when it
  draws the spine on a side of a rectilinear Axes.
* pltrs does not move a y label beyond the tick-label boxes that the
  `_get_ticklabel_bboxes` of a program returns.

These libraries override these members and draw the same figures in both
libraries: MetPy, mpltern, ultraplot, cartopy, mne, arviz, probscale,
mplcursors and pytransform3d.

## Private attributes

Other libraries read or write some private names of matplotlib. These
names work as in matplotlib, with these exceptions:

* Some values that pltrs computes at every layout read differently before
  the first draw:
  * the `_position` of an Axes
  * the `_points` of boxes
  * the `_paths` of a mesh
  * the `_posA_posB` of an annotation arrow
  * the `_facecolor3d` of a surface
* Some holders are pltrs classes with the protocol of the matplotlib
  class:
  * `_shared_axes`, `_twinned_axes` and `_align_label_groups` have the
    protocol of `cbook.Grouper`.
  * The `_cells` of a table have the protocol of a dict.
  * The `_children` of an Axes have the protocol of a list.
* Some integers that matplotlib makes are floats in pltrs. These are
  `_loc`, `_base` and the center of a geographic Axes.
* `_tight` is False where matplotlib keeps None.
* `_renderer`, `_clippath`, `_locs` and `_remove_method` have different
  types.
* pltrs does not draw from these assignments:
  * `Axes3D._dist`. Thus the `distance` option of holoviews has no effect.
  * The `_values` of a color bar. The assignment fails. Pass `values=` to
    `colorbar` instead.
  * The `_fontsize` of a legend.
  * A path assigned to the outline spine of a color bar or to the `_path`
    of a line.
  * The `_transform` of a figure.
* A pickle does not carry these assignments: `Legend._offset`,
  `AutoDateLocator._freq`, the `_nrows` and `_ncols` of a `GridSpec`, and
  the `_format` of a formatter.
* Some helpers have no state in pltrs and are missing:
  * the `_bbox` of a fill-between collection
  * the `_center` and `_width` of a polar spine
  * the `_process_args` of a contour set
  * the `_update_ticks` of an axis artist
* pltrs computes the values that a 3-D projection leaves on an artist
  when they are first read. Examples are `_vzs` and `_facecolors2d`.
* A collection added to an Axes that was already drawn reads such values
  before its own first projection. In matplotlib, it has no such values
  yet.
* A collection converted to 3-D after it was added comes last among the
  children of the Axes.
* `TransformNode._parents` is keyed by `id()` values.
* The invalidation flags `_invalid` and the `Affine2D._inverted` cache are
  pltrs implementations.

## Warnings and error messages

* pltrs does not give the `RuntimeWarning`s of numpy for nonsense input.
  matplotlib gives them when it computes with such input. The warnings
  are `invalid value encountered`, `divide by zero` and `overflow`.
* Nonsense input includes infinite or NaN coordinates, degenerate shapes,
  dates in a 3-D projection and a hexbin of points that hold `None` or
  NaN.
* If two values are wrong, pltrs can complain about the other value
  first.
* Some numpy warnings that pltrs gives name a different line of your
  program than matplotlib. Some show once more.
* `axvline(np.ma.masked)` warns "converting a masked element to nan" once.
  matplotlib warns four times.
* pltrs draws some data that matplotlib refuses:
  * `None` among `bar` heights. matplotlib raises `TypeError`.
  * `datetime64` values whose converter the program removed from
    `matplotlib.units.registry`. matplotlib raises the numpy
    `DTypePromotionError`.
  * Patch coordinates of a type that converts with `float()` but has no
    arithmetic.
  * 3-D state that a program set by hand and that the matplotlib draw
    refuses. One example is the `_codes3d` of a converted polygon
    collection, changed before `set_verts`. Another example is
    `_axlim_clip` assigned on a contour set.
* `sharex` refuses a geographic Axes with `TypeError` in both libraries.
  pltrs leaves the Axes unjoined. matplotlib joins the groups before it
  refuses.
* pltrs takes `set_resample("False")` as true. The matplotlib draw
  refuses the string.
* Without TeX, a usetex figure fails with the matplotlib `RuntimeError`.
  But an earlier run can leave typeset texts in the cache. Then the
  `FileNotFoundError` for the missing font metrics can name a different
  font.
* Some programs log the "findfont: Failed to find font weight ..."
  warnings a different number of times. pltrs looks up math and fallback
  fonts in a different order. The fonts drawn are the same.
* An Axes can refuse its settings while pltrs autoscales its view or
  applies its aspect. Examples are `adjustable='box'` on a twinned Axes
  and a box aspect that is not positive.
* The next draw or save raises such an error. `apply_aspect`,
  `get_position` and `get_tightbbox` of the Axes also raise it.
  `Figure.tight_layout` and `Figure.get_tightbbox` do not raise it. In
  matplotlib, they raise it too.
* An autoscale can make limits NaN. An example is a logit axis over a
  hexbin whose smallest positive datum is above one. pltrs refuses these
  limits when it draws the figure. matplotlib already refuses them in
  `set_yscale` and `autoscale_view`.
* A log axis can have no positive limit in its view. An example is a
  collection that `relim` does not see, such as `hlines`, followed by
  `relim()`, `autoscale_view()` and `set_yscale('log')`.
* For such a log axis, pltrs fails with "Data cannot be log-scaled because
  all values are <= 0.". matplotlib draws a log axis over limits of
  ±0.055. It takes the ticks from the smallest positive datum that its
  `relim` left.

## Numbers on other machines

pltrs computes as the numpy and matplotlib wheels compute on the machine
where it runs. It follows the processor level of numpy, which sets the
order in which `argsort` leaves ties. It follows fused multiply-adds on
arm64 and Apple Accelerate.

pltrs does not follow the items below. They change only last bits or
ill-conditioned results.

* The OpenBLAS kernels of numpy, which differ by processor. They affect
  long sums and LAPACK.
* The fused multiply-adds of these kernels in the matplotlib products of
  transforms. They can give rounding noise of the other sign. An example
  is two rotations by 45° on Windows, where the system `sin(pi/4)` equals
  its `cos(pi/4)`.
* The SVML math functions of numpy on Linux with AVX-512. Only pltrs
  (Rust) differs here.
* On Apple silicon, the Accelerate products of shapes other than 3×3 and
  4×4, and the Accelerate LAPACK.
* On Apple silicon, the sign of rounding noise in the numpy FFT.
* On arm64, the way Agg handles infinitely wide strokes. matplotlib fails
  with "Exceeded cell block limit". pltrs leaves such corners out.
* On arm64, the saturating conversion of infinite dates in numpy.
* On aarch64 Linux, Gouraud-shaded colors.
* On aarch64 Linux, the last bit of some values of the numpy FFT
  (`mlab.csd`). The likely cause is that numpy fuses the complex products
  there.
* With numpy 1.26, the decades below one of a log locator, in tick and
  contour level values. matplotlib gets them from the array power of
  numpy 1, which gives 10⁻¹ as 0.09999999999999999. pltrs gives them as
  numpy 2 does (0.1).

Some messages and warnings also differ with the system:

* On a Mac, matplotlib warns that it builds the font cache. pltrs reads
  the system fonts in milliseconds.
* The order of the fonts that a message names for a missing glyph can
  differ.
* Messages can differ on a file system that ignores case.

## Not built

These features do not work. The test harness runs each one to check it.

* Pickles do not cross between pltrs and matplotlib. A figure that
  matplotlib pickled does not load in pltrs. A figure that pltrs pickled
  does not load in matplotlib.
* pltrs never calls methods assigned on a 3-D axis or a 3-D Axes. Examples
  are `draw_pane`, `draw_grid` and `get_proj`.
* `_get_axis_line_edge_points` and `axis3d._tick_update_position` are
  missing.
* pltrs typesets the usetex labels and tick labels of
  `mpl_toolkits.axisartist` without TeX. Thus they draw where TeX is
  missing. matplotlib raises there.
* If a program calls `RendererPgf.draw_tex` directly, pltrs runs TeX on
  the string. matplotlib writes the string into the picture, and the TeX
  of the document typesets it. Saving a figure as PGF is not affected.
* pltrs accepts `gid`, `url` and `snap` in the `arrowprops` of an
  annotation, but it does not keep them. In an SVG, the arrow has no id
  or link.
* pltrs does not draw some state that a program changes in place:
  * the vertices of the path of a step line (`line._path.vertices` of a
    `drawstyle="steps"` line)
  * the matrix of `fig.dpi_scale_trans`
  * the points of the `bbox` of an Axes
  * a color given as a list or an array (`plot(color=c)`, then
    `c[0] = 1.0`)
* pltrs reads such a color when the artist gets it. matplotlib reads it
  when it draws.
* Other arrays that artists return belong to the artist, as in
  matplotlib. pltrs draws changes to them.
* Polygon markers whose number of sides is not a positive integer differ.
  `MarkerStyle((4.5, 0))` has 5 corners in pltrs and 6 in matplotlib.
  `MarkerStyle((-3, 0))` fails in pltrs. matplotlib makes an empty marker.
* On musl (Alpine), a draw that starts inside another draw raises
  `RecursionError` if it runs in a thread other than the main thread. An
  example is a callback that saves its figure.

## pltrs (Rust)

The conformance spec cannot port some matplotlib tests to pltrs (Rust).
`spec/reference/not_applicable.txt` lists them. These tests check
Python-only machinery, such as keyword aliases, `data=`, the Python modules
of the backend registry, pandas objects and Python warnings. pltrs (Rust)
expresses these with types. In pltrs (Python), each such test that the
matplotlib suite runs gives the matplotlib outcome.

Some probe waivers cover input that pltrs (Rust) refuses and matplotlib
accepts by accident. Examples are negative or fractional polygon sides,
undefined path codes, unsorted boundaries and colors outside 0 to 1.
These waivers concern the pltrs (Rust) API. pltrs (Python) gives the
matplotlib answer there.

On Windows, the clock functions of the system cannot convert some values
of `SOURCE_DATE_EPOCH`. These values are before 12:00 on 31 December 1969
or after 21:59:59 on 19 January 3001. For these values, the matplotlib
PDF, PostScript and SVG writers raise the `OSError` of
`datetime.fromtimestamp`. pltrs (Rust) writes the date. pltrs (Python)
raises the same error as matplotlib.
