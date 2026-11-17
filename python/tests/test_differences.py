"""
``DIFFERENCES.md`` tells users every difference from matplotlib that the
test harness's lists declare intended: each entry of those lists stands
below a topic line (``# topic: <anchor>``, in the Rust waivers
``// topic: <anchor>``) naming a section of the file, so that a new entry
without a place there fails here.
"""

import pathlib
import re

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not (ROOT / "spec" / "diff").is_dir():
    pytest.skip("the lists of intended differences are not in this tree (an sdist)",
                allow_module_level=True)

DIFFERENCES = ROOT / "DIFFERENCES.md"

#: The lists of intended differences, with the comment that starts their
#: lines and what an entry looks like.
LISTS = {
    "spec/diff/allowlist.txt": ("#", None),
    "spec/diff/private-allowlist.txt": ("#", None),
    "spec/diff/types-allowlist.txt": ("#", None),
    "spec/diff/docs-allowlist.txt": ("#", None),
    "spec/reference/not_applicable.txt": ("#", None),
    "crates/pltrs-spec/src/probes/waivers.rs": ("//", re.compile(r"\s*Waiver \{$")),
}


def _anchor(heading):
    """The anchor GitHub gives a heading: lower case, punctuation but
    hyphens dropped, spaces as hyphens."""
    return re.sub(r"[^\w\- ]", "", heading.strip().lower()).replace(" ", "-")


def _sections():
    """The anchors of the file's sections."""
    text = DIFFERENCES.read_text(encoding="utf-8")
    return {_anchor(line[3:]) for line in text.splitlines() if line.startswith("## ")}


def _entries(path, comment, entry):
    """Each entry of the list at *path* with the topic it stands below
    (None before the first), and the line it is on."""
    topic_line = re.compile(rf"\s*{re.escape(comment)} topic: (\S+)$")
    topic = None
    found = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if (match := topic_line.match(line)):
            topic = match[1]
        elif entry is None and line.strip() and not line.lstrip().startswith(comment):
            found.append((number, topic))
        elif entry is not None and entry.match(line):
            found.append((number, topic))
    return found


def test_the_file_has_sections():
    assert {"the-look-of-figures", "not-built"} <= _sections()


@pytest.mark.parametrize("name", LISTS)
def test_every_entry_names_a_section_of_differences_md(name):
    comment, entry = LISTS[name]
    sections = _sections()
    entries = _entries(ROOT / name, comment, entry)
    assert entries, f"{name} has no entries"
    untold = [f"{name}:{number} ({topic})" for number, topic in entries
              if topic not in sections]
    assert untold == [], ("entries below no topic line, or one naming no section of "
                          f"DIFFERENCES.md: {untold}")


def test_every_waiver_is_found():
    path = ROOT / "crates/pltrs-spec/src/probes/waivers.rs"
    entries = _entries(path, *LISTS["crates/pltrs-spec/src/probes/waivers.rs"])
    assert len(entries) == path.read_text(encoding="utf-8").count('\n        op: "')
