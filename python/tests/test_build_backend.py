"""
pltrs' build backend (``python/backend``): the wheels and sdists it
writes and checks, the metadata of its two distributions, and how it
links the extension (the tags are ``test_build_backend_tags.py``'s).
"""

import base64
import csv
import hashlib
import io
import json
import pathlib
import shutil
import sys
import tarfile
import tomllib
import types
import zipfile

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not (ROOT / "python" / "backend").is_dir():
    pytest.skip("the build backend is in the source tree only", allow_module_level=True)
sys.path.insert(0, str(ROOT / "python" / "backend"))

import pltrs_backend  # noqa: E402
from pltrs_backend import _audit, _cargo, _core_metadata, _linker, _sdist, _zigcc  # noqa: E402
from pltrs_backend._project import DISTRIBUTIONS, Project  # noqa: E402
from pltrs_backend._wheel import WheelWriter, normalise  # noqa: E402


def test_wheels_record_every_file(tmp_path):
    wheel = WheelWriter("Some-Name", "1.0")
    wheel.add("pkg/__init__.py", b"x = 1\n")
    wheel.add("pkg/_native.abi3.so", b"\x7fELF", executable=True)
    wheel.add_dist_info("METADATA", b"Metadata-Version: 2.4\n")
    name = wheel.write(tmp_path, ["cp312-abi3-linux_x86_64", "cp312-abi3-manylinux_2_17_x86_64"],
                       pure=False)
    assert name == "some_name-1.0-cp312-abi3-linux_x86_64.manylinux_2_17_x86_64.whl"
    with zipfile.ZipFile(tmp_path / name) as archive:
        names = archive.namelist()
        assert names[-1] == "some_name-1.0.dist-info/RECORD"
        assert archive.read("some_name-1.0.dist-info/WHEEL").decode().splitlines() == [
            "Wheel-Version: 1.0", "Generator: pltrs_backend", "Root-Is-Purelib: false",
            "Tag: cp312-abi3-linux_x86_64", "Tag: cp312-abi3-manylinux_2_17_x86_64"]
        rows = list(csv.reader(io.StringIO(archive.read(names[-1]).decode())))
        for path, digest, size in rows[:-1]:
            data = archive.read(path)
            expected = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
            assert (digest, int(size)) == ("sha256=" + expected.decode(), len(data))
        assert rows[-1] == [names[-1], "", ""]
        assert archive.getinfo("pkg/_native.abi3.so").external_attr >> 16 == 0o755
        assert archive.getinfo("pkg/__init__.py").date_time == (1980, 1, 1, 0, 0, 0)


def test_names_normalise_as_wheels_write_them():
    assert normalise("Py.Some--name_x") == "py_some_name_x"


