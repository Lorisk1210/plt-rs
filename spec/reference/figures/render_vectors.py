"""Render every figure case with matplotlib as SVG, PDF and PostScript into a
directory, for plt-spec's opt-in wide comparison of vector output.

    .venv/bin/python spec/reference/figures/render_vectors.py OUTDIR [PREFIX ...]
    PLT_VECTOR_REFERENCE=OUTDIR cargo test -p plt-spec --test figure_vectors \\
        -- --ignored --nocapture

Documents are named ``<group>__<name>.<format>`` and saved like the vector
goldens (``FigureCase.render_vector``) under the reference configuration.
Only cases whose names start with one of the PREFIXes are rendered when any
are given. Nothing is written to the spec itself.
"""

from __future__ import annotations

import argparse
import importlib
import multiprocessing
import pathlib
import sys
import traceback

REFERENCE = pathlib.Path(__file__).resolve().parent.parent
FORMATS = ["svg", "pdf", "ps"]


def _setup() -> None:
    """Import the cases under the reference configuration (in every worker)."""
    sys.path.insert(0, str(REFERENCE))
    import matplotlib

    matplotlib.use("Agg")
    from conftest import reference_rc

    reference_rc()
    for module in sorted((REFERENCE / "figures").glob("cases_*.py")):
        importlib.import_module(f"figures.{module.stem}")


def _render(job: tuple[str, str]) -> list[str]:
    """Writes one case in every format; returns the failures."""
    name, out = job
    from conftest import reference_rc
    from spec_support import all_cases

    case = next(c for c in all_cases() if c.name == name)
    failures = []
    for fmt in FORMATS:
        try:
            reference_rc()
            document = case.render_vector(fmt)
            (pathlib.Path(out) / f"{name.replace('/', '__')}.{fmt}").write_bytes(document)
        except Exception:  # noqa: BLE001 - report every failing case and go on
            failures.append(f"{name} {fmt}: {traceback.format_exc().splitlines()[-1]}")
    return failures


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out", type=pathlib.Path, help="directory to write the documents to")
    parser.add_argument("prefixes", nargs="*", help="render only cases starting with these")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    _setup()
    from spec_support import all_cases

    names = [
        case.name
        for case in all_cases()
        if not args.prefixes or any(case.name.startswith(p) for p in args.prefixes)
    ]
    jobs = [(name, str(args.out)) for name in names]
    with multiprocessing.Pool(initializer=_setup) as pool:
        for failures in pool.imap_unordered(_render, jobs):
            for failure in failures:
                print(failure)
    print(f"rendered {len(names)} cases")


if __name__ == "__main__":
    main()
