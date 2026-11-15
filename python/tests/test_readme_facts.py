"""
The numbers and facts ``README.md`` and ``CHANGELOG.md`` state, against
the files of the tree that back them: the committed outcomes of
matplotlib's suite, of the libraries' suites and of the notebooks, the
corpora of the differential tester, the spec's goldens, the examples, and
the package's metadata.

The documents mark each such fact with an HTML comment just before it,
``<!--fact:NAME-->``, which neither GitHub nor PyPI shows. A fact is
exact (the text after the marker starts with what the files give) or,
for round figures given with "about" or "some", approximate (within
`APPROXIMATE_TOLERANCE` of what the files give). When the files change,
this test names the document and the sentence to update.
"""

import ast
import json
import pathlib
import re
import statistics
import tomllib

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
README = ROOT / "README.md"
CHANGELOG = ROOT / "CHANGELOG.md"
if not (README.is_file() and CHANGELOG.is_file() and (ROOT / "spec").is_dir()):
    pytest.skip("the documents and the spec are in the source tree only", allow_module_level=True)

#: How far an approximate figure may be from what the files give.
APPROXIMATE_TOLERANCE = 0.03

#: A fact marker and the text that follows it, up to the next marker
#: (in the README with its lines joined).
MARKER = re.compile(r"<!--fact:([\w.-]+)-->((?:(?!<!--).)*)")

#: The keys of an outcome file that are no library's version.
OUTCOME_KEYS = {"matplotlib", "counts", "collection_errors", "outcomes"}


def _json(path):
    with open(ROOT / path, encoding="utf-8") as file:
        return json.load(file)


def _passing(outcomes):
    return {test for test, outcome in outcomes["outcomes"].items() if outcome == "passed"}


def _number(value):
    """*value* as the README writes numbers (``10,191``)."""
    return f"{value:,}"


def _all_reference_passing(directory):
    """The number of tests passing under matplotlib in *directory*'s
    outcome files, all of which the README says pass under pltrs."""
    reference = _passing(_json(f"{directory}/reference.json"))
    missing = reference - _passing(_json(f"{directory}/pltrs.json"))
    assert not missing, (
        f"the README says pltrs passes all {len(reference):,} tests of {directory} that pass "
        f"under matplotlib, but {len(missing)} do not pass, e.g. {sorted(missing)[:3]}")
    return _number(len(reference))


def _ecosystem():
    """The libraries of the ecosystem's suites, the tests passing under
    matplotlib, those of them passing under pltrs, and the libraries
    (test ids' first directories) some of whose passing tests pltrs fails."""
    reference = _json("spec/ecosystem/reference.json")
    pltrs = _json("spec/ecosystem/pltrs.json")
    libraries = set(reference) - OUTCOME_KEYS
    assert libraries == set(pltrs) - OUTCOME_KEYS
    passing = _passing(reference)
    both = passing & _passing(pltrs)
    incomplete = {test.split("/")[0] for test in passing - both}
    return libraries, passing, both, incomplete


def _corpora():
    # Code corpora: a data-only entry (kind = "data") holds no programs.
    with open(ROOT / "spec/diff/corpora.toml", "rb") as file:
        manifest = tomllib.load(file)
    return _number(sum(entry.get("kind", "code") != "data" for entry in manifest.values()))


def _corpus_compared(key):
    """Of the corpus programs the last full run of every corpus found
    matplotlib running to their end, drawing or writing a figure (those
    compared in full, ``spec/diff/corpus-outcomes.json``), how many there
    were (``ran``) or how many were clean (``clean``)."""
    return _json("spec/diff/corpus-outcomes.json")["total"]["runs"]["ended, drew"][key]


def _probes():
    return sum(len(_json(path.relative_to(ROOT))["probes"])
               for path in (ROOT / "spec/goldens/probes").glob("*.json"))


def _figure_cases():
    return sum(1 for _ in (ROOT / "spec/goldens/figures").rglob("*.json"))


def _example_graphs():
    rust = {path.stem for path in (ROOT / "examples/rust/src/bin").glob("*.rs")}
    python = {path.stem for path in (ROOT / "examples/python").glob("*.py")} - {"samples"}
    assert rust == python, "every example is written in Rust and in Python"
    return _number(len(rust))


def _gallery_figures():
    entry = re.compile(r"\bEntry::(?:figure|animation)\(")
    return sum(len(entry.findall(path.read_text(encoding="utf-8")))
               for path in (ROOT / "crates/pltrs/examples/gallery").glob("*.rs"))