def test_the_two_distributions():
    pltrs = Project(ROOT)
    alias = Project(ROOT, "matplotlib")
    assert alias.version == f"{alias.matplotlib_version}+pltrs.{pltrs.version}"
    assert alias.dependencies == [f"pltrs=={pltrs.version}"]
    metadata = alias.metadata().splitlines()
    assert "Name: matplotlib" in metadata
    assert f"Requires-Dist: pltrs=={pltrs.version}" in metadata
    files = dict(pltrs.package_files())
    assert "pltrs/__init__.py" in files and "pltrs/pyplot.py" in files
    assert {"pltrs/py.typed", "pltrs/pyplot.pyi", "pltrs/axes/_axes.pyi"} <= set(files)
    assert files["pltrs/DIFFERENCES.md"] == ROOT / "DIFFERENCES.md"
    assert not [path for path in files if "_native." in path or "__pycache__" in path]
    aliases = dict(alias.package_files())
    checked = dict(alias.checked_sources())
    base = ROOT / "python/alias"
    alias_stubs = {path.relative_to(base).as_posix() for path in base.rglob("*.pyi")}
    assert sorted(set(aliases) - set(checked) - alias_stubs) == [
        "_pltrs_matplotlib_alias.py", "matplotlib/__init__.py", "matplotlib/py.typed",
        "mpl_toolkits/axes_grid1/__init__.py", "mpl_toolkits/axisartist/__init__.py",
        "mpl_toolkits/mplot3d/__init__.py", "pylab.py"]
    # The alias' stubs: matplotlib's, pyplot's and those of matplotlib's
    # modules without one (units, dates, the backends), and of its
    # toolkits' __init__ beside their loaders, which have no py.typed.
    pltrs_stubs = {f"matplotlib/{path.relative_to(ROOT / 'python/pltrs').as_posix()}"
                   for path in (ROOT / "python/pltrs").rglob("*.pyi")
                   if path.relative_to(ROOT / "python/pltrs").parts[0] != "mpl_toolkits"}
    assert pltrs_stubs < alias_stubs
    assert {"matplotlib/units.pyi", "matplotlib/dates.pyi", "matplotlib/backends/backend_agg.pyi",
            "mpl_toolkits/mplot3d/__init__.pyi"} <= alias_stubs
    assert not (base / "mpl_toolkits/mplot3d/py.typed").exists()
    # Beside each stub, pltrs' module, which checkers look for (and its
    # toolkits' modules, which pyright reads of an untyped package); none
    # of pltrs' own modules.
    assert aliases["matplotlib/__init__.py"] == ROOT / "python/alias/matplotlib/__init__.py"
    assert {"matplotlib/backends/backend_agg.py", "matplotlib/units.py", "matplotlib/pyplot.py",
            "mpl_toolkits/mplot3d/axes3d.py"} <= set(checked)
    assert not [path for path in checked if path.endswith("__init__.py") and "/" in path[11:]
                and path.startswith("mpl_toolkits/")]
    assert not [path for path in checked
                if path.startswith("matplotlib/") and path[:-3] + ".pyi" not in alias_stubs]
    assert "matplotlib/_handles.py" not in checked
    with pytest.raises(ValueError, match="unknown distribution"):
        Project(ROOT, "seaborn")


def test_the_metadata_is_valid_and_carries_every_notice():
    project = Project(ROOT)
    metadata = project.metadata()
    assert _core_metadata.problems(metadata) == []
    message = _core_metadata.parse(metadata)
    with open(ROOT / "Cargo.toml", "rb") as file:
        workspace = tomllib.load(file)["workspace"]["package"]
    assert message["Version"] == workspace["version"]
    assert f"Source, {workspace['repository']}" in message.get_all("Project-URL")
    licences = message.get_all("License-File")
    for notice in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY.md", "LICENSES/matplotlib.txt",
                   "crates/pltrs-font/fonts/LICENSE_DEJAVU", "crates/pltrs-font/fonts/LICENSE_STIX",
                   "crates/pltrs-font/fonts/LICENSE_INTER", "crates/pltrs/src/web/assets/LICENSE",
                   "LICENSES/adobe-glyph-list.txt"):
        assert notice in licences
    # Both distributions are under the crates' licence.
    for name in DISTRIBUTIONS:
        distribution = _core_metadata.parse(Project(ROOT, name).metadata())
        assert distribution["License-Expression"] == workspace["license"] == "MIT OR Apache-2.0"
        assert distribution.get_all("License-File") == licences
    assert "Programming Language :: Python :: 3.12" in message.get_all("Classifier")
    assert message.get_payload().startswith("# pltrs")
    assert _core_metadata.lower_bounds(metadata)["numpy"]


def test_a_licence_expression_is_written_when_given(tmp_path):
    shutil.copy(ROOT / "Cargo.toml", tmp_path)
    (tmp_path / "python" / "pltrs").mkdir(parents=True)
    shutil.copy(ROOT / "python" / "pltrs" / "__init__.py", tmp_path / "python" / "pltrs")
    (tmp_path / "NOTICE.txt").write_text("notice\n")
    (tmp_path / "pyproject.toml").write_text(
        '[tool.pltrs-backend]\nlicense-files = ["NOTICE.txt"]\n'
        '[tool.pltrs-backend.urls]\nSource = "https://example.org/pltrs"\n'
        '[tool.pltrs-backend.pltrs]\nsummary = "s"\nrequires-python = ">=3.12"\n'
        'license = "MIT AND (Apache-2.0 OR BSD-3-Clause)"\n')
    metadata = Project(tmp_path).metadata()
    assert "License-Expression: MIT AND (Apache-2.0 OR BSD-3-Clause)" in metadata.splitlines()
    assert _core_metadata.problems(metadata) == []
    (tmp_path / "pyproject.toml").write_text(
        (tmp_path / "pyproject.toml").read_text().replace("NOTICE.txt", "LICENSE*"))
    with pytest.raises(ValueError, match="'LICENSE\\*' matches nothing"):
        Project(tmp_path).metadata()


