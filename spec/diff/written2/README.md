# The written2 family

The written2 family is a corpus of programs in the style of expert
users. They use the machinery under the charts more than the charts. `cargo xtask diff --corpus written2` runs them under matplotlib
and under pltrs (Python) and compares the results, as for the other
corpora. The corpus is the `[written2]` entry of
`spec/diff/corpora.toml`. The programs are code of this repository (MIT
OR Apache-2.0). For ordinary charts, see `spec/diff/written`.

`<topic>/` holds 40 topics with 400 programs. Examples are custom
artists, blitting without pyplot, transforms and clip paths by hand, and
registries. Others are legend handlers, `TextPath`, the rcParams
machinery, units, `axisartist`, `mplot3d`, documents read back and
custom backends.

Each program follows these rules:

* Its first comment line states its task.
* It uses only deterministic data: formulas, `np.linspace`, a SplitMix64
  in the program and literal tables.
* It reads no clock and uses no network.
* It writes only into its working directory.
* It reads only the files that it wrote and the font files in the data
  directory of matplotlib (`matplotlib.get_data_path()`).
* It makes documents reproducible with `svg.hashsalt` and with
  `metadata={"Date": None}` or `{"CreationDate": None}`.

If a program gives different results in two runs of matplotlib, the
program is wrong. Fix the program, not the harness.
