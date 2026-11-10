"""
The catalogue of the system's fonts that pltrs keeps in its cache
directory, as matplotlib keeps its font list there: written by the first
start that uses a font, read by the next ones, rebuilt when it is broken,
and a cache directory that cannot be written costs matplotlib's warning
and nothing else.
"""

import json
import os
import subprocess
import sys

import pytest

CATALOGUE = "fontlist-v2.pltrs"

# A program drawing text, then printing its cache directory, the font
# list, what findfont finds and the font names.
PROGRAM = """
import io, json, os
import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager as fm
fig = plt.figure()
fig.text(0.5, 0.5, "catalogue")
fig.savefig(io.BytesIO(), format="png")
fonts = sorted([os.path.basename(e.fname), e.index, e.name, e.style,
                str(e.weight), e.stretch] for e in fm.fontManager.ttflist)
print(json.dumps({"cache": pltrs.get_cachedir(), "fonts": fonts,
                  "found": os.path.basename(fm.findfont(
                      fm.FontProperties(family=["sans-serif"]))),
                  "names": sorted(fm.get_font_names())}))
"""

# matplotlib's test_lockfilefailure: the cache directory made read-only
# once it was looked up; then text is drawn.
READ_ONLY = """
import io, os
import pltrs
path = pltrs.get_cachedir()
os.chmod(path, 0o555)
import pltrs.pyplot as plt
fig = plt.figure()
fig.text(0.5, 0.5, "catalogue")
fig.savefig(io.BytesIO(), format="png")
print(path)
"""


def _run(config, code):
    """What *code* prints and logs, run with the configuration and cache
    directory *config*."""
    env = {key: value for key, value in os.environ.items()
           if key not in ("PLTRSCONFIGDIR", "MPLCONFIGDIR")}
    env["PLTRSCONFIGDIR"] = str(config)
    # matplotlib's name for it too: run under matplotlib (pltrs aliased to
    # it), READ_ONLY would otherwise take the write permission from the
    # user's own cache directory.
    env["MPLCONFIGDIR"] = str(config)
    result = subprocess.run([sys.executable, "-c", code], env=env,
                            capture_output=True, text=True, check=False)
    assert result.returncode == 0, result.stderr
    return result.stdout.splitlines()[-1], result.stderr


def test_the_catalogue_is_written_once_and_read_by_later_starts(tmp_path):
    first, _ = _run(tmp_path, PROGRAM)
    catalogue = tmp_path / CATALOGUE
    assert json.loads(first)["cache"] == str(tmp_path)
    written = catalogue.read_bytes()
    stamp = catalogue.stat().st_mtime_ns
    second, _ = _run(tmp_path, PROGRAM)
    assert second == first
    assert catalogue.stat().st_mtime_ns == stamp, "read, not written again"
    assert catalogue.read_bytes() == written


@pytest.mark.parametrize("broken", [b"", b"not a catalogue", None])
def test_broken_catalogues_are_rebuilt(tmp_path, broken):
    first, _ = _run(tmp_path, PROGRAM)
    catalogue = tmp_path / CATALOGUE
    written = catalogue.read_bytes()
    if broken is None:
        # Cut short.
        catalogue.write_bytes(written[:len(written) // 2])
    else:
        catalogue.write_bytes(broken)
    again, _ = _run(tmp_path, PROGRAM)
    assert again == first
    assert catalogue.read_bytes() == written


@pytest.mark.skipif(sys.platform == "win32" or os.geteuid() == 0,
                    reason="needs a directory its owner cannot write")
def test_an_unwritable_cache_directory_warns_as_matplotlib_does(tmp_path):
    try:
        path, logged = _run(tmp_path, READ_ONLY)
    finally:
        os.chmod(tmp_path, 0o755)
    assert path == str(tmp_path)
    # matplotlib logs "Could not save font_manager cache [Errno 13]
    # Permission denied: '<cache>/fontlist-v3.11.0.json.matplotlib-lock'".
    assert ("Could not save font_manager cache [Errno 13] Permission denied: "
            f"'{tmp_path / CATALOGUE}'") in logged
    assert not (tmp_path / CATALOGUE).exists()
