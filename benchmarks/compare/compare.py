"""
Runs Python scripts written in matplotlib's syntax, unchanged, under
matplotlib and under pltrs (Python), and compares the two: the time each
library took to have the script's figure ready, whether both drew the
same picture, and, with ``--video``, a side-by-side clip of the two
figures appearing at their measured times.

    python benchmarks/compare/compare.py SCRIPT.py [SCRIPT.py ...]
        [--runs N] [--look classic|modern|both] [--video DIR] [--json FILE]
        [--background IMAGE] [--font FILE]
        [--matplotlib-python PYTHON] [--pltrs-python PYTHON]

README.md beside this file says exactly what is timed.
"""

import argparse
import ast
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

import encode
import frames
import pictures
import report
import runs

#: The checkout this tool belongs to.
ROOT = pathlib.Path(__file__).resolve().parents[2]
#: The ``matplotlib`` alias of pltrs (Python) and its sources in the checkout.
ALIAS = ROOT / "python" / "alias"
SOURCES = ROOT / "python"
#: The extension's file names, one of which a built checkout holds.
EXTENSIONS = ("_native.abi3.so", "_native.pyd")
#: The looks ``--look both`` runs, in order.
LOOKS = ("classic", "modern")


def describe(script):
    """The first line of *script*'s docstring, else its file name."""
    path = pathlib.Path(script)
    try:
        docstring = ast.get_docstring(ast.parse(path.read_text(encoding="utf-8")))
    except (SyntaxError, UnicodeDecodeError):
        docstring = None
    first = docstring.strip().splitlines()[0].strip() if docstring and docstring.strip() else ""
    return first or path.name


def finds_pltrs(python):
    """Whether *python* imports pltrs from its own environment."""
    probe = "import importlib.util, sys; sys.exit(importlib.util.find_spec('pltrs') is None)"
    return subprocess.run([python, "-c", probe], capture_output=True, check=False).returncode == 0


def checkout_is_built():
    """Whether the checkout's ``python/pltrs`` holds a built extension."""
    return any((SOURCES / "pltrs" / name).is_file() for name in EXTENSIONS)


def interpreter(name):
    """The interpreter *name* as a path the children can run from their
    scratch directories: a relative path made absolute (not resolved, so
    that a venv's link keeps its venv), a bare name looked up on
    ``PATH``."""
    if os.sep in name or (os.altsep and os.altsep in name):
        return os.path.abspath(name)
    found = shutil.which(name)
    if found is None:
        raise SystemExit(f"no interpreter {name!r} on PATH")
    return found


def make_sides(matplotlib_python, pltrs_python):
    """The two `runs.Side` objects, pltrs' from its interpreter's
    environment or else from the checkout's ``python/``."""
    matplotlib_python, pltrs_python = interpreter(matplotlib_python), interpreter(pltrs_python)
    if finds_pltrs(pltrs_python):
        sources = None
    elif checkout_is_built():
        sources = str(SOURCES)
    else:
        raise SystemExit(
            f"{pltrs_python} cannot import pltrs, and the checkout's extension is not built: "
            "run `cargo xtask python --release` or name an interpreter that has pltrs (--pltrs-python)")
    return (runs.Side("matplotlib", matplotlib_python),
            runs.Side("pltrs", pltrs_python, alias=str(ALIAS), sources=sources))


def clip_fonts(side, font, config):
    """The clip's regular and bold font files: *font* for both when the
    user gave one, else those pltrs ships, as its font manager finds them;
    None for Pillow's own font where neither works."""
    if font is not None:
        return font, font
    result = config / "fonts.json"
    command = side.command("fonts", str(result))
    found = subprocess.run(command, env=side.environment("modern", config / side.library),
                           capture_output=True, text=True, check=False)
    if found.returncode != 0 or not result.is_file():
        print("pltrs' fonts were not found; the clip uses Pillow's own font.", file=sys.stderr)
        return None, None
    files = json.loads(result.read_text(encoding="utf-8"))
    return files.get("regular"), files.get("bold")


def write_clip(result, directory, encoder, scene, fonts):
    """Writes *result*'s clip into *directory*; returns its path."""
    sides = []
    for library in runs.LIBRARIES:
        found = result.measured[library]
        sides.append(frames.Side(library, found.warm.version or "", found.median(),
                                 pictures.combined(found.pictures)))
    clip = frames.make_clip(scene, sides, result.description, fonts)
    path = directory / f"{result.name}-{result.look}{encoder.suffix}"
    encoder.write(frames.frames(clip, encoder.fps), path)
    return path


