"""Support code for the plt-rs conformance spec.

The spec is the contract plt-rs must honour. It is *executed against real
matplotlib* to produce goldens, and plt-rs is then checked against those
goldens. Two kinds of golden exist:

* **Probes** — ``(area, op, args) -> result`` records of matplotlib's
  numerical/textual behaviour (colour parsing, tick locations, transforms,
  statistics, ...). Stored as JSON in ``spec/goldens/probes/<area>.json``.
* **Figure cases** — small scripts that build a figure. Each produces a PNG
  rendering plus a JSON file of structural *facts* (axes positions, limits,
  ticks, labels) in ``spec/goldens/figures/<group>/<name>.{png,json}``.
"""

from .golden import ProbeSet, all_probe_sets, to_jsonable
from .figcases import FigureCase, case, all_cases
from .data import SpecRng, rng

__all__ = [
    "FigureCase",
    "SpecRng",
    "ProbeSet",
    "all_cases",
    "all_probe_sets",
    "case",
    "rng",
    "to_jsonable",
]
