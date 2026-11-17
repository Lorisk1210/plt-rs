"""
Sphinx builds under matplotlib and under pltrs, and what differs between
them: the warnings, the files written (by name), the text files and the
PNG images (RMS at most 1, as the figure goldens are held to). Both are
built at one path, so that only the library differs.

Under pltrs the build runs on this interpreter with ``python/alias`` and
``python`` on the path, so that ``import matplotlib`` loads pltrs, as the
``matplotlib`` distribution of pltrs makes it; under matplotlib on the
reference venv's interpreter. Both draw in matplotlib's look
(``PLTRS_LOOK=matplotlib``).
"""

from dataclasses import dataclass
import os
import pathlib
import re
import shutil
import subprocess
import sys
import zipfile

import numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
#: The interpreter of real matplotlib.
REFERENCE_PYTHON = ROOT / ".venv" / "bin" / "python"
#: The highest RMS between two images taken for the same picture.
TOLERANCE = 1.0
#: Files compared by name only: Sphinx's pickled environment and doctrees,
#: its digest of the configuration (which holds the reprs of functions an
#: extension puts there, sphinx-gallery's scrapers), and the documents
#: pltrs writes otherwise than matplotlib (PDF and SVG, whose structure
#: `cargo xtask vectors` compares).
BY_NAME = re.compile(r"(^\.buildinfo|\.(doctree|pickle|pdf|svg))$")
#: The address in an object's default repr, which differs between runs,
#: and the search index's entry of such an address.
ADDRESS = re.compile(r" at 0x[0-9a-f]+")
ADDRESS_TERM = re.compile(r'"0x[0-9a-f]+":(\d+|\[[\d,]*\]),?')
#: A hashed directory of Sphinx's downloads (an MD5 of the file's path).
DOWNLOAD_DIR = re.compile(r"_downloads/[0-9a-f]{32}/")
#: A text file's suffixes.
TEXT = {".html", ".js", ".txt", ".css", ".py", ".rst", ".inv", ".json", ".xml", ""}


def has_sphinx(python):
    """Whether *python* can import Sphinx."""
    found = subprocess.run([str(python), "-c", "import sphinx"], capture_output=True,
                           check=False)
    return found.returncode == 0


@dataclass
class Build:
    """A finished build: its directory, exit status and warnings."""

    directory: pathlib.Path
    returncode: int
    warnings: list

    @property
    def html(self):
        """The HTML output's directory."""
        return self.directory / "_build" / "html"


def build(source, directory, *, pltrs, args=(), python=None, docs=".", env=None):
    """
    Builds the Sphinx project *source* (its documentation in the
    subdirectory *docs*) into *directory*, under pltrs if *pltrs* else
    under matplotlib, with Sphinx's arguments *args*. *python* is the
    interpreter of a venv with the library installed; by default, under
    pltrs this one with pltrs' sources on the path as matplotlib, under
    matplotlib the reference venv's.

    The project is copied to and built in ``build`` beside *directory*,
    then moved there, so that both libraries' builds see one path (pages
    show it in tracebacks, and the search index takes its words).
    """
    built_in = directory.parent / "build"
    for old in (directory, built_in):
        if old.exists():
            shutil.rmtree(old)
    shutil.copytree(source, built_in, ignore=shutil.ignore_patterns("_build", "_doctrees"))
    scratch = built_in / "_scratch"
    (scratch / "tmp").mkdir(parents=True)
    environment = {**os.environ, "PLTRS_LOOK": "matplotlib", "MPLBACKEND": "",
                   "MPLCONFIGDIR": str(scratch), "PLTRSCONFIGDIR": str(scratch),
                   "IPYTHONDIR": str(scratch / "ipython"), "TMPDIR": str(scratch / "tmp"),
                   "SOURCE_DATE_EPOCH": "0", "PYTHONHASHSEED": "0"}
    environment.pop("PYTHONPATH", None)
    if python is None and pltrs:
        path = [str(ROOT / "python" / "alias"), str(ROOT / "python")]
        environment["PYTHONPATH"] = os.pathsep.join(path)
    environment.update(env or {})
    python = python or (sys.executable if pltrs else REFERENCE_PYTHON)
    finished = subprocess.run(
        [str(python), "-m", "sphinx", "-b", "html", "-d", str(built_in / "_doctrees"),
         "-q", str(built_in / docs), str(built_in / "_build" / "html"), *args],
        env=environment, cwd=built_in, capture_output=True, text=True, check=False)
    built_in.rename(directory)
    lines = (finished.stderr + finished.stdout).splitlines()
    return Build(directory, finished.returncode, [line for line in lines if line.strip()])


