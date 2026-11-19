"""
What the tool prints and writes: the table of times, the user's
``times.txt`` lines and the JSON of every run.
"""

from dataclasses import dataclass

from runs import LIBRARIES


@dataclass
class Result:
    """One script in one look: both sides' runs and their pictures."""

    #: The script's file, its name (its file name's stem) and description.
    script: str
    name: str
    description: str
    look: str
    #: `runs.Measured` by library.
    measured: dict
    #: `pictures.compare`'s answer, in the classic look.
    picture: dict | None = None
    #: The clip's file, once written.
    clip: str | None = None

    @property
    def failed(self):
        """Whether either side failed."""
        return any(found.error is not None for found in self.measured.values())

    def ready(self, library):
        """*library*'s median time to the figure ready, or None."""
        found = self.measured[library]
        return None if found.error is not None else found.median()


def heading(runs):
    """The lines over the table: what it times."""
    return [
        f"Seconds from the start stamp to the figure ready (the script's plt.show()), median of {runs} run{"s" if runs != 1 else ""} [fastest, slowest].",
        "The span includes importing the library and running the script; it excludes the interpreter's own start.",
        "Each run is a fresh process; the two libraries take turns, one process at a time.",
    ]


def _seconds(found):
    """A side's cell: its median and range, or that it failed."""
    if found.error is not None:
        return "failed"
    times = found.ready
    return f"{found.median():.2f} [{min(times):.2f}, {max(times):.2f}]"


def _ratio(result):
    """How many times faster pltrs had the figure, or ``-``."""
    matplotlib, pltrs = (result.ready(library) for library in LIBRARIES)
    if matplotlib is None or pltrs is None or min(matplotlib, pltrs) <= 0:
        return "-"
    if pltrs <= matplotlib:
        return f"{matplotlib / pltrs:.2f}x faster"
    return f"{pltrs / matplotlib:.2f}x slower"


def _memory(result):
    """Both sides' median peak resident memory in MiB."""
    cells = []
    for library in LIBRARIES:
        peak = result.measured[library].median("peak_memory")
        cells.append("-" if peak is None else f"{peak / 2**20:.0f}")
    return " / ".join(cells)


def table(results):
    """The table's lines, one row per script and look."""
    rows = [("script", "look", "matplotlib s", "pltrs s", "pltrs", "to the end s", "peak MiB", "picture")]
    for result in results:
        ends = [result.measured[library].median("end") for library in LIBRARIES]
        rows.append((
            result.name,
            result.look,
            *(_seconds(result.measured[library]) for library in LIBRARIES),
            _ratio(result),
            " / ".join("-" if end is None else f"{end:.2f}" for end in ends),
            _memory(result),
            result.picture["verdict"] if result.picture else "",
        ))
    widths = [max(len(row[column]) for row in rows) for column in range(len(rows[0]))]
    lines = ["  ".join(cell.ljust(width) for cell, width in zip(row, widths)).rstrip() for row in rows]
    lines.insert(1, "  ".join("-" * width for width in widths))
    return lines


def times_line(result):
    """The clip's line of ``times.txt``, as the user's project writes it."""
    matplotlib, pltrs = (result.ready(library) for library in LIBRARIES)
    first = "pltrs" if pltrs <= matplotlib else "matplotlib"
    return (f"{result.name}-{result.look}{_suffix(result.clip)}\tmatplotlib {matplotlib:.2f}s"
            f"\tpltrs {pltrs:.2f}s\t{first} first by {abs(matplotlib - pltrs):.2f}s")


def _suffix(path):
    """The suffix of the clip's file."""
    return path[path.rfind("."):] if path else ""


def as_json(results, runs, sides):
    """Every run of every script, for ``--json``."""
    return {
        "runs": runs,
        "measures": heading(runs)[:2],
        "sides": {side.library: {"python": side.python, "sources": side.sources} for side in sides},
        "results": [_result_json(result) for result in results],
    }


def _result_json(result):
    sides = {}
    for library, found in result.measured.items():
        warm = found.warm
        sides[library] = {
            "version": warm.version if warm else None,
            "location": warm.location if warm else None,
            "ready": found.ready,
            "end": [run.end for run in found.runs],
            "peak_memory": [run.peak_memory for run in found.runs],
            "median_ready": found.median(),
            "error": found.error,
        }
    return {
        "script": result.script,
        "name": result.name,
        "description": result.description,
        "look": result.look,
        "sides": sides,
        "picture": result.picture,
        "clip": result.clip,
    }
