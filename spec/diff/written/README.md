# The written family

The written family is a corpus of short programs, each of which makes one
chart in the way a typical user writes it. `cargo xtask diff --corpus
written` runs them under matplotlib and under pltrs (Python) and compares
the results, as for the other corpora. The corpus is the `[written]`
entry of `spec/diff/corpora.toml`. The programs are code of this
repository (MIT OR Apache-2.0).

* `<topic>/` (60 topics, 600 programs): each topic is one user habit.
  Examples are saving and formats, layouts, legends, ticks, dates, chart
  kinds, 3-D, pandas, animations, pickling, and mistakes that warn or
  raise.
* `combo_<theme>/` (30 themes, 300 programs): each program combines
  three or more topics in one figure or session, as an analysis script
  grows. Some programs read values from one artist and give them to the
  next.

Each program follows these rules:

* Its first comment line states its task.
* It uses only deterministic data: formulas, `np.linspace`, seeded
  generators and literal tables.
* It reads no file and no clock.
* It ends with `savefig` calls or with open figures.

If a program gives different results in two runs of matplotlib, the
program is wrong. Fix the program, not the harness.
