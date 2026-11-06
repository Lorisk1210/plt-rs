"""Render every figure case with matplotlib as SVG, PDF, PostScript and PGF into
a directory, for pltrs-spec's opt-in wide comparison of vector output.

    .venv/bin/python spec/reference/figures/render_vectors.py [--formats svg,pdf,ps,pgf] \\
        OUTDIR [PREFIX ...]
    PLTRS_VECTOR_REFERENCE=OUTDIR cargo test -p pltrs-spec --test figure_vectors \\
        -- --ignored --nocapture

Documents are named ``<group>__<name>.<format>`` and saved like the vector
goldens (``FigureCase.render_vector``) under the reference configuration.
matplotlib's PGF backend measures text with LaTeX; here it measures text
as Agg does, as pltrs (Rust) measures it by default (its
``PgfTextMetrics::TextEngine``), so that the comparison needs no TeX, and
includes images with ``\\includegraphics``. PGF images go beside their
documents. Only cases whose names start with one of the PREFIXes are
rendered when any are given. Nothing is written to the spec itself.
"""

from __future__ import annotations

import argparse
import functools
import importlib
import multiprocessing
import pathlib
import sys
import traceback

REFERENCE = pathlib.Path(__file__).resolve().parent.parent
FORMATS = ["svg", "pdf", "ps", "pgf"]


def _setup() -> None:
    """Import the cases under the reference configuration (in every worker)."""
    sys.path.insert(0, str(REFERENCE))
    import matplotlib

    matplotlib.use("Agg")
    from conftest import reference_rc

    reference_rc()
    _measure_pgf_text_like_agg()
    for module in sorted((REFERENCE / "figures").glob("cases_*.py")):
        importlib.import_module(f"figures.{module.stem}")


def _measure_pgf_text_like_agg() -> None:
    """Make the PGF backend work without LaTeX: text metrics from Agg at
    the figure's resolution, images included with ``\\includegraphics``."""
    from matplotlib.backends import backend_pgf
    from matplotlib.backends.backend_agg import RendererAgg

    @functools.lru_cache
    def agg(dpi):
        return RendererAgg(1, 1, dpi)

    def metrics(self, s, prop, ismath):
        return agg(self.dpi).get_text_width_height_descent(s, prop, ismath)

    backend_pgf.RendererPgf.get_text_width_height_descent = metrics
    backend_pgf._get_image_inclusion_command = lambda: r"\includegraphics"


def _render_pgf(case, path: pathlib.Path) -> None:
    """Saves the case as a PGF file (images go beside it)."""
    import matplotlib as mpl
    import matplotlib.pyplot as plt

    with mpl.rc_context(case.rc):
        fig = plt.figure(figsize=case.figsize, dpi=case.dpi)
        try:
            case.build(fig)
            fig.savefig(path, format="pgf", dpi=case.dpi)
        finally:
            plt.close(fig)


def _render(job: tuple[str, str, list[str]]) -> list[str]:
    """Writes one case in the given formats; returns the failures."""
    name, out, formats = job
    from conftest import reference_rc
    from spec_support import all_cases

    from spec_support.tex import document_skip_reason

    case = next(c for c in all_cases() if c.name == name)
    failures = []
    for fmt in formats:
        if reason := document_skip_reason(case, fmt):
            failures.append(f"{name} {fmt}: skipped, {reason}")
            continue
        try:
            reference_rc()
            path = pathlib.Path(out) / f"{name.replace('/', '__')}.{fmt}"
            if fmt == "pgf":
                _render_pgf(case, path)
            else:
                path.write_bytes(case.render_vector(fmt))
        except Exception:  # noqa: BLE001 - report every failing case and go on
            path.unlink(missing_ok=True)  # a partly written document
            failures.append(f"{name} {fmt}: {traceback.format_exc().splitlines()[-1]}")
    return failures


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out", type=pathlib.Path, help="directory to write the documents to")
    parser.add_argument("prefixes", nargs="*", help="render only cases starting with these")
    parser.add_argument("--formats", default=",".join(FORMATS),
                        help="comma-separated formats to write (default: all)")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    _setup()
    from spec_support import all_cases

    names = [
        case.name
        for case in all_cases()
        if not args.prefixes or any(case.name.startswith(p) for p in args.prefixes)
    ]
    formats = args.formats.split(",")
    jobs = [(name, str(args.out), formats) for name in names]
    with multiprocessing.Pool(initializer=_setup) as pool:
        for failures in pool.imap_unordered(_render, jobs):
            for failure in failures:
                print(failure)
    print(f"rendered {len(names)} cases")


if __name__ == "__main__":
    main()
