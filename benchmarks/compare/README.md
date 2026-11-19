# Comparing matplotlib and pltrs on your own scripts

`compare.py` takes Python scripts written for matplotlib and runs each
one, unchanged, under matplotlib and under pltrs (Python). It prints how
long each library took to have the script's figure ready, says whether
both drew the same picture, and can write a clip of the two figures
appearing side by side at their measured times.

`benchmarks/README.md` is the measured report of pltrs' speed, with
confidence intervals on a quiet machine. This tool is for your own
scripts: any figure you can draw with matplotlib, timed the way a user
waits for it.

## Three commands

From the checkout, with matplotlib in `.venv` and pltrs (Python) built
into the checkout's `python/`:

```sh
cargo xtask python --release        # once: build pltrs (Python) into python/pltrs
.venv/bin/python benchmarks/compare/compare.py my_plot.py other_plot.py
.venv/bin/python benchmarks/compare/compare.py benchmarks/compare/examples/*.py --video clips/
```

The scripts need nothing special. A script imports `matplotlib.pyplot`
(or `pltrs.pyplot`), draws, and usually ends with `plt.show()` or
`savefig`. `examples/` holds ten such scripts; copy one to start your own.
The description shown in the clip is the first line of the script's
docstring, or its file name if it has none.

Options:

| option | what it does |
|---|---|
| `--runs N` | measured runs per side, 5 by default |
| `--look classic\|modern\|both` | pltrs in matplotlib's look (`PLTRS_LOOK=matplotlib`), in its own modern look, or each script once in each (the default) |
| `--video DIR` | write a clip per script and look, plus `times.txt` |
| `--json FILE` | write every run's numbers |
| `--background IMAGE`, `--font FILE` | the clip's background and font |
| `--matplotlib-python PY`, `--pltrs-python PY` | the interpreters of each side; both default to the one running the tool |
| `--work DIR` | the scratch directory, which keeps its configuration and font caches between invocations; a temporary one by default |

## How each side runs

matplotlib's side runs the script as it is. If the script imports
`pltrs` by name, that side maps `pltrs` and `pltrs.*` to matplotlib's
modules.

pltrs' side puts the checkout's `python/alias` first on `sys.path`, so
the script's `import matplotlib` and `import mpl_toolkits` get pltrs.
pltrs itself comes from the interpreter's environment when it has one
there (an installed `pltrs`, or `python/` on `PYTHONPATH`). Otherwise it
comes from the checkout's `python/`, which needs the extension built. The
tool prints which one it used, with both libraries' versions and
directories.

Both sides run with `MPLBACKEND=agg`, so no window opens, and with
`PYTHONIOENCODING=utf-8`, so that a script may print any text on Windows
too. Each run is a fresh process in a scratch directory of its own, with
configuration and cache directories (`MPLCONFIGDIR`, `PLTRSCONFIGDIR`)
that belong to the tool and never to you. A script that saves files with
relative paths saves them into that scratch directory, which is removed
afterwards.

## What is timed

The clock starts when the parent sends a start stamp, after the child's
interpreter has started and before it has imported anything of the
script's. It stops when the figure is ready:

* At the script's first `plt.show()`. The tool's `show` draws every open
  figure and then stamps the time. After the stamp it keeps a copy of
  each figure's picture and returns without blocking, so the rest of the
  script runs on.
* At the script's end, after its open figures are drawn, if it never
  calls `show`.

The span therefore includes importing the library, importing what the
script imports (numpy, for example), making the data and drawing. It
excludes the interpreter's own start-up and the copying of the pictures.
The table also gives the span to the script's end, and each process's
peak resident memory where the system reports one.

The two libraries never run at the same time. Before the measured runs,
one unmeasured round fills the font caches. Then `--runs` rounds follow,
alternating which library goes first. The table shows the median and the
range of each side's runs; the clip plays the medians.

Timings depend on the machine and on what else it is doing. Close other
work before measuring, and compare runs made on the same machine.

## Reading the table

```
script    look     matplotlib s       pltrs s            pltrs         to the end s  peak MiB   picture
--------  -------  -----------------  -----------------  ------------  ------------  ---------  ----------------
01_chirp  classic  M.MM [M.MM, M.MM]  P.PP [P.PP, P.PP]  N.NNx faster  M.MM / P.PP   MMM / PPP  the same picture
```

* `matplotlib s` and `pltrs s` are the median seconds to the figure
  ready, with the fastest and the slowest run in brackets. A side whose
  script raised says `failed`, and its traceback is printed above the
  table.
* `pltrs` is how many times faster (or slower) pltrs had the figure.
* `to the end s` and `peak MiB` give matplotlib's value, then pltrs'.
* `picture`, in the classic look only, compares the two pictures. "the
  same picture" means the RMS of their difference (over 0 to 255) is at
  most 1, the bound pltrs' own conformance checks use. Otherwise it gives
  the RMS, or says why the pictures cannot be compared (different sizes,
  a different number of figures). In the modern look pltrs draws its own
  style on purpose, so its pictures are not compared.

The tool exits with status 1 if any side of any script failed, after
reporting the other side's time and running the remaining scripts.

## The clip

With `--video DIR`, each script gets `NAME-classic.mp4` and
`NAME-modern.mp4`, and `DIR/times.txt` gets one line per clip. A clip is
1920×1080 at 30 frames a second. A clock runs from the start stamp with
the script's description under it, and a progress bar marks each
library's time. The two figures appear on cards at their measured times,
matplotlib's on the left and pltrs' on the right, each card titled with
the library, its version and its time. The clip ends on which library had
the figure first, and by how much.

The figures on the cards are the pictures each library rendered. With
several figures, they stand side by side on the card. A script that
leaves no figure open shows the last image file it saved.

Clips are encoded with `ffmpeg` if it is on `PATH`, else with the one the
`imageio-ffmpeg` package brings (`pip install imageio-ffmpeg`), as H.264
at CRF 18 in yuv420p. Without either, the tool writes an animated WebP
(or GIF) through Pillow at half the size and frame rate, and says so.
Frames go to the encoder through a pipe, so only the clips are left on
disk.

The background is a plain gradient. The text uses the fonts pltrs ships
(Inter), found through pltrs' font manager. `--background IMAGE` and
`--font FILE` replace them. Pillow is the only package the tool needs
beyond the two libraries, and both libraries already depend on it.

## What is not included

The tool times figures drawn off screen. It does not open or capture real
windows, so the time a GUI toolkit takes to show a window is not part of
the measurement.

## Files

| file | what it holds |
|---|---|
| `compare.py` | the command line: the sides, the runs of each script, the clips |
| `child.py` | the child process that runs one script and stamps its `show` |
| `runs.py` | starting the children one at a time, and what they report |
| `pictures.py` | comparing the two sides' pictures |
| `report.py` | the table, `times.txt` and the JSON |
| `frames.py` | the clip's frames |
| `encode.py` | writing the frames with ffmpeg or Pillow |
| `examples/` | ten scripts in matplotlib's syntax |

`python/tests/test_bench_compare.py` tests the child, the alias and the
frames under pltrs alone.
