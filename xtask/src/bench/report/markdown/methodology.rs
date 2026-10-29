//! How the numbers were made, on what, and how to make them again.

use std::fmt::Write as _;

use super::super::super::guard::TOLERANCE;
use super::super::super::stats::Sampling;
use super::super::Report;

/// The methodology section, the machine included.
pub(super) fn write(out: &mut String, report: &Report) {
    out.push_str("## Methodology\n\n");
    workloads(out, report);
    timing(out, report);
    comparability(out);
    machine(out, report);
}

/// What runs, and in what.
fn workloads(out: &mut String, report: &Report) {
    let versions = &report.versions;
    let _ = write!(
        out,
        "### Workloads\n\n\
         Each workload is one figure saved in one format. Its Python version is a script in \
         [`benchmarks/python`](python), written in matplotlib's syntax (`import \
         matplotlib.pyplot as plt`). Its Rust version is in \
         [`xtask/src/bench/workloads`](../xtask/src/bench/workloads). Both versions make \
         the same data from the same seeded generator (SplitMix64, drawn with numpy in \
         Python) and draw the same figure, which the [correctness guard](#correctness-guard) \
         checks. The workloads fall into four tiers.\n\n\
         * Small, everyday figures are a line plot, a 2 × 2 grid, labelled bars and scatter \
         groups. Start-up and the cost of each call weigh most here.\n\
         * Medium figures have thousands to tens of thousands of points, legends or a \
         colorbar. One four-panel figure is saved in every format.\n\
         * Many small calls means hundreds of `plot`, `scatter`, `bar`, `annotate` or setter \
         calls, one artist each. In pltrs every call crosses from Python into Rust.\n\
         * Large workloads have millions of points, large images and meshes, contours, 3-D \
         axes, heavy text and layout.\n\n\
         The workloads save PNG, SVG, PDF, PostScript and EPS.\n\n\
         ### The three implementations\n\n\
         * matplotlib {mpl} runs on Python {python} with numpy {numpy} and the Agg backend \
         (`MPLBACKEND=agg`, which matplotlib also picks when there is no display).\n\
         * pltrs runs copies of the same scripts in which `import matplotlib.pyplot as plt` \
         reads `import pltrs.pyplot as plt` and nothing else changes. It runs on Python \
         {pltrs_python} with numpy {pltrs_numpy}, with its extension built in release mode \
         from this commit.\n\
         * plt draws the same figures through its Rust API, built in release mode \
         ({rustc}).\n\n\
         All three draw in matplotlib's look with matplotlib's default rcParams (DejaVu \
         Sans, default hinting, 100 dpi). pltrs selects that look when it is imported, and \
         the Rust workloads select it the same way (`plt::look::Appearance::MATPLOTLIB`).\n\n",
        mpl = versions.reference("matplotlib"),
        python = versions.reference("python"),
        numpy = versions.reference("numpy"),
        pltrs_python = versions.pltrs("python"),
        pltrs_numpy = versions.pltrs("numpy"),
        rustc = report.machine.rustc,
    );
}

/// How warm and cold runs are timed, and on how many threads.
fn timing(out: &mut String, report: &Report) {
    let _ = write!(
        out,
        "### Warm runs time the figure alone\n\n\
         The process has made its imports and its data and has drawn the figure once to \
         warm up. A warm run then builds the figure, saves it to memory and closes it. Each \
         workload runs {warm}. Python's garbage collector runs before each run, outside the \
         clock. The tables give the median and the interquartile range, the span of the \
         middle half of the runs. Warm runs profit from what each library keeps from one \
         figure to the next. Both remember the layout of the texts they have measured \
         (matplotlib in `_get_text_metrics_with_cache`, plt-rs in its text engine), which \
         helps text-heavy figures most. Cold runs start without these caches.\n\n\
         ### Cold runs time what a user waits for\n\n\
         Each cold run is a fresh process, `python script.py out.png` or plt's executable \
         (`cargo xtask bench --child cold NAME out.png`). It starts the interpreter or the \
         executable, makes its imports and its data, builds the figure, writes the file and \
         exits. Each workload runs {cold}. The three implementations take turns, so that a \
         change in the machine's load weighs on all of them alike. The harness times a run \
         from `posix_spawn` to the child's exit, and `wait4` reports its peak resident \
         memory. The scripts have run before, so the operating system's file cache and \
         matplotlib's font cache are warm. A cold run is a fresh process, not a freshly \
         booted machine. Making the data is part of a cold run (numpy in Python, Rust in \
         plt), as it is part of a user's script. It takes a few milliseconds for most \
         workloads and a few hundred for the largest (10⁷ normal values for \
         `stats/hist_1e7`).\n\n\
         ### Threads\n\n\
         plt-rs draws on one thread, as matplotlib does. Every Python process holds numpy's \
         BLAS to one thread (`OPENBLAS_NUM_THREADS`, `OMP_NUM_THREADS` and `MKL_NUM_THREADS` \
         set to 1). Left alone, OpenBLAS starts a thread per hardware thread when numpy is \
         imported, which adds the same start-up cost to both Python columns ([Start-up and \
         imports](#start-up-and-imports) measures it). The processes are not pinned to \
         CPUs and may run on CPUs {affinity}.\n\n",
        warm = runs(&report.plan.warm),
        cold = runs(&report.plan.cold),
        affinity = report.machine.affinity,
    );
}

