//! How the numbers were made fair, what is left out, the machine, and how
//! to make the numbers again.

use std::fmt::Write as _;

use super::super::super::guard::{DEVIATION, TOLERANCE};
use super::super::super::machine::Machine;
use super::super::super::results::Results;
use super::super::super::stats::{RESAMPLES, Sampling};
use super::Links;

/// The fairness rules, what is not compared, and the machine.
pub(super) fn write(out: &mut String, results: &Results, links: &Links) {
    out.push_str("## How the comparison is kept fair\n\n");
    same_output(out, links);
    same_work(out, results);
    same_conditions(out, results);
    statistics(out);
    not_compared(out, results);
    machine(out, results);
}

/// The figures are the same, and the guard checks it.
fn same_output(out: &mut String, links: &Links) {
    let _ = write!(
        out,
        "### Same output\n\n\
         Each workload makes one figure. Its Python version is a script for matplotlib in \
         [`benchmarks/python`]({python}). pltrs (Python) runs a copy of that script, in which \
         every `import matplotlib...` reads `import pltrs...`. Nothing else changes. The Rust \
         version of the workload is in [`xtask/src/bench/workloads`]({workloads}). Both \
         versions make the same data with the same seeded generator, SplitMix64. The Python \
         version draws the values with numpy.\n\n\
         All three implementations draw in the matplotlib look with the default rcParams of \
         matplotlib. These set the same figure size, 100 dpi, DejaVu Sans, and the default \
         hinting and antialiasing. pltrs (Python) draws in the modern look by default, so it runs with \
         `PLTRS_LOOK=matplotlib`. The Rust workloads select the matplotlib look in code. \
         matplotlib and pltrs read their configuration from empty directories of the \
         benchmark (`MPLCONFIGDIR`, `PLTRSCONFIGDIR`). Thus no rc file of the user changes a \
         default.\n\n\
         Before the harness times a workload, the [correctness guard](#correctness-guard) \
         makes each implementation save its figure. The guard compares each figure with the \
         figure of matplotlib:\n\n\
         * The PNG pixels must be within an RMS difference of {TOLERANCE} of the pixels of \
         matplotlib, on a scale of 0 to 255. For an animation, every frame must pass this \
         check.\n\
         * An SVG, PDF, PostScript or EPS document must hold the same drawing operations as \
         the document of matplotlib, with every number within {DEVIATION}. The guard reduces \
         both documents to their drawing operations. It resolves every reference, such as \
         glyphs, markers, clip paths, fonts and images. The vector tests of the conformance \
         spec do the same (`pltrs_spec::figures::vector`). The guard ignores ids, object \
         numbers, metadata and compression.\n\n\
         If a figure fails a check, the harness times it but marks it with †. The summary \
         leaves it out.\n\n",
        python = links.python,
        workloads = links.workloads,
    );
}

/// The work done is the same, setting by setting.
fn same_work(out: &mut String, results: &Results) {
    let v = &results.versions;
    let _ = write!(
        out,
        "### Same work\n\n\
         * **Metadata.** All three implementations write their default metadata. PDF, \
         PostScript and SVG documents hold the creator, the producer and the date. PNG files \
         hold the name of the software. Each side makes SVG ids from the default random salt.\n\
         * **PNG compression.** Both libraries compress at the default zlib level 6. They \
         pick the filter of each row adaptively, by the smallest sum of absolute \
         differences. matplotlib saves PNG through Pillow {pillow}, which uses zlib-ng \
         {zlib_ng}. The DEFLATE encoder of pltrs is a port of the zlib-ng 2.3.3 compressor. \
         pltrs also filters and chunks the rows as Pillow does. Thus the same pixels \
         compress to the same bytes. The tables give the size of every document.\n\
         * **PDF compression.** Each side compresses streams at `pdf.compression` 6. \
         matplotlib uses Python's zlib {python_zlib}. pltrs uses its zlib-ng port, which is a \
         little faster than zlib and compresses a little differently.\n\
         * **Paths.** Simplification and snapping keep their defaults on every side. The \
         `output/lissajous_1e5_*` workloads turn off `path.simplify` on every side. Thus each \
         of the 10⁵ segments reaches the document.\n\
         * **Drawing without output.** `layout/constrained_8x8_no_draw` runs \
         `fig.draw_without_rendering()` in Python and `Figure::draw_without_rendering` in \
         Rust. In both, every artist draws to a renderer that writes no output.\n\
         * **Caches.** A warm run must not profit from a cache that only one side keeps. \
         matplotlib keeps one text cache from one figure to the next. This cache holds the \
         last 50 parsed formulas (`MathTextParser._parse_cached`). matplotlib stores its \
         text metrics with the renderer of each figure. The text engine of pltrs (Rust) \
         keeps every line, formula and glyph outline that it laid out. Before every warm \
         run, the harness clears these caches on all three sides, outside the clock. Fonts \
         stay loaded on every side, because matplotlib also keeps its `FT2Font` objects and \
         its font list.\n\n",
        pillow = v.reference("pillow"),
        zlib_ng = v.reference("pillow zlib-ng"),
        python_zlib = v.reference("python zlib"),
    );
}