def test_metadata_problems_are_named():
    assert _core_metadata.problems(
        "Metadata-Version: 2.4\nName: -x\nVersion: 1.0.0a\nLicense-Expression: MIT OR\n"
        "Requires-Dist: numpy (>=1\nFoo: 1\nSummary: a\nSummary: b\n"
        "Project-URL: https://example.org\nDescription-Content-Type: text/markdown\n") == [
        "bad Name: '-x'", "bad Version: '1.0.0a'",
        "bad License-Expression: incomplete in 'MIT OR'", "bad Requires-Dist: 'numpy (>=1'",
        "unknown field Foo", "Summary given 2 times",
        "bad Project-URL: 'https://example.org'",
        "a Description-Content-Type but no description"]
    assert _core_metadata._license_expression_problem(
        "(MIT OR Apache-2.0) AND GPL-3.0-or-later WITH GCC-exception-3.1") is None
    assert _core_metadata._license_expression_problem("MIT AND AND BSD") == "unexpected 'AND'"
    assert _core_metadata.lower_bounds(
        "Metadata-Version: 2.4\nName: x\nVersion: 1\nRequires-Dist: numpy>=1.26,<3\n"
        "Requires-Dist: pillow (>=10.1)\nRequires-Dist: tomli>=2; python_version<'3.11'\n") == {
        "numpy": "1.26", "pillow": "10.1"}


def test_the_alias_wheel_and_the_sdist_are_built_and_checked(tmp_path, monkeypatch):
    monkeypatch.chdir(ROOT)
    name = pltrs_backend.build_wheel(str(tmp_path), {"distribution": "matplotlib"})
    assert name.endswith("-py3-none-any.whl") and _audit.wheel_problems(tmp_path / name) == []
    sdist = pltrs_backend.build_sdist(str(tmp_path))
    assert _audit.sdist_problems(tmp_path / sdist) == []
    with tarfile.open(tmp_path / sdist) as archive:
        names = archive.getnames()
        stem = sdist[:-len(".tar.gz")]
        for kept in ("Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml", "PKG-INFO",
                     "crates/pltrs-python/build.rs", "crates/pltrs-font/fonts/DejaVuSans.ttf",
                     "python/backend/pltrs_backend/_zigcc.py", "python/tests/test_smoke.py"):
            assert f"{stem}/{kept}" in names
        # The licence, and each published crate's copies of it.
        for licence in ("LICENSE-MIT", "LICENSE-APACHE"):
            for at in ("", "crates/pltrs/", "crates/pltrs-core/"):
                member = archive.getmember(f"{stem}/{at}{licence}")
                assert member.isfile()
                assert archive.extractfile(member).read() == (ROOT / licence).read_bytes()
        for dropped in ("crates/pltrs-spec", "xtask", "examples", "spec", "benchmarks", "target"):
            assert not [n for n in names if n.startswith(f"{stem}/{dropped}/")]
        manifest = tomllib.loads(archive.extractfile(f"{stem}/Cargo.toml").read().decode())
        assert manifest["workspace"]["members"] == ["crates/*"]
        assert "pltrs-spec" not in manifest["workspace"]["dependencies"]
        lock = tomllib.loads(archive.extractfile(f"{stem}/Cargo.lock").read().decode())
        assert "xtask" not in {package["name"] for package in lock["package"]}


