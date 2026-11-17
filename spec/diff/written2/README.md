# The written2 family

Programs of the kind experts write, working the machinery under the
charts rather than the charts themselves, run by
`cargo xtask diff --corpus written2` (the `[written2]` entry of
`spec/diff/corpora.toml`, a corpus of the repository's own) under
matplotlib and under pltrs (Python), and compared as the other corpora are.
They are this repository's code (MIT OR Apache-2.0), written for pltrs in
task USR2 from the task alone; `spec/diff/written` holds the ordinary
charts of task USR1.

* `<topic>/` (40 topics, 400 programs): custom artists drawing on the
  renderer, canvases and blitting without pyplot, transforms, paths and
  clip paths by hand, registries (scales, projections, colormaps,
  styles, backends), legend handlers and path effects, offset boxes,
  fonts and TextPath, mathtext, events, widgets and animations driven by
  code, the rcParams machinery, units and converters, axes_grid1,
  axisartist and mplot3d in depth, the SVG, PDF and PS documents read
  back, extents and artist trees, copies and pickles, pyplot's
  bookkeeping, old idioms, warnings and deprecations, large inputs,
  collections, images, text layout, contours and triangulations,
  layouts, norms, tickers, dates, vector fields, spines, helper
  libraries and backends of one's own.

Each program states its task in its first comment line, uses only
deterministic data (formulas, `np.linspace`, a SplitMix64 written out,
literal tables), reads no clock and no network, writes only into its
working directory, and reads no file but the ones it wrote and the font
files of matplotlib's own data directory (`matplotlib.get_data_path()`).
Documents are made reproducible the usual way (`svg.hashsalt`,
`metadata={"Date": None}` / `{"CreationDate": None}`). A program
differing between two runs of matplotlib is a mistake of the program:
fix the program, not the harness.