/// Whether the results were measured on Windows, whose harness spawns
/// and measures processes its own way.
fn on_windows(results: &Results) -> bool {
    results.machine.kernel.starts_with("Windows")
}

/// The kernel as the report names it: Windows' and macOS' are recorded
/// with their names (`Windows NT 10.0.26200`, `Darwin 25.6.0`), Linux's as
/// the bare release.
fn kernel(recorded: &str) -> String {
    if recorded.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Linux {recorded}")
    } else {
        recorded.to_owned()
    }
}

/// The conditions are the same.
fn same_conditions(out: &mut String, results: &Results) {
    let plan = &results.plan;
    let (spawned, peak) = if on_windows(results) {
        (
            "`CreateProcess` to the exit of the child. On Windows, the `python.exe` of a venv \
             is a launcher that runs the interpreter as a second process. The harness starts \
             the interpreter directly instead, and it sets the variable that names the venv, \
             as the launcher does.",
            "Windows reports the peak working set of each cold process \
             (`GetProcessMemoryInfo`).",
        )
    } else {
        (
            "`posix_spawn` to the exit of the child.",
            "The kernel reports the peak resident memory of each cold process (`wait4`).",
        )
    };
    let _ = write!(
        out,
        "### Same conditions\n\n\
         * **Data outside the clock.** Each script makes its data when Python imports it. \
         Each Rust workload makes its data before its first run. In Python, a warm run times \
         `draw()` and `savefig` to memory. In Rust, a warm run times building the figure and \
         saving it to memory. A cold run times the whole program, which includes making the \
         data. numpy makes the data in Python, and plain Rust makes it in pltrs (Rust). Thus \
         this part also differs by language.\n\
         * **Teardown outside the clock.** After the clock stops, Python closes the figure \
         with `plt.close`, and Rust drops it. The garbage collector of Python stays on, as in \
         a user program. It runs a full collection before each warm run, outside the clock. \
         Thus no run pays for the garbage of the run before.\n\
         * **Warm runs in turns.** Each Python column runs its script in a long-lived process \
         (`bench.py serve`). pltrs (Rust) runs in the harness. After one warm-up run each, the \
         three implementations take turns. Each round has one timed run of each \
         implementation, and the order rotates from round to round. Thus a change in the load \
         or clock speed of the machine affects all three equally. {warm}\n\
         * **Cold runs in turns.** Each cold run is a fresh process. Python runs `python \
         script.py out.png`, and pltrs (Rust) runs its executable (`cargo xtask bench --child \
         cold NAME out.png`). The three implementations take turns in the same way. {cold} \
         The harness times a run from {spawned} The scripts and the executable ran before, so \
         the file cache of the operating system and the font caches of the libraries are \
         warm.\n\
         * **One thread.** pltrs draws on one thread, as matplotlib does. The harness sets \
         `OPENBLAS_NUM_THREADS`, `OMP_NUM_THREADS` and `MKL_NUM_THREADS` to 1 in every Python \
         process. Thus the BLAS of numpy uses one thread. {blas_threads} {placement}\n\
         * **Peak memory.** {peak} The tables give the median over the runs.\n\n",
        warm = rounds(&plan.warm),
        cold = rounds(&plan.cold),
        blas_threads = blas_threads(results),
        placement = placement(&results.machine),
    );
}

/// Where the processes run: anywhere, or confined to the fastest of the
/// machine's classes of CPUs (`crate::bench::confine`).
fn placement(machine: &Machine) -> String {
    if machine.confinement.is_none() {
        return "The harness does not pin a process to a CPU.".to_owned();
    }
    format!(
        "The CPUs of this machine run at different speeds, and the operating system can place \
         a new process on any of them. Thus the harness confines itself and every process that \
         it starts to CPUs {cpus}, the fastest class. The row \"CPU confinement\" of [Machine \
         and versions](#machine-and-versions) gives the classes. The confinement is the same \
         for matplotlib, pltrs (Python) and pltrs (Rust). If nothing limits the threads of \
         numpy, numpy starts one thread per confined CPU. Without the confinement, the \
         speed-ups stay the same, but the medians of short cold runs depend on the CPUs that \
         most runs used. On the Framework laptop, a bare interpreter starts in 8.0 to 8.4 ms on \
         a fast core. On a slow core, it starts in 12.4 to 13.2 ms.",
        cpus = machine.affinity
    )
}

