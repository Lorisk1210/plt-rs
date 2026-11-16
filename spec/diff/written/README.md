# The written family

Short programs of the kind users write to get one chart done, run by
`cargo xtask diff --corpus written` (the `[written]` entry of
`spec/diff/corpora.toml`, a corpus of the repository's own) under
matplotlib and under pltrs (Python), and compared as the other corpora are.
They are this repository's code (MIT OR Apache-2.0), written for pltrs in
task USR1 from the task alone.

* `<topic>/` (60 topics, 600 programs): one habit each, as users have them:
  saving and formats, layouts, legends, colour bars, ticks, dates,
  categories and scales, every kind of chart, images, 3-D, projections,
  styles, pandas, events and widgets driven by code, animations,
  pickling, canvas buffers, pyplot's state, and the mistakes that warn or
  raise.
* `combo_<theme>/` (30 themes, 300 programs): three or more topics in one
  figure or session, as an analysis script grows (dashboards from one
  frame, report loops, helper libraries called with options), and values
  read back from one artist and fed to the next.

Each program states its task in its first comment line, uses only
deterministic data (formulas, `np.linspace`, seeded generators, literal
tables), reads no file and no clock, and ends in `savefig` calls or
figures left open. A program differing between two runs of matplotlib is
a mistake of the program: fix the program, not the harness.