def test_the_sdist_rewrites_fail_loudly():
    # A members line written otherwise than the rewrite expects.
    with pytest.raises(ValueError, match="Cargo.toml"):
        _sdist.workspace_manifest('[workspace]\nmembers=["a", "xtask"]\n', ["xtask"])
    assert _sdist.workspace_manifest(
        '[workspace]\nmembers = [\n  "crates/*",\n  "xtask",\n]\n', ["xtask"]) == \
        '[workspace]\nmembers = ["crates/*"]\n'
    lock = ('version = 4\n\n[[package]]\nname = "a"\nversion = "0.1.0"\n'
            'dependencies = [\n "xtask",\n]\n\n[[package]]\nname = "xtask"\nversion = "0.1.0"\n')
    with pytest.raises(ValueError, match="Cargo.lock"):
        _sdist.lock_file(lock, ["xtask"])
    assert "xtask" not in _sdist.lock_file(lock.replace(' "xtask",\n', ""), ["xtask"])


def test_audits_find_what_is_wrong(tmp_path):
    wheel = WheelWriter("pkg", "1.0")
    wheel.add("pkg/__init__.py", b"")
    wheel.add_dist_info("METADATA", b"Metadata-Version: 2.4\nName: pkg\nVersion: 1.1\n")
    name = wheel.write(tmp_path, ["py3-none-linux_x86_64"], pure=True)
    assert _audit.wheel_problems(tmp_path / name) == [
        "Root-Is-Purelib does not match the platform tag",
        "the metadata's version 1.1 is not the file name's 1.0",
        "a pure wheel is tagged py3-none-linux_x86_64"]
    with zipfile.ZipFile(tmp_path / name, "a") as archive:
        archive.writestr("pkg/extra.py", b"")
    assert "RECORD misses pkg/extra.py" in _audit.wheel_problems(tmp_path / name)
    assert _audit.rust_target("manylinux_2_17_aarch64") == "aarch64-unknown-linux-gnu"
    assert _audit.rust_target("musllinux_1_2_x86_64") == "x86_64-unknown-linux-musl"
    assert _audit.rust_target("macosx_10_12_universal2") == "universal2"
    assert _audit.rust_target("macosx_10_12_x86_64") == "x86_64-apple-darwin"
    assert _audit.rust_target("macosx_11_0_arm64") == "aarch64-apple-darwin"
    assert _audit.rust_target("linux_x86_64") == "x86_64-unknown-linux-gnu"
    assert _audit.rust_target("win_arm64") == "aarch64-pc-windows-msvc"


def test_linking_against_an_old_glibc_or_musl_takes_zig(tmp_path, monkeypatch):
    host = "x86_64-unknown-linux-gnu"
    monkeypatch.delenv("CARGO_ENCODED_RUSTFLAGS", raising=False)
    monkeypatch.setenv("RUSTFLAGS", "-C debuginfo=1")
    assert _linker.settings(None, host, None, tmp_path) == _linker.Link(None)
    assert _linker.settings(host, host, None, tmp_path) == _linker.Link(None)
    monkeypatch.setenv("ZIG", "/opt/zig/zig")
    # The host is not named to cargo, which would rename the functions the
    # extension's symbol order names.
    native = _linker.settings(host, host, "2.17", tmp_path)
    assert native.target is None
    assert native.rustc_args == ("-C", f"linker={tmp_path / 'zig-cc-x86_64-linux-gnu.2.17'}")
    link = _linker.settings("aarch64-unknown-linux-gnu", host, "2.17", tmp_path)
    assert link.target == "aarch64-unknown-linux-gnu" and not link.environment
    shim = (tmp_path / "zig-cc-aarch64-linux-gnu.2.17").read_text()
    assert link.rustc_args == ("-C", f"linker={tmp_path / 'zig-cc-aarch64-linux-gnu.2.17'}")
    assert shim.endswith("/opt/zig/zig aarch64-linux-gnu.2.17 \"$@\"\n")
    musl = _linker.settings("x86_64-unknown-linux-musl", host, None, tmp_path)
    assert musl.environment == {"CARGO_ENCODED_RUSTFLAGS": "\x1f".join(
        ["-C", "debuginfo=1", *_linker.MUSL_RUSTFLAGS])}
    assert (tmp_path / "zig-cc-x86_64-linux-musl").read_text().endswith(
        "x86_64-linux-musl \"$@\"\n")
    with pytest.raises(ValueError, match="not a glibc version"):
        _linker.settings(None, host, "2.x", tmp_path)
    with pytest.raises(ValueError, match="glibc only"):
        _linker.settings("x86_64-unknown-linux-musl", host, "2.17", tmp_path)
    monkeypatch.delenv("ZIG")
    monkeypatch.setenv("PATH", str(tmp_path / "nowhere"))
    with pytest.raises(RuntimeError, match="needs Zig"):
        _linker.settings(None, host, "2.17", tmp_path)
    alpine = _linker.settings(None, "aarch64-unknown-linux-musl", None, tmp_path)
    assert alpine.rustc_args == ("-C", "linker=cc") and alpine.target is None


