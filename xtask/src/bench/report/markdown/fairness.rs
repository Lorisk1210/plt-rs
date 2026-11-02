//! How the numbers were made fair, what is left out, the machine, and how
//! to make the numbers again.

use std::fmt::Write as _;

use super::super::super::guard::{DEVIATION, TOLERANCE};
use super::super::super::results::Results;
use super::super::super::stats::{RESAMPLES, Sampling};

/// The fairness rules, what is not compared, and the machine.
pub(super) fn write(out: &mut String, results: &Results) {
    out.push_str("## How the comparison is kept fair\n\n");
    same_output(out);
    same_work(out, results);
    same_conditions(out, results);
    statistics(out);
    not_compared(out, results);
    machine(out, results);
}

/// The figures are the same, and the guard checks it.
fn same_output(out: &mut String) {
    let _ = write!(
        out,
        "### Same output\n\n\
         Each workload is one figure. Its Python version is a script in \
         [`benchmarks/python`](python) written for matplotlib. pltrs (Python) runs a copy in \
         which every `import matplotlib...` reads `import pltrs...` and nothing else \
         changes. Its Rust version is in \
         [`xtask/src/bench/workloads`](../xtask/src/bench/workloads). Both make the same data \
         from the same seeded generator (SplitMix64, drawn with numpy in Python).\n\n\
         All three draw in matplotlib's look with matplotlib's default rcParams: the same \
         figure size, 100 dpi, DejaVu Sans, matplotlib's default hinting and antialiasing. \
         pltrs (Python), which draws in its own modern look by default, runs with \
         `PLTRS_LOOK=matplotlib`, and the Rust workloads select the same look in code. \
         matplotlib and pltrs read their configuration from empty directories of the \
         benchmark (`MPLCONFIGDIR`, `PLTRSCONFIGDIR`), so no rc file of the user's changes \
         a default.\n\n\
         Before a workload is timed, the [correctness guard](#correctness-guard) has each \
         implementation save its figure and compares it with matplotlib's.\n\n\
         * The pixels of the figure as PNG must be within an RMS difference of {TOLERANCE} \
         (of 0 to 255) of matplotlib's. For an animation, every frame must be.\n\
         * An SVG, PDF, PostScript or EPS document must hold the same drawing operations \
         as matplotlib's, with every number within {DEVIATION}. The comparison reduces \
         both documents to their drawing operations and resolves every reference (glyphs, \
         markers, clip paths, fonts, images), as the conformance spec's vector tests do \
         (`pltrs_spec::figures::vector`). Ids, object numbers, metadata and compression \
         are left out.\n\n\
         A figure that fails a check is timed but marked †, and the summary leaves it \
         out.\n\n",
    );
}

/// The work done is the same, setting by setting.
fn same_work(out: &mut String, results: &Results) {
    let v = &results.versions;
    let _ = write!(
        out,
        "### Same work\n\n\
         * **Metadata.** All three write their default metadata: creator, producer and \
         date in PDF, PostScript and SVG, and the software's name in PNG. SVG ids come \
         from the default random salt on every side.\n\
         * **PNG compression.** Both libraries compress at zlib's default level 6 and pick \
         each row's filter adaptively (the smallest sum of absolute differences). \
         matplotlib saves PNG through Pillow {pillow}, whose zlib is zlib-ng {zlib_ng}. \
         zlib-ng's level 6 is faster than zlib's and compresses a little less. pltrs' \
         encoder works as zlib does, so its files are a few percent smaller and cost it \
         more time. The tables give the size of every document.\n\
         * **PDF compression.** Streams are compressed at `pdf.compression` 6 on every \
         side, by Python's zlib {python_zlib} in matplotlib and by pltrs' own DEFLATE, \
         which works as zlib does (hash chains, lazy matching).\n\
         * **Paths.** Simplification and snapping keep their defaults on every side. The \
         `output/lissajous_1e5_*` workloads turn `path.simplify` off on every side, so \
         that each of the 10⁵ segments reaches the document.\n\
         * **Drawing without output.** `layout/constrained_8x8_no_draw` runs \
         `fig.draw_without_rendering()` in Python and `Figure::draw_without_rendering` in \
         Rust: every artist draws to a renderer that outputs nothing.\n\
         * **Caches.** A warm run must not profit from a cache one side keeps and the other \
         lacks. matplotlib keeps one cache of laid-out text from one figure to the next, \
         its last 50 parsed formulas (`MathTextParser._parse_cached`); its text metrics \
         live with each figure's renderer. pltrs (Rust)'s text engine keeps every line, \
         formula and glyph outline it has laid out. Before every warm run, the harness \
         drops these caches on all three sides (outside the clock). Fonts stay loaded on \
         every side, as matplotlib keeps its `FT2Font` objects and its font list.\n\n",
        pillow = v.reference("pillow"),
        zlib_ng = v.reference("pillow zlib-ng"),
        python_zlib = v.reference("python zlib"),
    );
}

