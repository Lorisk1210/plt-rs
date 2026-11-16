"""
Programs that open a toolkit's window: the gallery's embedding examples
and those running pyplot under a GUI backend, listed in `LIST`, compared
as any program is once their windows drew (``gui_shim.py``).

Each run of such a program gets an X server of its own (an Xvfb started
for it and stopped after it, ``python/tests/gui_events/xdisplay.py``),
never a display the tester did not start, and the toolkits
``python/tools/toolkits.sh install`` puts in ``~/.local/gtk`` (GTK 3 and 4,
PyGObject, wxPython; the Qt bindings and Tk are the venvs' own). A run
lacking either skips the listed programs, saying why (`Toolkits.lacking`).

`LIST` holds one program per line, by its name in the report, followed by
the variables its run sets (``MPLBACKEND=tkagg``); a name may end in
``@VARIANT`` to run the same program again with other variables (mplcvd
under each of the toolkits it supports).
"""

from __future__ import annotations

import contextlib
import os
import re
import pathlib
import shutil
import subprocess
import sys
from collections.abc import Iterator

HERE = pathlib.Path(__file__).resolve().parent
#: The listed programs and the variables of their runs.
LIST = HERE / "gui.txt"
#: The toolkits' installer, which also prints their variables.
TOOLKITS = HERE.parents[1] / "python" / "tools" / "toolkits.sh"
#: Seconds a listed program may run under one implementation: its windows
#: draw and go quiet, else the shim ends its loop after
#: ``gui_shim.PATIENCE``.
TIMEOUT = 120
#: The locale of both runs: the toolkits set the process' locale from the
#: environment, which would format numbers with the host's grouping.
LOCALE = "C.UTF-8"
#: A word of a line of `LIST` setting a variable.
_VARIABLE = re.compile(r"[A-Z_][A-Z0-9_]*=\S*")
#: The variable naming the X server's process id to the shim.
SERVER_PID = "DIFF_GUI_SERVER_PID"


class Lacking(Exception):
    """This machine lacks what the windows need (`Toolkits.lacking`)."""


def listed(path: pathlib.Path = LIST) -> dict[str, dict[str, str]]:
    """The programs of *path* by name (which may hold spaces), each with
    the variables its run sets (the line's last ``VARIABLE=value`` words);
    comments (``#``) and blank lines are skipped."""
    programs: dict[str, dict[str, str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        words = line.split("#", 1)[0].split()
        variables: dict[str, str] = {}
        while words and _VARIABLE.fullmatch(words[-1]):
            variable, _, value = words.pop().partition("=")
            variables[variable] = value
        if words:
            programs[" ".join(words)] = dict(reversed(variables.items()))
    return programs


def base_name(name: str) -> str:
    """The program a listed *name* runs (without its ``@VARIANT``)."""
    return name.split("@", 1)[0]


#: Where ``toolkits.sh install`` puts the toolkits (its ``TOOLKITS_PREFIX``).
TOOLKITS_PREFIX = pathlib.Path(os.environ.get("TOOLKITS_PREFIX", pathlib.Path.home() / ".local" / "gtk"))


def distributions(sites: list[pathlib.Path]) -> bytes:
    """What the *sites* hold: the names of their ``*.dist-info``
    directories (a distribution and its version), sorted."""
    return "\n".join(sorted(f"{site.name}/{info.name}" for site in sites
                             for info in site.glob("*.dist-info"))).encode()


def toolkit_distributions() -> bytes:
    """What the toolkits' sites hold (`distributions`), which the runs of
    the listed programs get: one site per Python ``toolkits.sh`` built."""
    return distributions(sorted(TOOLKITS_PREFIX.glob("site-*")))


class Toolkits:
    """What runs of the listed programs need here: the toolkits'
    variables and Xvfb; `lacking` says what is missing."""

    def __init__(self, python: str) -> None:
        self.variables: dict[str, str] = {}
        self.python_path: list[str] = []
        self.lacking: str | None = None
        if sys.platform != "linux":
            self.lacking = "the GUI programs run on Linux only (an Xvfb per run)"
            return
        if shutil.which("Xvfb") is None:
            self.lacking = "Xvfb is not on PATH"
            return
        env = {k: v for k, v in os.environ.items() if k != "PYTHONPATH"}
        done = subprocess.run(["sh", str(TOOLKITS), "env"], env=env | {"TOOLKITS_PYTHON": python},
                              capture_output=True, text=True)
        if done.returncode != 0:
            self.lacking = (f"no GUI toolkits ({done.stderr.strip() or 'toolkits.sh env failed'}): "
                            "run python/tools/toolkits.sh install")
            return
        for name, _, value in (line.partition("=") for line in done.stdout.splitlines() if "=" in line):
            if name == "PYTHONPATH":
                self.python_path = [entry for entry in value.split(os.pathsep) if entry]
            else:
                self.variables[name] = value

    def environment(self, env: dict[str, str], variables: dict[str, str]) -> dict[str, str]:
        """*env* for a listed program's run with *variables*: the
        toolkits' variables and site, the locale, no compositor."""
        env = env | self.variables | {"LC_ALL": LOCALE} | variables
        env.pop("WAYLAND_DISPLAY", None)
        if self.python_path:
            env["PYTHONPATH"] = os.pathsep.join([*filter(None, [env.get("PYTHONPATH")]), *self.python_path])
        return env


@contextlib.contextmanager
def display() -> Iterator[dict[str, str]]:
    """An Xvfb of the run's own, as the variables naming it (``DISPLAY``
    and the server's process id for the shim); stopped when the block
    ends. `Lacking` when none can start."""
    sys.path.insert(0, str(HERE.parents[1] / "python" / "tests"))
    try:
        from gui_events import xdisplay
    finally:
        sys.path.remove(str(HERE.parents[1] / "python" / "tests"))
    try:
        with xdisplay.private_display() as name:
            yield {"DISPLAY": name, SERVER_PID: str(xdisplay.server_pid(name))}
    except xdisplay.Unavailable as err:
        raise Lacking(f"no X server: {err}") from None
