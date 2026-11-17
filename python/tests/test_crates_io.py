"""
What the crates published on crates.io carry, against the tree: each
crate's licence files, ``THIRD_PARTY.md`` and ``LICENSES/``, as
``python/tools/crate_notices.py`` makes them from the root's, and the
manifests' publishing fields (which crates publish, the versions the
crates require of each other, the metadata crates.io shows).
"""

import pathlib
import sys
import tomllib

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not ((ROOT / "Cargo.toml").is_file() and (ROOT / "THIRD_PARTY.md").is_file()):
    pytest.skip("the crates are in the source tree only", allow_module_level=True)
sys.path.insert(0, str(ROOT / "python" / "tools"))

import crate_notices  # noqa: E402

#: The fields crates.io shows that every published crate gives.
METADATA = ("description", "readme", "keywords", "categories", "license", "repository",
            "rust-version", "edition")


def _manifest(path):
    with open(path, "rb") as file:
        return tomllib.load(file)


WORKSPACE = _manifest(ROOT / "Cargo.toml")["workspace"]


def _field(package, name):
    """The value of the package field *name*, inherited from the
    workspace when the crate says ``name.workspace = true``."""
    value = package.get(name)
    if isinstance(value, dict) and value.get("workspace"):
        return WORKSPACE["package"].get(name)
    return value


def test_every_published_crate_carries_its_notices():
    problems = crate_notices.differences()
    assert problems == [], (
        "run `python python/tools/crate_notices.py` after changing THIRD_PARTY.md or "
        "LICENSES/:\n" + "\n".join(problems))


def test_the_notices_name_what_each_crate_ports():
    files = crate_notices.notices()
    # The licences a crate's rows cite are the texts it carries.
    assert files["crates/pltrs-codec/LICENSES/zlib-ng.txt"] is not None
    assert files["crates/pltrs-core/LICENSES/fdlibm.txt"] is not None
    assert files["crates/pltrs-core/LICENSES/pillow.txt"] is None
    assert "| FreeType 2.14 |" in files["crates/pltrs-raster/THIRD_PARTY.md"]
    assert "based in part on the work of the FreeType Team" in (
        files["crates/pltrs-raster/THIRD_PARTY.md"])
    assert "the changes in summary" in files["crates/pltrs-text/THIRD_PARTY.md"]
    # A crate no row names carries pltrs' licence alone.
    assert files["crates/pltrs-ui/THIRD_PARTY.md"] is None
    assert files["crates/pltrs-ui/LICENSE-MIT"] == (ROOT / "LICENSE-MIT").read_text()


def test_only_the_library_crates_publish():
    members = {path.parent.name: _manifest(path)["package"]
               for path in ROOT.glob("crates/*/Cargo.toml")}
    for name, package in members.items():
        published = _field(package, "publish") is not False
        assert published == (name in crate_notices.CRATES), name
    assert "publish" not in WORKSPACE["package"]


def test_the_crates_require_each_other_at_exactly_the_release():
    version = WORKSPACE["package"]["version"]
    dependencies = WORKSPACE["dependencies"]
    for name in crate_notices.CRATES:
        assert dependencies[name]["version"] == f"={version}", name
        assert dependencies[name]["path"] == f"crates/{name}"


@pytest.mark.parametrize("name", crate_notices.CRATES)
def test_the_metadata_crates_io_shows(name):
    package = _manifest(ROOT / "crates" / name / "Cargo.toml")["package"]
    for field in METADATA:
        assert _field(package, field), f"{name}: {field}"
    assert (ROOT / "crates" / name / _field(package, "readme")).is_file()
    # crates.io takes at most five keywords of up to 20 characters.
    keywords = _field(package, "keywords")
    assert len(keywords) <= 5 and all(len(keyword) <= 20 for keyword in keywords)
