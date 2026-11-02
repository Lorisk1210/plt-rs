"""
pltrs' build backend (``python/backend``): the platform tags it gives
each system, the wheels it writes and the two distributions it builds.
"""

import base64
import csv
import hashlib
import io
import pathlib
import sys
import zipfile

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not (ROOT / "python" / "backend").is_dir():
    pytest.skip("the build backend is in the source tree only", allow_module_level=True)
sys.path.insert(0, str(ROOT / "python" / "backend"))

from pltrs_backend import _tags  # noqa: E402
from pltrs_backend._project import Project  # noqa: E402
from pltrs_backend._wheel import WheelWriter, normalise  # noqa: E402


def test_windows_tags_follow_the_architecture():
    assert _tags.platform_tags("x86_64-pc-windows-msvc", b"") == ["win_amd64"]
    assert _tags.platform_tags("aarch64-pc-windows-msvc", b"") == ["win_arm64"]
    assert _tags.platform_tags("i686-pc-windows-msvc", b"") == ["win32"]


def test_macos_tags_follow_the_deployment_target():
    assert _tags.platform_tags("x86_64-apple-darwin", b"") == ["macosx_10_12_x86_64"]
    assert _tags.platform_tags("aarch64-apple-darwin", b"") == ["macosx_11_0_arm64"]
    assert _tags.platform_tags("universal2", b"") == ["macosx_10_12_universal2"]
    assert _tags.macos_tag("x86_64", "10.15") == "macosx_10_15_x86_64"
    assert _tags.macos_tag("aarch64", "10.15") == "macosx_11_0_arm64"
    assert _tags.macos_tag("aarch64", "12.3") == "macosx_12_0_arm64"
    assert _tags.macos_tag("universal2", "13") == "macosx_13_0_universal2"
    with pytest.raises(ValueError, match="not a version"):
        _tags.macos_tag("x86_64", "ventura")


def test_unknown_targets_have_no_tag():
    with pytest.raises(ValueError, match="no wheel platform"):
        _tags.platform_tags("wasm32-unknown-unknown", b"")
    with pytest.raises(ValueError, match="not an ELF"):
        _tags.platform_tags("x86_64-unknown-linux-gnu", b"MZ")


def test_linux_tags_read_the_glibc_the_library_needs():
    extension = ROOT / "python" / "pltrs" / "_native.abi3.so"
    if not extension.exists() or sys.platform != "linux":
        pytest.skip("needs the extension built on Linux (cargo xtask python)")
    needed, versions = _tags._elf_needs(extension.read_bytes())
    assert "libc.so.6" in needed
    tags = _tags.linux_tags("x86_64", extension.read_bytes())
    assert tags[0].startswith("manylinux_2_") and tags[0].endswith("_x86_64")
    newest = max(int(v.split(".")[1]) for vs in versions.values() for v in vs
                 if v.startswith("GLIBC_2."))
    assert tags[0] == f"manylinux_2_{max(newest, 17)}_x86_64"


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
    assert not [path for path in files if "_native." in path or "__pycache__" in path]
    assert sorted(dict(alias.package_files())) == [
        "_pltrs_matplotlib_alias.py", "matplotlib/__init__.py",
        "mpl_toolkits/axes_grid1/__init__.py", "mpl_toolkits/axisartist/__init__.py",
        "mpl_toolkits/mplot3d/__init__.py", "pylab.py"]
    with pytest.raises(ValueError, match="unknown distribution"):
        Project(ROOT, "seaborn")