/// What numpy's BLAS does with threads when it is left alone.
fn blas_threads(results: &Results) -> &'static str {
    if is_accelerate(results) {
        "Here, the BLAS of numpy is Apple's Accelerate. It starts no threads when Python \
         imports numpy."
    } else {
        "If nothing limits it, OpenBLAS starts one thread per hardware thread when Python \
         imports numpy. This costs both Python columns the same time. [Start-up and \
         imports](#start-up-and-imports) measures it."
    }
}

/// Whether numpy's BLAS is Apple's Accelerate (numpy's macOS wheels);
/// results recorded before the BLAS was were all measured with `OpenBLAS`.
pub(super) fn is_accelerate(results: &Results) -> bool {
    results.versions.reference("numpy blas") == "accelerate"
}

/// How many rounds `sampling` runs a workload, in two sentences.
fn rounds(sampling: &Sampling) -> String {
    format!(
        "Each workload runs at least {} and at most {} rounds. After {} rounds, the harness \
         stops when the slowest implementation has run for {} s in total.",
        sampling.min_runs, sampling.max_runs, sampling.min_runs, sampling.min_seconds
    )
}

/// How the numbers are summarised.
fn statistics(out: &mut String) {
    let _ = write!(
        out,
        "### Statistics\n\n\
         The tables give the median time of each implementation. The brackets give the \
         interquartile range, which holds the middle half of the runs. A speed-up is the \
         median of matplotlib divided by the median of the other implementation.\n\n\
         The 95% confidence interval of a speed-up comes from {RESAMPLES} bootstrap resamples \
         of the rounds. Each resample draws the runs of a round together. A fixed seed makes \
         the same raw data give the same intervals. A workload counts as faster or slower \
         only if the interval excludes 1. Typical speed-ups are geometric means. This average \
         treats 2× faster and 2× slower symmetrically.\n\n\
         ### Claims kept apart\n\n\
         * pltrs (Python) against matplotlib is the drop-in claim. It uses the same script, \
         the same interpreter and the same numpy. Only the imports change.\n\
         * pltrs (Rust) against matplotlib compares the same figure, written natively in each \
         language. It shows how fast pltrs can make the figure. It does not show how much \
         faster a Python program becomes.\n\
         * Cold runs include start-up, which differs by language. The summary gives warm and \
         cold numbers separately. It never averages them together or across claims.\n\n",
    );
}

