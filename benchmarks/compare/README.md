# Compare matplotlib and pltrs on your own scripts

`compare.py` runs matplotlib scripts without changes under matplotlib
and under pltrs (Python). It prints the time until each library has the
figure ready, and tells you if the two pictures are the same. It can
also write a clip that shows the two figures side by side at their
measured times.

For the measured speed of pltrs, read `benchmarks/README.md`. Use this
tool to time your own scripts.

## Run the tool

Install matplotlib in `.venv`. Then run these commands from the
checkout:

```sh
cargo xtask python --release        # once: build pltrs (Python) into python/pltrs
.venv/bin/python benchmarks/compare/compare.py my_plot.py other_plot.py
.venv/bin/python benchmarks/compare/compare.py benchmarks/compare/examples/*.py --video clips/
```

A script imports `matplotlib.pyplot` or `pltrs.pyplot`, draws, and
usually ends with `plt.show()` or `savefig`. `examples/` holds ten such
scripts. Copy one to start your own. The clip shows the first line of
the docstring as the description, or the file name if there is no
docstring.

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

The matplotlib side runs the script as it is. If the script imports
`pltrs`, this side maps `pltrs` and `pltrs.*` to matplotlib.

The pltrs side puts `python/alias` first on `sys.path`, so `import
matplotlib` and `import mpl_toolkits` load pltrs. If the interpreter
can import pltrs, the tool uses that pltrs. Examples are an installed
`pltrs` or `python/` on `PYTHONPATH`. If not, the tool uses `python/` of
the checkout, which needs the built extension. The tool prints which
pltrs it used, and the versions and directories of both libraries.

Both sides run with `MPLBACKEND=agg`, so no window opens. They also run
with `PYTHONIOENCODING=utf-8`, so a script can print any text on
Windows. Each run is a new process in a new scratch directory, with
private configuration and cache directories (`MPLCONFIGDIR`,
`PLTRSCONFIGDIR`). Files that a script saves with relative paths go to
the scratch directory. The tool deletes it after the run.

## What the tool times

The clock starts when the parent process sends a start stamp. At that
time, the child interpreter runs but has not imported anything for the
script. The clock stops when the figure is ready:

* At the first `plt.show()`. The `show` of the tool draws every open
  figure and records the time. Then it copies the picture of each figure
  and returns at once, so the script continues.
* At the end of the script, after the tool draws the open figures, if
  the script never calls `show`.

The time includes the import of the library and of the modules of the
script, such as numpy. It also includes making the data and drawing. It
does not include the start of the interpreter or the copy of the
pictures. The table also gives the time to the end of the script and,
if the system reports it, the peak resident memory of each process.

The two libraries never run at the same time. First, one unmeasured
round fills the font caches. Then the `--runs` rounds follow, and the
library that goes first alternates. The table shows the median and the
range of each side. The clip uses the medians.

The results depend on the machine and its load. Stop other work before
you measure, and compare only runs from the same machine.

## Read the table

```
script    look     matplotlib s       pltrs s            pltrs         to the end s  peak MiB   picture
--------  -------  -----------------  -----------------  ------------  ------------  ---------  ----------------
01_chirp  classic  M.MM [M.MM, M.MM]  P.PP [P.PP, P.PP]  N.NNx faster  M.MM / P.PP   MMM / PPP  the same picture
```

* `matplotlib s` and `pltrs s` give the median seconds until the figure
  is ready. The brackets hold the fastest and the slowest run. If the
  script failed on a side, that side shows `failed`, and the traceback
  is above the table.
* `pltrs` gives how many times faster or slower pltrs was.
* `to the end s` and `peak MiB` give the matplotlib value, then the
  pltrs value.
* `picture` compares the pictures in the classic look only. The value
  "the same picture" means an RMS difference of at most 1 on a scale of
  0 to 255.
  The conformance checks of pltrs use the same limit. If not, the column
  gives the RMS, or the reason that the pictures cannot be compared, for
  example different sizes. The modern look draws a different style on
  purpose, so the tool does not compare its pictures.

If a side of a script fails, the tool exits with status 1. It first
reports the other side and runs the remaining scripts.

## The clip

With `--video DIR`, the tool writes `NAME-classic.mp4` and
`NAME-modern.mp4` for each script, and one line per clip to
`DIR/times.txt`. A clip is 1920×1080 at 30 frames per second. It shows
a clock from the start stamp, the description of the script and a
progress bar with the time of each library. Each figure appears on a
card at its measured time, matplotlib on the left and pltrs on the
right. The title of a card gives the library, its version and its time.
The clip ends with the faster library and the difference.

A card shows the pictures that its library rendered, side by side if
there are several figures. If a script leaves no open figure, the card
shows the last image file that the script saved.

The tool encodes the clips as H.264 at CRF 18 in yuv420p. It uses
`ffmpeg` on `PATH`, or else the `ffmpeg` of the `imageio-ffmpeg` package
(`pip install imageio-ffmpeg`). Without either, it writes an animated
WebP or GIF through Pillow. This file has half the size and half the
frame rate, and the tool tells you. The frames go to the encoder
through a pipe, so only the clips stay on disk.

The background is a plain gradient. The text uses Inter, the font that
pltrs ships. `--background IMAGE` and `--font FILE` replace them. The
tool needs only Pillow in addition to the two libraries, and both
libraries depend on Pillow.

## Limits

The tool times figures that it draws off screen. It does not open
windows, so the time that a GUI toolkit takes to show a window is not
measured.

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
frames under pltrs only.