def _matplotlib_tests():
    """The tests of matplotlib's suite as ``cargo xtask mpl-tests`` counts
    them: the ``test_*`` functions of each ``test_*.py`` module and of its
    module-level classes, from the reference venv's matplotlib."""
    found = sorted([*(ROOT / ".venv").glob("lib/python3*/site-packages/matplotlib/tests"),
                    *(ROOT / ".venv").glob("Lib/site-packages/matplotlib/tests")])
    if not found:
        pytest.skip("matplotlib's tests are in the reference venv (.venv), which is missing")
    count = 0
    for module in found[0].glob("test_*.py"):
        tree = ast.parse(module.read_text(encoding="utf-8"))
        bodies = [tree.body] + [node.body for node in tree.body if isinstance(node, ast.ClassDef)]
        count += len({node.name for body in bodies for node in body
                      if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
                      and node.name.startswith("test_")})
    return _number(count)


def _pyproject():
    with open(ROOT / "pyproject.toml", "rb") as file:
        return tomllib.load(file)["tool"]["pltrs-backend"]


def _minimum_python():
    requires = _pyproject()["pltrs"]["requires-python"]
    assert requires.startswith(">="), requires
    return requires[2:]


def _dependencies():
    """pltrs' dependencies, named as their projects name themselves."""
    display = {"pillow": "Pillow"}
    names = [re.split(r"[<>=!~;\[ ]", requirement)[0]
             for requirement in _pyproject()["pltrs"]["dependencies"]]
    names = [display.get(name, name) for name in names]
    return ", ".join(names[:-1]) + " and " + names[-1]


def _version():
    """pltrs' version: the Cargo workspace's."""
    with open(ROOT / "Cargo.toml", "rb") as file:
        return tomllib.load(file)["workspace"]["package"]["version"]


def _alias_version():
    init = (ROOT / "python/pltrs/__init__.py").read_text(encoding="utf-8")
    matplotlib = re.search(r'^__version__ = "([^"]+)"$', init, re.MULTILINE).group(1)
    return f"{matplotlib}+pltrs.{_version()}"


def _incomplete_libraries():
    """The libraries some of whose tests that pass under matplotlib fail
    under pltrs, in alphabetical order (``a, b and c``)."""
    names = sorted(_ecosystem()[3])
    return names[0] if len(names) == 1 else ", ".join(names[:-1]) + " and " + names[-1]


def _benchmarks():
    """The benchmark report ``cargo xtask bench --report`` writes from
    ``benchmarks/results.json``."""
    return (ROOT / "benchmarks/README.md").read_text(encoding="utf-8")


def _benchmark_summary(claim, run):
    """The typical speed-up of *claim* (0 for pltrs (Python), 1 for pltrs
    (Rust)) in *run* (``"warm"`` or ``"cold"``), as the report's summary
    tables give it (``4.3×``)."""
    rows = re.findall(r"^\| \*\*all workloads\*\* \| \d+ \| [^|]+ \| ([\d.]+×) faster "
                      r"\| [^|]+ \| ([\d.]+×) faster \|$", _benchmarks(), re.MULTILINE)
    assert len(rows) == 2, "the report's two summary tables"
    return rows[claim][0 if run == "warm" else 1]


def _benchmark_match(pattern):
    """What the report says in the group of *pattern*."""
    found = re.search(pattern, _benchmarks())
    assert found, f"benchmarks/README.md no longer says {pattern!r}"
    return found.group(1)


def _benchmark_compared():
    """How many workloads the report's summary tables compare with
    matplotlib."""
    rows = re.findall(r"^\| \*\*all workloads\*\* \| (\d+) \|", _benchmarks(), re.MULTILINE)
    assert len(set(rows)) == 1, "the report's two summary tables compare the same workloads"
    return rows[0]


def _benchmark_memory_above():
    """By how much the cold runs of pltrs (Python) peak above
    matplotlib's, by the medians of ``benchmarks/results.json``: how many
    of the workloads do, and the largest excess in MiB."""
    results = json.loads((ROOT / "benchmarks/results.json").read_text(encoding="utf-8"))
    excesses = []
    for workload in results["workloads"]:
        cold = workload.get("cold") or {}
        peaks = [cold.get(name, {}).get("peak_kib") for name in ("pltrs-python", "matplotlib")]
        if all(peaks):
            excesses.append(statistics.median(peaks[0]) - statistics.median(peaks[1]))
    return str(sum(excess > 0 for excess in excesses)), f"{max(excesses) / 1024:.1f} MiB"


def _benchmark_thin_results():
    """The results under 1.5× the report lists."""
    table = _benchmarks().split("### Where the margins are thin", 1)[1].split("\n## ", 1)[0]
    return str(len(re.findall(r"^\| `", table, re.MULTILINE)))