/// What is left out, and why.
fn not_compared(out: &mut String, results: &Results) {
    out.push_str(
        "## What is not compared\n\n\
         * PGF output. The PGF backend of matplotlib runs LaTeX to measure every text. pltrs \
         lays out PGF with its own font metrics. The tables give the PGF time of pltrs (Rust) \
         without a comparison.\n\
         * Text set by TeX (`text.usetex`). Both libraries run the same external TeX \
         programs, and these programs would take most of the time.\n\
         * Interactive windows. Their frame rate depends on the library, the display server \
         and the screen.\n\
         * GIF and video files of animations. matplotlib gives the frames to the GIF encoder \
         of Pillow or to ffmpeg. These encoders quantize and encode differently from pltrs. \
         Instead, the animation workload renders its frames as PNG into the page of the \
         JavaScript player (`to_jshtml`). This is the same work on every side.\n\
         * The memory of warm runs. The warm runs of pltrs (Rust) share the process of the \
         harness. Only cold processes give a clean peak.\n",
    );
    if results.has_unsupported() {
        out.push_str(
            "* Workloads that pltrs (Python) cannot run yet. Their cells say *not supported \
             yet*. [Not supported yet](#not-supported-yet-by-pltrs-python) lists the exception \
             that each script raised.\n",
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
         {model}\
         | CPU | {cpu} |\n\
         | cores | {cores} cores, {threads} threads |\n\
         | memory | {memory:.0} GiB |\n\
         | operating system | {os}, {kernel} |\n\
         | CPU frequency governor | {governor} |\n\
         {power}\
         | CPUs the processes may run on | {affinity} |\n\
         {confinement}\
         | {load} | {before} at the start, {after} at the end |\n\
         | Rust | {rustc} |\n\n\
         | | matplotlib's venv | pltrs (Python)'s venv |\n|---|---|---|\n",
        model = optional_row("computer", machine.model.as_deref()),
        cpu = machine.cpu,
        cores = machine.cores,
        threads = machine.threads,
        memory = machine.memory_gib,
        os = machine.os,
        kernel = kernel(&machine.kernel),
        load = if on_windows(results) {
            "processor load"
        } else {
            "load average (1, 5, 15 min)"
        },
        governor = machine.governor.as_deref().unwrap_or("not reported"),
        power = optional_row("power settings", machine.power.as_deref()),
        affinity = machine.affinity,
        confinement = optional_row("CPU confinement", machine.confinement.as_deref()),
        before = results.load_before,
        after = results.load_after,
        rustc = machine.rustc,
    );
    for (name, title) in [
        ("python", "Python"),
        ("interpreter", "interpreter"),
        ("numpy", "numpy"),
        ("numpy core", "numpy core library (SHA-256 prefix)"),
        ("numpy blas", "numpy's BLAS"),
        ("pillow", "Pillow"),
        ("pillow zlib-ng", "Pillow's zlib-ng"),
        ("python zlib", "Python's zlib"),
        ("matplotlib", "matplotlib"),
    ] {
        // Results recorded before a version was asked for lack it.
        if v.reference(name) == "unknown" && v.pltrs(name) == "unknown" {
            continue;
        }
        let _ = writeln!(
            out,
            "| {title} | {} | {} |",
            v.reference(name),
            v.pltrs(name)
        );
    }
    out.push_str(
        "\nOther work can share the machine while the benchmark runs. The load row of the \
         machine table shows this. The rounds spread the noise over all three \
         implementations. The intervals show the noise that remains.\n\n",
    );
}

/// A row of the machine's table for a value the machine may not report.
fn optional_row(title: &str, value: Option<&str>) -> String {
    value.map_or_else(String::new, |value| format!("| {title} | {value} |\n"))
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
         The full report takes about an hour. Run it on a machine that does no other work. \
         If you give workload prefixes such as `small/` or `calls/`, `--report` measures only \
         those workloads. `--to DIR` writes the report to another directory.\n\n\
         `--report` writes the copies of the scripts for pltrs to `target/bench/pltrs`. To \
         run one script by hand in matplotlib and in pltrs, use these commands:\n\n\
         ```sh\n\
         MPLBACKEND=agg .venv/bin/python benchmarks/python/line.py line.png\n\
         PYTHONPATH=python PLTRS_LOOK=matplotlib .venv-pltrs/bin/python target/bench/pltrs/line.py line.png\n\
         ```\n\n\
         The guard keeps the renderings that it compared in `target/bench/guard/`. The cold \
         runs keep their documents in `target/bench/cold/`.\n\n\
         If a machine cannot build pltrs, it can run a pack of a build from another machine. \
         `cargo xtask bench --pack DIR` puts these items into `DIR`:\n\n\
         * the executable,\n\
         * the package of pltrs (Python), with its extension built as the wheel builds it,\n\
         * the scripts and their harness.\n\n\
         To use the pack, do these steps:\n\n\
         1. Copy `DIR` to the machine.\n\
         2. On that machine, make the venv `DIR/.venv` with matplotlib and the venv \
         `DIR/.venv-pltrs` without matplotlib.\n\
         3. Run `DIR/xtask bench --report`. It measures as above and names the packed \
         commit.\n\n\
         `DIR/xtask bench [PREFIX...] --against OTHER` times the build in `DIR` against the \
         pack in `OTHER`. It times both faces of pltrs, warm and cold, in turns.\n"
    );
}

#[cfg(test)]
mod tests {
    use super::{Machine, placement};

    #[test]
    fn the_report_says_where_the_processes_ran() {
        let unconfined = Machine::default();
        assert_eq!(
            placement(&unconfined),
            "The harness does not pin a process to a CPU."
        );
        let confined = Machine {
            affinity: "0-3,12-15".to_owned(),
            confinement: Some(
                "8 of the 24 CPUs, those of the fastest class (5157 MHz; the others 3289 MHz)"
                    .to_owned(),
            ),
            ..Machine::default()
        };
        let sentence = placement(&confined);
        assert!(sentence.contains("every process that it starts to CPUs 0-3,12-15"));
        assert!(sentence.contains("8.0 to 8.4 ms"));
    }
}
