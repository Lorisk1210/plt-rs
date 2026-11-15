"""The leak harness's short mode as a test: the harness's own scenes,
made, drawn, saved and dropped under pltrs, leave nothing alive that
matplotlib does not leave (its outcomes recorded in ``reference.json``),
and figures updated in place keep no more than they show (see the
package's docstring; ``python -m leaks`` runs the full harness)."""

import os
import pathlib

from . import runner


def test_scenes_leave_alive_only_what_matplotlib_leaves(tmp_path):
    names = runner.selected([], own=True)
    jobs = int(os.environ.get("PLTRS_LEAKS_JOBS", "4"))
    verdicts, _, _ = runner.run(names, runner.SHORT, pathlib.Path(tmp_path), jobs,
                                 recorded=True)
    bad = [runner.summary(verdict) for verdict in verdicts if verdict.unexpected]
    assert not bad, "\n".join(bad)