#: Facts whose text the README gives as it is.
EXACT = {
    "mpl-suite.all-passing": lambda: _all_reference_passing("spec/mpl-suite"),
    "notebooks.all-passing": lambda: _all_reference_passing("spec/notebooks"),
    "ecosystem.libraries": lambda: _number(len(_ecosystem()[0])),
    "ecosystem.reference-passing": lambda: _number(len(_ecosystem()[1])),
    "ecosystem.pltrs-passing": lambda: _number(len(_ecosystem()[2])),
    "ecosystem.complete-libraries":
        lambda: _number(len(_ecosystem()[0]) - len(_ecosystem()[3])),
    "ecosystem.incomplete-libraries": _incomplete_libraries,
    "ecosystem.failing": lambda: _number(len(_ecosystem()[1]) - len(_ecosystem()[2])),
    "corpora.compared-different":
        lambda: _number(_corpus_compared("ran") - _corpus_compared("clean")),
    "version": _version,
    "corpora": _corpora,
    "corpora.compared": lambda: _number(_corpus_compared("ran")),
    "corpora.compared-clean": lambda: _number(_corpus_compared("clean")),
    "examples.graphs": _example_graphs,
    "mpl-tests": _matplotlib_tests,
    "python.minimum": _minimum_python,
    "python.dependencies": _dependencies,
    "alias.version": _alias_version,
    "bench.workloads": lambda: _benchmark_match(r"times matplotlib [\d.]+ against both faces "
                                                r"of pltrs on (\d+) workloads"),
    "bench.date": lambda: _benchmark_match(r"measured on (\S+) at commit"),
    "bench.commit": lambda: _benchmark_match(r"at commit `([^`]+)`"),
    "bench.python.warm": lambda: _benchmark_summary(0, "warm"),
    "bench.python.cold": lambda: _benchmark_summary(0, "cold"),
    "bench.rust.warm": lambda: _benchmark_summary(1, "warm"),
    "bench.rust.cold": lambda: _benchmark_summary(1, "cold"),
    "bench.python.memory": lambda: _benchmark_match(
        r"A cold run of pltrs \(Python\) typically peaks at ([\d.]+) times"),
    "bench.python.memory.range": lambda: _benchmark_match(
        r"A cold run of pltrs \(Python\) typically peaks at [\d.]+ times[^.]*?"
        r"and at ([\d.]+ to [\d.]+) times over all"),
    "bench.python.memory.above": lambda: _benchmark_memory_above()[0],
    "bench.python.memory.excess": lambda: _benchmark_memory_above()[1],
    "bench.compared": _benchmark_compared,
    "bench.import.matplotlib": lambda: _benchmark_match(r"imports `matplotlib.pyplot` takes (\d+ ms)"),
    "bench.import.pltrs": lambda: _benchmark_match(r"imports `pltrs.pyplot` takes (\d+ ms)"),
    "bench.thin": _benchmark_thin_results,
}

#: Facts the README rounds ("about 48,500 probes"), as numbers.
APPROXIMATE = {
    "spec.probes": _probes,
    "spec.figure-cases": _figure_cases,
    "gallery.figures": _gallery_figures,
}

#: The documents, by file name, as they are.
TEXTS = {path.name: path.read_text(encoding="utf-8") for path in (README, CHANGELOG)}
#: (document, fact, the text after its marker) for every marker.
MARKED = [(document, match.group(1), match.group(2))
          for document, text in TEXTS.items()
          for match in MARKER.finditer(re.sub(r"\s*\n\s*", " ", text))]


@pytest.mark.parametrize("document", sorted(TEXTS))
def test_no_marker_starts_a_line(document):
    """A line starting with ``<!--`` would start an HTML block in
    Markdown, which ends the paragraph and shows the rest of the line as
    raw HTML."""
    assert not [line for line in TEXTS[document].splitlines()
                if line.lstrip().startswith("<!--fact:")]


def test_every_fact_is_stated_and_every_marker_known():
    named = {name for _, name, _ in MARKED}
    assert named - EXACT.keys() - APPROXIMATE.keys() == set(), "markers of unknown facts"
    assert EXACT.keys() | APPROXIMATE.keys() <= named, "facts no document marks any more"


@pytest.mark.parametrize(("document", "name", "text"), MARKED,
                         ids=[f"{document}:{name}" for document, name, _ in MARKED])
def test_documents_state_what_the_files_give(document, name, text):
    if name in EXACT:
        expected = EXACT[name]()
        assert text.lstrip("`").startswith(expected), (
            f"{document} says {text[:40]!r} after <!--fact:{name}-->; the files give {expected!r}")
    else:
        stated = re.match(r"[\d,]+", text)
        assert stated, f"no number after <!--fact:{name}--> in {document}"
        value = int(stated.group().replace(",", ""))
        actual = APPROXIMATE[name]()
        assert abs(value - actual) <= APPROXIMATE_TOLERANCE * actual, (
            f"{document} says about {value:,} after <!--fact:{name}-->; the files give {actual:,}")


def test_the_libraries_the_readme_names_pass_in_full():
    """The README names pandas and seaborn among the libraries whose
    suites pltrs passes in full."""
    _, passing, _, incomplete = _ecosystem()
    for library in ("pandas", "seaborn"):
        assert any(test.startswith(f"{library}/") for test in passing)
        assert library not in incomplete
