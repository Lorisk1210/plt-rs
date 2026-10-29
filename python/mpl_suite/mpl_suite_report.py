"""A pytest plugin recording how every test of a run ended.

``cargo xtask mpl-suite`` loads it with ``-p mpl_suite_report`` and names
the output file in ``MPL_SUITE_RESULTS``. The file gets one JSON object
per line: ``{"id", "outcome", "reason"}`` for each test, where the outcome
is one of ``passed``, ``failed``, ``error``, ``skipped``, ``xfailed`` and
``xpassed``, and ``{"collect", "reason"}`` for each test file that could
not be collected. Under pytest-xdist only the controller writes: workers
send it their reports.
"""

import json
import os

#: Outcome of a phase report, by (phase, pytest outcome, has wasxfail).
_PHASE_OUTCOMES = {
    ("setup", "failed", False): "error",
    ("setup", "skipped", False): "skipped",
    ("setup", "skipped", True): "xfailed",
    ("call", "passed", False): "passed",
    ("call", "passed", True): "xpassed",
    ("call", "failed", False): "failed",
    ("call", "skipped", False): "skipped",
    ("call", "skipped", True): "xfailed",
}

_out = None
_pending = {}


def pytest_configure(config):
    """Open the output file, in the controller (or a run without xdist)."""
    global _out
    path = os.environ.get("MPL_SUITE_RESULTS")
    if path and not hasattr(config, "workerinput"):
        _out = open(path, "w", encoding="utf-8")


def pytest_unconfigure(config):
    """Close the output file."""
    if _out is not None:
        _out.close()


def _reason(report):
    """A one-line reason for a report that did not pass."""
    longrepr = report.longrepr
    if longrepr is None:
        return getattr(report, "wasxfail", "") or ""
    if isinstance(longrepr, tuple):  # a skip: (path, line, reason)
        return str(longrepr[2])
    crash = getattr(longrepr, "reprcrash", None)
    if crash is not None and crash.message:
        return crash.message
    lines = str(longrepr).strip().splitlines()
    errors = [line[1:].strip() for line in lines if line.startswith("E ")]
    return errors[-1] if errors else (lines[-1] if lines else "")


def _write(record):
    _out.write(json.dumps(record) + "\n")
    _out.flush()


def pytest_collectreport(report):
    """Record a test file that failed to collect."""
    if _out is not None and report.failed:
        _write({"collect": report.nodeid, "reason": _reason(report)})


def pytest_runtest_logreport(report):
    """Combine a test's phase reports and record it after its teardown."""
    if _out is None:
        return
    if report.when == "teardown":
        outcome, reason = _pending.pop(report.nodeid, ("passed", ""))
        if report.failed and outcome in ("passed", "xpassed"):
            outcome, reason = "error", _reason(report)
        _write({"id": report.nodeid, "outcome": outcome, "reason": reason})
        return
    key = (report.when, report.outcome, hasattr(report, "wasxfail"))
    outcome = _PHASE_OUTCOMES.get(key, "failed" if report.failed else None)
    if outcome is not None and (report.when == "call" or outcome != "passed"):
        reason = "" if outcome == "passed" else _reason(report)
        _pending[report.nodeid] = (outcome, reason)