/// The conditions are the same.
fn same_conditions(out: &mut String, results: &Results) {
    let plan = &results.plan;
    let _ = write!(
        out,
        "### Same conditions\n\n\
         * **Data outside the clock.** Each script makes its data when it is imported, and \
         each Rust workload before its first run. A warm run times `draw()` and \
         `savefig` to memory in Python, and building the figure and saving it to memory in \
         Rust. A cold run times the whole program, making its data included: numpy makes \
         it in Python and plain Rust in pltrs (Rust), so that part differs by language \
         too.\n\
         * **Teardown outside the clock.** After a warm run stops the clock, Python closes \
         the figure (`plt.close`) and Rust drops it. Python's garbage collector stays on, as \
         in a user's program, and runs a full collection before each warm run, outside the \
         clock, so that no run pays for the garbage of the one before.\n\
         * **Warm runs in turns.** Each Python column serves its script from a long-lived \
         process (`bench.py serve`) and pltrs (Rust) runs in the harness. After a warm-up \
         run each, they take turns: one timed run each per round, in an order that rotates \
         from round to round. A change in the machine's load or clock speed therefore \
         weighs on all three alike. Each workload runs {warm}.\n\
         * **Cold runs in turns.** Each cold run is a fresh process: `python script.py \
         out.png`, or the executable of pltrs (Rust) (`cargo xtask bench --child cold NAME \
         out.png`). The three take turns the same way, {cold}. The harness times a run from \
         `posix_spawn` to the child's exit. The scripts and the executable have run before, \
         so the operating system's file cache and the libraries' font caches are warm.\n\
         * **One thread.** pltrs draws on one thread, as matplotlib does. Every Python \
         process holds numpy's BLAS to one thread (`OPENBLAS_NUM_THREADS`, \
         `OMP_NUM_THREADS` and `MKL_NUM_THREADS` set to 1). Left alone, OpenBLAS starts a \
         thread per hardware thread when numpy is imported, which costs both Python \
         columns the same ([Start-up and imports](#start-up-and-imports) measures it). \
         No process is pinned to a CPU.\n\
         * **Peak memory.** The kernel reports the peak resident memory of each cold \
         process (`wait4`), and the tables give the median over its runs.\n\n",
        warm = runs(&plan.warm),
        cold = runs(&plan.cold),
    );
}

/// How often `sampling` runs a workload, in words.
fn runs(sampling: &Sampling) -> String {
    format!(
        "for at least {} rounds, and more until the slowest implementation has run for {} \
         s, at most {} rounds",
        sampling.min_runs, sampling.min_seconds, sampling.max_runs
    )
}

/// How the numbers are summarised.
fn statistics(out: &mut String) {
    let _ = write!(
        out,
        "### Statistics\n\n\
         The tables give the median time of each implementation and, in brackets, the \
         interquartile range, which holds the middle half of the runs. A speed-up is \
         matplotlib's median over the other's. Its 95% confidence interval comes from \
         {RESAMPLES} bootstrap resamples of the rounds, the runs of a round drawn \
         together, with a fixed seed so that the same raw data gives the same intervals. A \
         workload counts as faster or slower only when the interval excludes 1. Typical \
         speed-ups are geometric means, the average that treats 2× faster and 2× slower \
         symmetrically.\n\n\
         ### Claims kept apart\n\n\
         * pltrs (Python) against matplotlib is the drop-in claim: the same script, the \
         same interpreter, the same numpy, only the imports changed.\n\
         * pltrs (Rust) against matplotlib compares the same figure written natively in \
         each language. It says how fast the figure can be made, not how fast a Python \
         program becomes.\n\
         * Cold runs include start-up, which differs by language. The summary gives warm \
         and cold numbers apart and never averages them together or across claims.\n\n",
    );
}

