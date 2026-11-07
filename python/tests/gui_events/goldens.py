"""
The goldens: matplotlib's runs of the scenarios, recorded with QtAgg
(``python -m gui_events --record``), which the tests compare pltrs' runs
with, so that they need neither matplotlib nor Qt. A scenario's records
are kept as JSON lines (without the times), its snapshots as PNG files.
"""

import json
import pathlib

from .compare import normalise
from .driver import Run

#: Where the goldens are.
GOLDENS = pathlib.Path(__file__).resolve().parent / "goldens"


def _records_path(name):
    return GOLDENS / f"{name}.jsonl"


def record(name, run):
    """Keep *run* (a `Run` of matplotlib) as scenario *name*'s golden."""
    from PIL import Image
    GOLDENS.mkdir(exist_ok=True)
    for old in GOLDENS.glob(f"{name}.*.png"):
        old.unlink()
    lines = [json.dumps(record, sort_keys=True, ensure_ascii=False)
             for record in normalise(run.records)]
    _records_path(name).write_text("\n".join(lines) + "\n", encoding="utf-8")
    for snapshot, (width, height, pixels) in run.snapshots.items():
        Image.frombytes("RGB", (width, height), pixels).save(
            GOLDENS / f"{name}.{snapshot}.png", optimize=True)


def load(name):
    """Scenario *name*'s golden (a `Run`), or None without one."""
    from PIL import Image
    path = _records_path(name)
    if not path.exists():
        return None
    records = [json.loads(line) for line in
               path.read_text(encoding="utf-8").splitlines()]
    snapshots = {}
    for png in GOLDENS.glob(f"{name}.*.png"):
        with Image.open(png) as image:
            rgb = image.convert("RGB")
            snapshots[png.stem.removeprefix(f"{name}.")] = (
                rgb.width, rgb.height, rgb.tobytes())
    return Run(records, snapshots)