def test_the_extension_is_built_as_xtask_builds_it_without_debug_information(
        tmp_path, monkeypatch):
    # The profile leaves the debug information out (the MSVC link-time
    # optimisation's memory), and the backend overrides none of it: a
    # profile setting changes the hash in every mangled name, and the
    # function order, written from `cargo xtask python --release`, would
    # name none of the wheel's functions.
    with open(ROOT / "Cargo.toml", "rb") as file:
        profile = tomllib.load(file)["profile"]["extension"]
    assert profile["debug"] is False and profile["strip"] == "debuginfo"
    calls = []

    def cargo(command, **kwargs):
        calls.append((command, kwargs["env"]))
        library = tmp_path / "pltrs_python.dll"
        message = {"reason": "compiler-artifact", "filenames": [str(library)],
                   "target": {"name": "pltrs_python", "kind": ["cdylib"]}}
        return types.SimpleNamespace(returncode=0, stdout=json.dumps(message))

    monkeypatch.setattr(_cargo.subprocess, "run", cargo)
    monkeypatch.setenv("CARGO", "cargo")
    monkeypatch.delenv("CARGO_PROFILE_EXTENSION_DEBUG", raising=False)
    assert _cargo._build_one(tmp_path, _linker.Link(None)) == tmp_path / "pltrs_python.dll"
    (command, environment), = calls
    assert command[:5] == ["cargo", "rustc", "--profile", "extension", "-p"]
    assert not any("strip" in argument or "debug" in argument for argument in command)
    assert not [key for key in environment if key.startswith("CARGO_PROFILE_")]


def test_msvc_links_a_deterministic_extension(tmp_path):
    for target, host in [(None, "x86_64-pc-windows-msvc"),
                         ("aarch64-pc-windows-msvc", "x86_64-pc-windows-msvc"),
                         ("x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu")]:
        assert _linker.settings(target, host, None, tmp_path) == _linker.Link(
            target, ("-C", "link-arg=/Brepro"))
    assert _linker.settings(None, "x86_64-pc-windows-gnu", None, tmp_path).rustc_args == ()


def test_the_zig_linker_takes_what_rustc_passes(tmp_path):
    response = tmp_path / "linker-arguments"
    response.write_text("-o\nout\\ dir/lib.so\n-lgcc_s\n", encoding="utf-8")
    arguments = list(_zigcc.expanded(["-fuse-ld=lld", "-B/x/gcc-ld", "-m64", f"@{response}",
                                      "-Wl,-O1", "-Wl,--fix-cortex-a53-843419", "-shared",
                                      "-Wl,--symbol-ordering-file=/x/symbol-order.txt",
                                      "-Wl,--no-warn-symbol-ordering"]))
    assert list(_zigcc.filtered(arguments)) == ["-o", "out dir/lib.so", "-lunwind", "-shared"]
    # What Zig's driver refuses, LLD gets when the library is linked again.
    assert [flag for flag in map(_zigcc.relinked, arguments) if flag] == [
        "--fix-cortex-a53-843419", "--symbol-ordering-file=/x/symbol-order.txt",
        "--no-warn-symbol-ordering"]
    report = ("ld.lld -o /c/libc.so.6 x.o\n"
              "ld.lld --eh-frame-hdr -shared -o out/lib.so a.o --as-needed /c/libc.so.6\n")
    assert _zigcc.lld_command(report, "out/lib.so") == [
        "--eh-frame-hdr", "-shared", "-o", "out/lib.so", "a.o", "--as-needed", "/c/libc.so.6"]
    assert _zigcc.lld_command(report, "other.so") is None