def run_script(script, look, sides, arguments, work):
    """Measures *script* in *look*; returns its `report.Result`."""
    measured = runs.measure(sides, script, look, arguments.runs,
                            work / "runs" / f"{script.stem}-{look}", work / "config")
    result = report.Result(str(script), script.stem, describe(script), look, measured)
    for library, found in measured.items():
        if found.error is not None:
            owner = "pltrs'" if library == "pltrs" else f"{library}'s"
            print(f"\n{script.name} ({look}): {owner} side failed:\n{found.error}", file=sys.stderr)
    if look == "classic" and not result.failed:
        result.picture = pictures.compare(*(measured[library].pictures for library in runs.LIBRARIES))
    return result


def say_sides(sides, results):
    """Prints which libraries ran, their versions and where they came from."""
    for side in sides:
        warm = next((result.measured[side.library].warm for result in results
                     if result.measured[side.library].warm is not None), None)
        version = warm.version if warm and warm.version else "unknown version"
        location = warm.location if warm and warm.location else "not loaded"
        origin = ""
        if side.library == "pltrs":
            origin = ", from the checkout's python/" if side.sources else ", from the interpreter's environment"
        print(f"{side.library} {version} ({location}{origin}), run by {side.python}")


def _arguments():
    parser = argparse.ArgumentParser(
        description="Run matplotlib scripts under matplotlib and pltrs (Python) and compare them.")
    parser.add_argument("scripts", nargs="+", type=pathlib.Path, help="Python scripts in matplotlib's syntax")
    parser.add_argument("--runs", type=int, default=5, help="measured runs per side (default 5)")
    parser.add_argument("--look", choices=(*LOOKS, "both"), default="both",
                        help="pltrs' look: matplotlib's (classic), its own (modern) or both (default)")
    parser.add_argument("--video", type=pathlib.Path, metavar="DIR", help="write a clip per script and look into DIR")
    parser.add_argument("--json", type=pathlib.Path, metavar="FILE", help="write every run's numbers to FILE")
    parser.add_argument("--background", type=pathlib.Path, metavar="IMAGE", help="the clip's background image")
    parser.add_argument("--font", type=pathlib.Path, metavar="FILE", help="the clip's font (TrueType or OpenType)")
    parser.add_argument("--matplotlib-python", default=sys.executable, help="interpreter with matplotlib")
    parser.add_argument("--pltrs-python", default=sys.executable, help="interpreter for pltrs (Python)")
    parser.add_argument("--work", type=pathlib.Path, help="scratch directory (default: a temporary one)")
    arguments = parser.parse_args()
    if arguments.runs < 1:
        parser.error("--runs must be at least 1")
    for script in arguments.scripts:
        if not script.is_file():
            parser.error(f"no such script: {script}")
    return arguments


def main():
    """The command line; returns the exit status."""
    arguments = _arguments()
    sides = make_sides(arguments.matplotlib_python, arguments.pltrs_python)
    looks = LOOKS if arguments.look == "both" else (arguments.look,)
    scripts = [script.resolve() for script in arguments.scripts]
    work = arguments.work.resolve() if arguments.work else pathlib.Path(tempfile.mkdtemp(prefix="pltrs-compare-"))
    try:
        results = []
        for script in scripts:
            for look in looks:
                print(f"{script.name} ({look}): {describe(script)}", flush=True)
                results.append(run_script(script, look, sides, arguments, work))
        print()
        say_sides(sides, results)
        print()
        print("\n".join(report.heading(arguments.runs) + [""] + report.table(results)))
        if arguments.video is not None:
            make_clips(results, sides, arguments, work)
        if arguments.json is not None:
            arguments.json.write_text(json.dumps(report.as_json(results, arguments.runs, sides), indent=1) + "\n",
                                      encoding="utf-8")
    finally:
        if arguments.work is None:
            shutil.rmtree(work, ignore_errors=True)
        else:
            shutil.rmtree(work / "runs", ignore_errors=True)
    return 1 if any(result.failed for result in results) else 0


def make_clips(results, sides, arguments, work):
    """Writes the clip of each result both sides ran, and ``times.txt``."""
    directory = arguments.video
    directory.mkdir(parents=True, exist_ok=True)
    encoder = encode.find_encoder()
    print(f"\nClips by {encoder.name}:")
    regular, bold = clip_fonts(sides[1], arguments.font, work / "config")
    fonts = frames.load_fonts(regular, bold)
    scene = frames.background(arguments.background)
    lines = []
    for result in results:
        if result.failed or not all(result.measured[library].pictures for library in runs.LIBRARIES):
            print(f"  no clip of {result.name} ({result.look}): a side failed or drew nothing")
            continue
        result.clip = str(write_clip(result, directory, encoder, scene, fonts))
        lines.append(report.times_line(result))
        print(f"  {result.clip}", flush=True)
    (directory / "times.txt").write_text("".join(line + "\n" for line in lines), encoding="utf-8")


if __name__ == "__main__":
    sys.exit(main())
