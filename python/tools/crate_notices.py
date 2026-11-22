"""
Writes, or checks, the licence files each published crate carries.

``cargo package`` packs a crate from its own directory, so each crate on
crates.io must hold the notices of what it contains:

* ``LICENSE-MIT`` and ``LICENSE-APACHE``, copies of pltrs' licence at the
  root (copies, not links: a checkout without symbolic links, Windows'
  by default, would make a link a file naming its target);
* ``THIRD_PARTY.md``: the rows of the root's ``THIRD_PARTY.md`` that name
  the crate (its directory ``crates/<crate>/``, ```<crate>``` or a path
  ``<crate_name>::``), with what the licences ask beside them (FreeType's
  credit, the summary of changes matplotlib's licence asks for);
* ``LICENSES/``: the licence texts those rows cite (```name.txt```).

A crate no row names gets neither. The root's ``THIRD_PARTY.md`` and
``LICENSES/`` are the sources: edit them, then run this tool.

    python python/tools/crate_notices.py           # write the files
    python python/tools/crate_notices.py --check   # list what differs

``--check`` prints each difference and exits with 1 if there is one
(``python/tests/test_crate_notices.py`` runs it).
"""

import pathlib
import re
import sys
import textwrap

ROOT = pathlib.Path(__file__).resolve().parents[2]

#: The crates published on crates.io, in the order they are published.
CRATES = (
    "pltrs-core", "pltrs-codec", "pltrs-font", "pltrs-raster", "pltrs-platform",
    "pltrs-text", "pltrs-render", "pltrs-ui", "pltrs",
)

#: pltrs' licence, at the root and in every published crate.
LICENCES = ("LICENSE-MIT", "LICENSE-APACHE")

#: The line every written ``THIRD_PARTY.md`` starts its text with, which
#: tells a reader where it comes from.
GENERATED = ("<!-- Written by python/tools/crate_notices.py from the repository's "
             "THIRD_PARTY.md: edit that file, then run the tool. -->")

#: A licence text a row cites, as ``(`matplotlib.txt`, "FreeType")``.
CITED = re.compile(r"`([a-z0-9-]+\.txt)`")

#: The paragraph of the root's ``THIRD_PARTY.md`` FreeType's licence asks
#: for, kept with the rows of FreeType's code.
FREETYPE_CREDIT = "The FreeType parts are based in part on the work of the FreeType Team"

#: The paragraph matplotlib's licence asks for (a summary of the changes),
#: kept with every crate that ports matplotlib's code.
MATPLOTLIB_CHANGES = "The matplotlib license requires a summary of the changes"


def _sections(text):
    """The tables of the root's ``THIRD_PARTY.md``, each as ``(heading,
    header lines, rows)``, and its paragraphs outside them."""
    tables, paragraphs = [], []
    heading = None
    for block in text.split("\n\n"):
        lines = block.strip("\n").split("\n")
        if lines[0].startswith("## "):
            heading = lines[0]
            lines = lines[1:]
            if not lines:
                continue
        if lines[0].startswith("|"):
            tables.append((heading, lines[:2], lines[2:]))
        else:
            paragraphs.append("\n".join(lines))
    return tables, paragraphs


def _names(row, crate):
    """Whether the table row *row* concerns the crate *crate*."""
    return (f"crates/{crate}/" in row or f"`crates/{crate}`" in row
            or f"`{crate}`" in row or f"{crate.replace('-', '_')}::" in row)


def notices(root=ROOT):
    """The files of every published crate's notices: ``{path relative to
    root: text}``, with ``None`` for a file a crate must not have."""
    text = (root / "THIRD_PARTY.md").read_text(encoding="utf-8")
    tables, paragraphs = _sections(text)
    available = {path.name for path in (root / "LICENSES").glob("*.txt")}
    files = {}
    for crate in CRATES:
        directory = f"crates/{crate}"
        for licence in LICENCES:
            files[f"{directory}/{licence}"] = (root / licence).read_text(encoding="utf-8")
        kept = [(heading, header, [row for row in rows if _names(row, crate)])
                for heading, header, rows in tables]
        rows = [row for _, _, selected in kept for row in selected]
        cited = sorted({name for row in rows for name in CITED.findall(row)} & available)
        for name in sorted(available):
            files[f"{directory}/LICENSES/{name}"] = (
                (root / "LICENSES" / name).read_text(encoding="utf-8") if name in cited else None)
        files[f"{directory}/THIRD_PARTY.md"] = (
            _crate_notice(crate, kept, paragraphs, cited) if rows else None)
    return files


def _crate_notice(crate, kept, paragraphs, cited):
    """The ``THIRD_PARTY.md`` of *crate*: its rows of the root's tables,
    with the paragraphs their licences ask for."""
    parts = [
        f"# Third-party code and data in {crate}",
        GENERATED,
        textwrap.fill(
            f"{crate} is part of pltrs. pltrs has two licenses, and you can use it "
            "under either one: the MIT license (`LICENSE-MIT`) or the Apache License "
            "2.0 (`LICENSE-APACHE`). The parts in the tables come from other projects "
            "and keep their licenses. The license texts are in `LICENSES/`, or next "
            "to the files that they apply to. The tables are the rows of "
            "`THIRD_PARTY.md` in the pltrs repository that apply to this crate. The "
            "paths are paths in the pltrs repository.", width=72),
    ]
    for heading, header, rows in kept:
        if rows:
            parts += [heading, "\n".join(header + rows)]
            if any(row.startswith("| FreeType") for row in rows):
                parts += [p for p in paragraphs if p.startswith(FREETYPE_CREDIT)]
            if heading == "## Ported code" and "matplotlib.txt" in cited:
                parts += [p for p in paragraphs if p.startswith(MATPLOTLIB_CHANGES)]
    return "\n\n".join(parts) + "\n"


def differences(root=ROOT):
    """What differs between the crates' notices and what the root's
    files make them, one line each."""
    problems = []
    for relative, wanted in notices(root).items():
        path = root / relative
        if wanted is None:
            if path.exists() or path.is_symlink():
                problems.append(f"{relative}: should not exist")
        elif path.is_symlink() or not path.is_file():
            problems.append(f"{relative}: missing (or a link)")
        elif path.read_text(encoding="utf-8") != wanted:
            problems.append(f"{relative}: differs from what the root's files make it")
    return problems


def write(root=ROOT):
    """Writes every crate's notices and removes those a crate must not
    have."""
    for relative, wanted in notices(root).items():
        path = root / relative
        if wanted is None:
            if path.exists() or path.is_symlink():
                path.unlink()
            continue
        if path.is_symlink():
            path.unlink()
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as file:
            file.write(wanted)
    for crate in CRATES:
        licences = root / "crates" / crate / "LICENSES"
        if licences.is_dir() and not any(licences.iterdir()):
            licences.rmdir()


def main(argv):
    if argv == ["--check"]:
        problems = differences()
        for problem in problems:
            print(problem)
        return 1 if problems else 0
    if argv:
        print("usage: crate_notices.py [--check]", file=sys.stderr)
        return 2
    write()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