/// How often `sampling` runs a workload, in words.
fn runs(sampling: &Sampling) -> String {
    format!(
        "at least {} times and for at least {} s, at most {} times",
        sampling.min_runs, sampling.min_seconds, sampling.max_runs
    )
}

/// What is left out of the comparison, and where the implementations do
/// the same work by construction.
fn comparability(out: &mut String) {
    out.push_str(
        "### What is not compared\n\n\
         * PGF output. matplotlib's PGF backend runs LaTeX to measure every text, while \
         plt-rs lays PGF out with its own font metrics, so plt's PGF time stands alone.\n\
         * Workloads pltrs cannot run yet. They show *not supported yet*, and [Not supported \
         yet](#not-supported-yet-by-pltrs) lists the exception each script raised.\n\
         * Metadata. plt saves its documents without metadata, so that two builds can be \
         compared byte for byte. matplotlib and pltrs write their few default metadata \
         fields.\n\n\
         ### Where the work is the same by construction\n\n\
         * All three compress PDF streams with DEFLATE at `pdf.compression` 6, and plt-rs' \
         compressor works as zlib's does (hash chains, lazy matching). For a long \
         unsimplified path (`output/lissajous_1e5_pdf`, a 2.3 MB content stream) \
         compression takes about a tenth of a second in either library, most of both \
         times, so the margin there is thin.\n\
         * matplotlib lays a figure out without drawing it (`layout/constrained_8x8_no_draw`) \
         with `fig.draw_without_rendering()`, which runs every artist's draw with output \
         disabled. Its counterpart in plt, `Figure::layout`, runs the layout pass alone. \
         Both leave the same positions, limits and ticks.\n\n",
    );
}

/// The machine and its load.
fn machine(out: &mut String, report: &Report) {
    let machine = &report.machine;
    let _ = write!(
        out,
        "### Machine\n\n\
         | | |\n|---|---|\n\
         | CPU | {cpu} |\n\
         | cores | {cores} cores, {threads} threads |\n\
         | memory | {memory:.0} GiB |\n\
         | operating system | {os}, Linux {kernel} |\n\
         | CPU frequency governor | {governor} |\n\
         | load average (1, 5, 15 min) | {load_before} at the start, {load_after} at the end |\n\n\
         Other work shared the machine while the benchmark ran, as the load average shows. \
         Medians and interquartile ranges resist the occasional slow run, but every number \
         here carries that noise.\n\n",
        cpu = machine.cpu,
        cores = machine.cores,
        threads = machine.threads,
        memory = machine.memory_gib,
        os = machine.os,
        kernel = machine.kernel,
        governor = machine.governor.as_deref().unwrap_or("not reported"),
        load_before = report.load_before,
        load_after = report.load_after,
    );
}

/// How to make the report again, and to run one script by hand.
pub(super) fn reproduce(out: &mut String) {
    let _ = write!(
        out,
        "## Reproducing\n\n\
         ```sh\n\
         cargo xtask python setup               # once: the reference and pltrs venvs\n\
         cargo xtask bench --report             # builds pltrs, measures everything, rewrites this file\n\
         cargo xtask bench --report --runs 10   # more runs of each measurement\n\
         cargo xtask bench [PREFIX...] --reference   # quick: plt against matplotlib, warm\n\
         ```\n\n\
         The full report takes about half an hour. With workload prefixes (`small/`, \
         `calls/`), `--report` measures only those workloads and writes a report of them \
         alone. To run one script by hand in matplotlib and in pltrs (`--report` writes the \
         pltrs copies to `target/bench/pltrs`):\n\n\
         ```sh\n\
         MPLBACKEND=agg .venv/bin/python benchmarks/python/line.py line.png\n\
         PYTHONPATH=python .venv-pltrs/bin/python target/bench/pltrs/line.py line.png\n\
         ```\n\n\
         The guard accepts an RMS difference of up to {TOLERANCE}. The PNGs it compared stay \
         in `target/bench/guard/`, and the documents of the cold runs in \
         `target/bench/cold/`.\n"
    );
}