/// What is left out, and why.
fn not_compared(out: &mut String, results: &Results) {
    out.push_str(
        "## What is not compared\n\n\
         * PGF output. matplotlib's PGF backend runs LaTeX to measure every text, while \
         pltrs lays PGF out with its own font metrics. The PGF time of pltrs (Rust) stands \
         alone in the tables.\n\
         * Text set by TeX (`text.usetex`). Both libraries run the same external TeX \
         programs, whose time would dominate.\n\
         * Interactive windows. Their frame rate depends on the display server and the \
         screen, not on the library alone.\n\
         * GIF and video files of animations. matplotlib hands frames to Pillow's GIF \
         encoder or to ffmpeg, which quantize and encode differently from pltrs. The \
         animation workload renders its frames as PNG into the page of the JavaScript \
         player instead (`to_jshtml`), the same work on every side.\n\
         * The memory of warm runs. The warm runs of pltrs (Rust) share the harness's \
         process, so only cold processes give a clean peak.\n",
    );
    if results.has_unsupported() {
        out.push_str(
            "* Workloads pltrs (Python) cannot run yet. Their cells read *not supported yet*, \
             and [Not supported yet](#not-supported-yet-by-pltrs-python) lists the exception \
             each script raised.\n",
        );
    }
    out.push('\n');
}

/// The machine, the tools and the load.
fn machine(out: &mut String, results: &Results) {
    let machine = &results.machine;
    let v = &results.versions;
    let _ = write!(
        out,
        "## Machine and versions\n\n\
         | | |\n|---|---|\n\
         | CPU | {cpu} |\n\
         | cores | {cores} cores, {threads} threads |\n\
         | memory | {memory:.0} GiB |\n\
         | operating system | {os}, Linux {kernel} |\n\
         | CPU frequency governor | {governor} |\n\
         | CPUs the processes may run on | {affinity} |\n\
         | load average (1, 5, 15 min) | {before} at the start, {after} at the end |\n\
         | Rust | {rustc} |\n\n\
         | | matplotlib's venv | pltrs (Python)'s venv |\n|---|---|---|\n",
        cpu = machine.cpu,
        cores = machine.cores,
        threads = machine.threads,
        memory = machine.memory_gib,
        os = machine.os,
        kernel = machine.kernel,
        governor = machine.governor.as_deref().unwrap_or("not reported"),
        affinity = machine.affinity,
        before = results.load_before,
        after = results.load_after,
        rustc = machine.rustc,
    );
    for (name, title) in [
        ("python", "Python"),
        ("interpreter", "interpreter"),
        ("numpy", "numpy"),
        ("numpy core", "numpy core library (SHA-256 prefix)"),
        ("pillow", "Pillow"),
        ("pillow zlib-ng", "Pillow's zlib-ng"),
        ("python zlib", "Python's zlib"),
        ("matplotlib", "matplotlib"),
    ] {
        let _ = writeln!(
            out,
            "| {title} | {} | {} |",
            v.reference(name),
            v.pltrs(name)
        );
    }
    out.push_str(
        "\nOther work may share the machine while the benchmark runs, as the load average \
         shows. The rounds spread that noise over all three implementations, and the \
         intervals show what is left of it.\n\n",
    );
}

/// How to make the report again, and to run one script by hand.
pub(super) fn reproduce(out: &mut String) {
    let _ = write!(
        out,
        "## Reproducing\n\n\
         ```sh\n\
         cargo xtask python setup               # once: the reference and pltrs venvs\n\
         cargo xtask bench --report             # builds pltrs, measures everything, rewrites this directory\n\
         cargo xtask bench --report --runs 20   # more rounds of each measurement\n\
         cargo xtask bench --rewrite-report     # the report and charts again from results.json\n\
         cargo xtask bench [PREFIX...] --reference   # quick: pltrs (Rust) against matplotlib, warm\n\
         ```\n\n\
         The full report takes about an hour, best on an otherwise idle machine. With \
         workload prefixes (`small/`, `calls/`), `--report` measures only those workloads, \
         and `--to DIR` writes the report somewhere else. To run one script by hand in \
         matplotlib and in pltrs (`--report` writes the pltrs copies to \
         `target/bench/pltrs`):\n\n\
         ```sh\n\
         MPLBACKEND=agg .venv/bin/python benchmarks/python/line.py line.png\n\
         PYTHONPATH=python PLTRS_LOOK=matplotlib .venv-pltrs/bin/python target/bench/pltrs/line.py line.png\n\
         ```\n\n\
         The guard keeps the renderings it compared in `target/bench/guard/`, and the cold \
         runs their documents in `target/bench/cold/`.\n"
    );
}