def files(html):
    """The files under *html*, by their paths relative to it (with the
    hashed directories of downloads named ``<hash>``)."""
    return {DOWNLOAD_DIR.sub("_downloads/<hash>/", path.relative_to(html).as_posix()): path
            for path in sorted(html.rglob("*")) if path.is_file()}


def rms(expected, actual):
    """The RMS between two images of one size, as matplotlib's
    ``calculate_rms`` measures it, or None if their sizes differ."""
    first = np.asarray(Image.open(expected).convert("RGBA"), dtype=float)
    second = np.asarray(Image.open(actual).convert("RGBA"), dtype=float)
    if first.shape != second.shape:
        return None
    return float(np.sqrt(((first - second) ** 2).mean()))


def differences(matplotlib, pltrs):
    """What differs between the builds *matplotlib* and *pltrs*, each
    difference a line."""
    found = []
    if matplotlib.returncode != pltrs.returncode:
        found.append(f"exit status {matplotlib.returncode} under matplotlib, "
                     f"{pltrs.returncode} under pltrs")
    if matplotlib.warnings != pltrs.warnings:
        found.append("warnings differ:\n  matplotlib: " + "\n  ".join(matplotlib.warnings)
                     + "\n  pltrs: " + "\n  ".join(pltrs.warnings))
    expected, actual = files(matplotlib.html), files(pltrs.html)
    found.extend(f"only under matplotlib: {name}" for name in sorted(expected.keys() - actual))
    found.extend(f"only under pltrs: {name}" for name in sorted(actual.keys() - expected))
    for name in sorted(expected.keys() & actual.keys()):
        difference = file_difference(expected[name], actual[name])
        if difference:
            found.append(f"{name}: {difference}")
    return found


def file_difference(expected, actual):
    """How the file *actual* differs from *expected*, or None."""
    if BY_NAME.search(expected.name):
        return None
    if expected.suffix == ".png":
        error = rms(expected, actual)
        if error is None:
            return (f"sizes differ: {Image.open(expected).size} under matplotlib, "
                    f"{Image.open(actual).size} under pltrs")
        return f"RMS {error:.3f}" if error > TOLERANCE else None
    if expected.suffix in TEXT:
        first, second = (ADDRESS_TERM.sub("", ADDRESS.sub(
            " at 0x…", path.read_text(encoding="utf-8", errors="replace")))
            for path in (expected, actual))
        if first == second:
            return None
        lines = zip(first.splitlines(), second.splitlines())
        line, (was, now) = next(
            ((number, pair) for number, pair in enumerate(lines, 1) if pair[0] != pair[1]),
            (0, ("<length>", "<length>")))
        return f"line {line} differs:\n  matplotlib: {was}\n  pltrs: {now}"
    if expected.suffix == ".zip":
        return None if _members(expected) == _members(actual) else "members differ"
    return None if expected.read_bytes() == actual.read_bytes() else "bytes differ"


def _members(archive):
    """The members of the zip file *archive*, by name (without the times
    the files had when it was written)."""
    with zipfile.ZipFile(archive) as opened:
        return {name: opened.read(name) for name in opened.namelist()}
