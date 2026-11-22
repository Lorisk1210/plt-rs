"""Write coverage.md from match.jsonl and print the per-module counts."""
import collections
import json
import sys

from estimates import ESTIMATES, IMPLEMENTED  # noqa: E402

from paths import COVERAGE, DATA as D
STATUSES = ["covered", "differently", "n/a", "missing"]
records = [json.loads(line) for line in open(f"{D}/match.jsonl")]


def label(r):
    owner = f"{r['owner']}." if r["owner"] else ""
    return f"{owner}{r['name']}"


def cell(text):
    return (text or "").replace("|", "\\|").replace("\n", " ")


counts = collections.OrderedDict()
for r in records:
    c = counts.setdefault(r["module"], collections.Counter())
    c[r["status"]] += 1
totals = collections.Counter()
for c in counts.values():
    totals.update(c)

out = []
w = out.append
w("# matplotlib 3.11 public API coverage in pltrs")
w("")
w("`spec/api-audit/run` generates this file with enumerate.py, rust_index.py, match.py,")
w("rules*.py, classmap.py and report.py. Do not edit it by hand.")
w("")
w("This file tells for each public item of matplotlib if pltrs has it. Introspection in")
w("the reference venv finds these items:")
w("")
w("* the public functions, classes, methods, properties and class attributes of the")
w("  modules of the API reference")
w("* the members of private bases such as `_AxesBase`, under the private base")
w("* the instance attributes of Figure, Axes, Line2D, Text, Legend, AxesImage, Colorbar,")
w("  XAxis and FigureCanvasBase")
w("* the parameters of every `Axes`, `Axes3D` and `pyplot` method")
w("")
w("The modules include the renderers, canvases and document writers of the backends,")
w("`backend_managers`, `ft2font`, `dviread`, `texmanager` and `testing`. The pltrs API")
w("comes from the rustdoc JSON of `pltrs`, `pltrs-core`, `pltrs-text`, `pltrs-render`,")
w("`pltrs-font`, `pltrs-raster` and `pltrs-codec`.")
w("")
w("A matcher finds each item by its name or conventional name on the pltrs type of the")
w("class. It also uses the porting guide tables, `matplotlib's ...` mentions in the")
w("documentation and option fields for parameters. Hand rules classify the other items")
w("and correct wrong matches. The statuses are:")
w("")
w("* `covered`: pltrs has the item, which the column names.")
w("* `differently`: pltrs has it in another form, which the note describes.")
w("* `n/a`: it does not apply to pltrs, for the reason in the note.")
w("* `missing`: pltrs does not have it.")
w("")
w("The file skips deprecated matplotlib APIs (`[*Deprecated*]` docstrings): "
  f"{totals['deprecated']} items, listed at the end.")
w("")
w("## Counts per module")
w("")
w("| module | covered | differently | n/a | missing | total |")
w("|---|---:|---:|---:|---:|---:|")
for module, c in counts.items():
    total = sum(c[s] for s in STATUSES)
    w(f"| `{module}` | {c['covered']} | {c['differently']} | {c['n/a']} | {c['missing']} | {total} |")
total = sum(totals[s] for s in STATUSES)
w(f"| **all** | **{totals['covered']}** | **{totals['differently']}** | **{totals['n/a']}** | "
  f"**{totals['missing']}** | **{total}** |")
w("")
by_kind = collections.Counter((r["kind"] == "kwarg", r["status"]) for r in records)
items = {s: sum(v for (k, st), v in by_kind.items() if not k and st == s) for s in STATUSES}
w(f"Without the parameters: {sum(items.values())} classes, functions, methods and attributes "
  f"({items['covered']} covered, {items['differently']} differently, {items['n/a']} n/a, "
  f"{items['missing']} missing).")
w("")
w(f"Parameters of plotting methods: {sum(v for (k, s), v in by_kind.items() if k and s in STATUSES)} "
  f"({by_kind[(True, 'covered')]} covered, {by_kind[(True, 'differently')]} "
  f"differently, {by_kind[(True, 'n/a')]} n/a, {by_kind[(True, 'missing')]} missing).")
w("")
w("## Implemented by the audits")
w("")
for date, implemented in IMPLEMENTED:
    w(f"Found missing by the audit of {date} and implemented in pltrs (Rust), with tests,")
    w("doctests and rows in the porting guide (`crates/pltrs/src/porting/`):")
    w("")
    for item, text in implemented:
        w(f"* `{item}`: {text}")
    w("")
w("The items the audit of 2026-10-26 found missing were implemented by it, and checked against")
w("matplotlib by the probes of area `api_audit` (`spec/reference/probes/test_api_audit.py`,")
w("`crates/pltrs-spec/src/probes/areas/api_audit.rs`); those still missing then were")
w("implemented since (their verdicts are in `rules_impl.py`).")
w("")
w("## Still missing")
w("")
w("| matplotlib | what | estimate |")
w("|---|---|---|")
missing = [r for r in records if r["status"] == "missing"]
groups = collections.OrderedDict()
for r in missing:
    groups.setdefault(r["note"].split(": ", 1)[-1], []).append(f"{r['module']}:{label(r)}")
for note, items in groups.items():
    shown = ", ".join(f"`{i}`" for i in items[:6]) + (f" (+{len(items) - 6})" if len(items) > 6 else "")
    w(f"| {cell(shown)} | {cell(note)} | {cell(ESTIMATES.get(note, ''))} |")
w("")
w("## Full table")
for module in counts:
    w("")
    w(f"### `{module}`")
    w("")
    w("| item | kind | status | pltrs | note |")
    w("|---|---|---|---|---|")
    for r in records:
        if r["module"] != module or r["status"] == "deprecated":
            continue
        w(f"| `{cell(label(r))}` | {r['kind']} | {r['status']} | {cell(r['rust'])} | {cell(r['note'])} |")
w("")
w("## Skipped: deprecated in matplotlib 3.11")
w("")
for r in records:
    if r["status"] == "deprecated":
        w(f"* `{r['module']}:{label(r)}`")
open(COVERAGE, "w").write("\n".join(out) + "\n")

for module, c in counts.items():
    print(f"{module:55s} {c['covered']:5d} {c['differently']:5d} {c['n/a']:5d} {c['missing']:4d}")
print(f"{'all':55s} {totals['covered']:5d} {totals['differently']:5d} {totals['n/a']:5d} {totals['missing']:4d}  (deprecated skipped: {totals['deprecated']})")
