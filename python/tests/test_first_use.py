"""
Several programs using a new configuration directory at once, as the first
programs after an installation (or a test suite's workers) do: each writes
what pltrs keeps there on first use (the data directory with the shipped
data, linked or, without the privilege of links on Windows, copied; the
font catalogue; the docstring tables), and none may fail, whatever the
others are writing. On Windows, copies racing each other once crashed
(WinError 32 and 183).
"""

import os
import pathlib
import re
import subprocess
import sys
import textwrap

import pytest

import pltrs

#: How many programs start at once, and how often, each time on a new
#: directory.
PROGRAMS, ROUNDS = 8, 5

#: What each program does: everything that writes the directory on first
#: use.
PROGRAM = textwrap.dedent("""
    import io
    import pltrs
    pltrs.use("agg")
    pltrs.get_data_path()
    import pltrs.pyplot as plt
    fig = plt.figure()
    fig.text(0.5, 0.5, "first use")
    fig.savefig(io.BytesIO(), format="png")
    print(pltrs.get_data_path())
""")

#: The names of the files written on the way (`<name>.<process>`, also
#: hidden, `.partial` copies, `.tmp` catalogues), none of which may stay.
TEMPORARY = re.compile(r"\.\d+(\.partial)?$|\.tmp$")

#: The entries of the package's shipped data, which the data directory
#: reaches.
SHIPPED = sorted(entry.name for entry in
                 pathlib.Path(pltrs.__file__).with_name("mpl-data").iterdir())


def _files(path):
    """The files below *path*, relative to it, sorted."""
    return sorted(str(file.relative_to(path)) for file in path.rglob("*") if file.is_file())


@pytest.mark.parametrize("round", range(ROUNDS))
def test_programs_sharing_a_new_configuration_directory_all_run(round, tmp_path):
    config = tmp_path / "config"
    config.mkdir()
    # (matplotlib's name for it too, for a run under matplotlib, pltrs
    # aliased to it.)
    env = {**os.environ, "PLTRSCONFIGDIR": str(config), "MPLCONFIGDIR": str(config),
           "MPLBACKEND": "agg",
           "PYTHONPATH": os.pathsep.join(filter(None, [
               str(pathlib.Path(pltrs.__file__).parents[1]), os.environ.get("PYTHONPATH")]))}
    programs = [subprocess.Popen([sys.executable, "-c", PROGRAM], env=env, text=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                for _ in range(PROGRAMS)]
    data_paths = set()
    for program in programs:
        out, err = program.communicate(timeout=300)
        assert (program.returncode, err) == (0, ""), err
        data_paths.add(out.strip())
    assert len(data_paths) == 1
    data = pathlib.Path(data_paths.pop())
    assert [name for name in SHIPPED if not (data / name).exists()] == []
    package = pathlib.Path(pltrs.__file__).with_name("mpl-data")
    for name in SHIPPED:
        if (package / name).is_dir():
            assert _files(data / name) == _files(package / name), name
    assert [path.name for path in config.rglob("*") if TEMPORARY.search(path.name)] == []
