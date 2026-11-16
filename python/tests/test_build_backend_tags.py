"""
The platform tags pltrs' build backend gives a wheel, read from the
library it holds (``python/backend``: ``_elf``, ``_manylinux``,
``_macho``, ``_pe``), on libraries made byte by byte (``binaries``) and
on the extension where it is built.
"""

import pathlib
import sys

import pytest

import binaries

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not (ROOT / "python" / "backend").is_dir():
    pytest.skip("the build backend is in the source tree only", allow_module_level=True)
sys.path.insert(0, str(ROOT / "python" / "backend"))

from pltrs_backend import _audit, _elf, _macho, _manylinux, _pe, _tags  # noqa: E402
from pltrs_backend._wheel import WheelWriter  # noqa: E402

GNU = "x86_64-unknown-linux-gnu"


def glibc_library(*versions, symbols=(), needed=("libc.so.6",), machine=62):
    """A library needing glibc's *versions* (``(name, weak)``), each
    used by one symbol, and the extra *symbols*."""
    return binaries.elf(
        machine=machine, needed=needed,
        versions=[("libc.so.6", name, weak) for name, weak in versions],
        symbols=[(f"f_{name}", name, weak) for name, weak in versions] + list(symbols))


def test_elf_libraries_list_needs_versions_and_undefined_symbols():
    data = binaries.elf(
        needed=("libc.so.6", "libgcc_s.so.1"),
        versions=[("libc.so.6", "GLIBC_2.17", False), ("libc.so.6", "GLIBC_2.39", True),
                  ("libgcc_s.so.1", "GCC_4.2.0", False)],
        symbols=[("memcpy", "GLIBC_2.17", False), ("pidfd_spawnp", "GLIBC_2.39", True),
                 ("_Unwind_Resume", "GCC_4.2.0", False), ("PyList_New", None, False),
                 ("__gmon_start__", None, True)])
    elf = _elf.parse(data)
    assert elf.arch == "x86_64"
    assert elf.needed == ("libc.so.6", "libgcc_s.so.1")
    assert [(v.library, v.name, v.weak) for v in elf.versions] == [
        ("libc.so.6", "GLIBC_2.17", False), ("libc.so.6", "GLIBC_2.39", True),
        ("libgcc_s.so.1", "GCC_4.2.0", False)]
    undefined = {symbol.name: symbol for symbol in elf.undefined}
    assert undefined["memcpy"].version.name == "GLIBC_2.17"
    assert undefined["pidfd_spawnp"].weak and undefined["PyList_New"].version is None
    assert _elf.parse(binaries.elf(machine=183)).arch == "aarch64"
    assert _elf.parse(binaries.elf(machine=21, little=False)).arch == "ppc64"
    assert _elf.parse(binaries.elf(machine=21)).arch == "ppc64le"
    with pytest.raises(ValueError, match="not an ELF"):
        _elf.parse(b"MZ")


def test_the_oldest_glibc_policy_the_library_meets_names_the_tag():
    assert _tags.platform_tags(GNU, glibc_library(("GLIBC_2.2.5", False), ("GLIBC_2.17", False))) \
        == ["manylinux_2_17_x86_64", "manylinux2014_x86_64"]
    assert _tags.platform_tags(GNU, glibc_library(("GLIBC_2.28", False))) == [
        "manylinux_2_28_x86_64"]
    # No policy for glibc 2.30: the next one's.
    assert _tags.platform_tags(GNU, glibc_library(("GLIBC_2.30", False))) == [
        "manylinux_2_31_x86_64"]
    # A version only weak references need does not count.
    assert _tags.platform_tags(GNU, glibc_library(("GLIBC_2.17", False), ("GLIBC_2.39", True))) \
        == ["manylinux_2_17_x86_64", "manylinux2014_x86_64"]
    aarch64 = glibc_library(("GLIBC_2.17", False), machine=183)
    assert _tags.platform_tags("aarch64-unknown-linux-gnu", aarch64) == [
        "manylinux_2_17_aarch64", "manylinux2014_aarch64"]


def test_libgcc_versions_and_forbidden_symbols_raise_the_policy():
    def with_gcc(version, machine=62):
        return binaries.elf(machine=machine, needed=("libc.so.6", "libgcc_s.so.1"),
                            versions=[("libgcc_s.so.1", version, False)],
                            symbols=[("_Unwind_Resume", version, False)])
    assert _tags.platform_tags(GNU, with_gcc("GCC_4.8.0"))[0] == "manylinux_2_17_x86_64"
    assert _tags.platform_tags(GNU, with_gcc("GCC_7.0.0"))[0] == "manylinux_2_27_x86_64"
    assert _tags.platform_tags("aarch64-unknown-linux-gnu", with_gcc("GCC_7.0.0", 183))[0] == \
        "manylinux_2_26_aarch64"
    # CentOS 7's glibc has symbols glibc 2.17 lacks: only weak references
    # to them are allowed in manylinux2014.
    strong = binaries.elf(versions=[("libc.so.6", "GLIBC_2.17", False)],
                          symbols=[("__issignaling", "GLIBC_2.17", False)])
    assert _tags.platform_tags(GNU, strong) == ["manylinux_2_24_x86_64"]
    weak = binaries.elf(versions=[("libc.so.6", "GLIBC_2.17", False)],
                        symbols=[("__cxa_thread_atexit_impl", None, True),
                                 ("memcpy", "GLIBC_2.17", False)])
    assert _tags.platform_tags(GNU, weak)[0] == "manylinux_2_17_x86_64"


def test_libraries_no_policy_allows_get_the_plain_linux_tag(capsys):
    foreign = glibc_library(("GLIBC_2.17", False), needed=("libc.so.6", "libpng16.so.16"))
    assert _tags.platform_tags(GNU, foreign) == ["linux_x86_64"]
    assert "libpng16.so.16" in capsys.readouterr().err
    private = glibc_library(("GLIBC_PRIVATE", False))
    assert _tags.platform_tags(GNU, private) == ["linux_x86_64"]
    cxx = binaries.elf(needed=("libstdc++.so.6",), versions=[("libstdc++.so.6", "GLIBCXX_3.4", False)],
                       symbols=[("_ZdlPv", "GLIBCXX_3.4", False)])
    assert _tags.platform_tags(GNU, cxx) == ["linux_x86_64"]
    assert _manylinux.policy_problems(_elf.parse(cxx), 17) == [
        "GLIBCXX_3.4 of libstdc++.so.6 (a version this check does not know)"]


def test_unversioned_symbols_the_c_library_lacks_stop_the_build():
    # What linking memfd_create against glibc 2.17's symbols leaves.
    data = glibc_library(("GLIBC_2.17", False), symbols=[
        ("memfd_create", None, False), ("PyFloat_FromDouble", None, False),
        ("_Py_NoneStruct", None, False), ("statx", None, True)])
    with pytest.raises(RuntimeError, match="would not load: memfd_create$"):
        _tags.platform_tags(GNU, data)


def test_musl_libraries_are_musllinux(capsys):
    musl = binaries.elf(needed=("libc.so",))
    assert _tags.platform_tags("x86_64-unknown-linux-musl", musl) == ["musllinux_1_2_x86_64"]
    alpine = binaries.elf(machine=183, needed=("libc.musl-aarch64.so.1",))
    assert _tags.platform_tags("aarch64-unknown-linux-musl", alpine) == [
        "musllinux_1_2_aarch64"]
    assert _tags.platform_tags("x86_64-unknown-linux-musl",
                               binaries.elf(needed=("libc.so", "libz.so.1"))) == ["linux_x86_64"]
    assert "libz.so.1" in capsys.readouterr().err


def test_a_library_of_another_architecture_is_refused():
    with pytest.raises(ValueError, match="built for aarch64, not x86_64"):
        _tags.platform_tags(GNU, binaries.elf(machine=183))
    with pytest.raises(ValueError, match="no wheel platform"):
        _tags.platform_tags("wasm32-unknown-unknown", b"")


def test_macos_tags_come_from_each_slice():
    arm64 = binaries.macho(binaries.ARM64, (11, 0))
    x86_64 = binaries.macho(binaries.X86_64, (10, 12))
    assert _tags.platform_tags("aarch64-apple-darwin", arm64) == ["macosx_11_0_arm64"]
    assert _tags.platform_tags("x86_64-apple-darwin", x86_64) == ["macosx_10_12_x86_64"]
    legacy = binaries.macho(binaries.X86_64, (10, 9), legacy=True)
    assert _tags.platform_tags("x86_64-apple-darwin", legacy) == ["macosx_10_9_x86_64"]
    both = binaries.fat((binaries.X86_64, x86_64), (binaries.ARM64, arm64))
    assert _tags.platform_tags("universal2", both) == ["macosx_10_12_universal2"]
    newer_arm = binaries.fat((binaries.X86_64, x86_64),
                             (binaries.ARM64, binaries.macho(binaries.ARM64, (13, 0))))
    assert _tags.platform_tags("universal2", newer_arm) == ["macosx_13_0_universal2"]
    # pip knows only <major>_0 from macOS 11 on: 11.3 rounds up.
    point = binaries.macho(binaries.ARM64, (11, 3))
    assert _tags.platform_tags("aarch64-apple-darwin", point) == ["macosx_12_0_arm64"]


def test_macos_libraries_must_be_the_systems_and_name_their_macos():
    homebrew = binaries.macho(binaries.ARM64, dylibs=(
        "/usr/lib/libSystem.B.dylib", "/opt/homebrew/lib/libpng16.16.dylib",
        "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit"))
    with pytest.raises(RuntimeError, match="lacks: /opt/homebrew/lib/libpng16.16.dylib$"):
        _tags.platform_tags("aarch64-apple-darwin", homebrew)
    with pytest.raises(ValueError, match="names no macOS version"):
        _tags.platform_tags("aarch64-apple-darwin", binaries.macho(binaries.ARM64, None))
    with pytest.raises(ValueError, match="holds arm64, not arm64, x86_64"):
        _tags.platform_tags("universal2", binaries.macho(binaries.ARM64))
    assert [piece.dylibs for piece in _macho.slices(homebrew)][0][2].endswith("AppKit")


def test_windows_tags_follow_the_architecture_and_the_stable_abi():
    assert _tags.platform_tags("x86_64-pc-windows-msvc", binaries.pe(0x8664)) == ["win_amd64"]
    assert _tags.platform_tags("aarch64-pc-windows-msvc", binaries.pe(0xAA64)) == ["win_arm64"]
    assert _tags.platform_tags("i686-pc-windows-msvc", binaries.pe(0x14C)) == ["win32"]
    assert _pe.parse(binaries.pe(dlls=("python3.dll", "VCRUNTIME140.dll")))[1] == [
        "python3.dll", "VCRUNTIME140.dll"]
    with pytest.raises(RuntimeError, match="imports python312.dll"):
        _tags.platform_tags("x86_64-pc-windows-msvc", binaries.pe(dlls=("python312.dll",)))
    with pytest.raises(RuntimeError, match="VCRUNTIME140D.dll"):
        _tags.platform_tags("x86_64-pc-windows-msvc", binaries.pe(dlls=("VCRUNTIME140D.dll",)))
    with pytest.raises(ValueError, match="built for aarch64, not x86_64"):
        _tags.platform_tags("x86_64-pc-windows-msvc", binaries.pe(0xAA64))


def test_the_built_extension_earns_the_tag_of_the_glibc_it_needs():
    extension = ROOT / "python" / "pltrs" / "_native.abi3.so"
    if not extension.exists() or sys.platform != "linux":
        pytest.skip("needs the extension built on Linux (cargo xtask python)")
    elf = _elf.parse(extension.read_bytes())
    assert "libc.so.6" in elf.needed
    newest = max(_manylinux._glibc_minor(version.name) for version in elf.versions
                 if version.name.startswith("GLIBC_2.") and not version.weak)
    tags = _tags.platform_tags(f"{elf.arch}-unknown-linux-gnu", extension.read_bytes())
    minor = int(tags[0].split("_")[2])
    assert minor >= newest and not _manylinux.policy_problems(elf, minor)


#: Libraries of each target, with the extension file they are.
TAGGED_LIBRARIES = [
    (GNU, glibc_library(("GLIBC_2.17", False)), "_native.abi3.so"),
    ("aarch64-unknown-linux-musl", binaries.elf(machine=183, needed=("libc.so",)),
     "_native.abi3.so"),
    ("x86_64-apple-darwin", binaries.macho(binaries.X86_64, (10, 12)), "_native.abi3.so"),
    ("aarch64-apple-darwin", binaries.macho(binaries.ARM64), "_native.abi3.so"),
    ("universal2", binaries.fat((binaries.X86_64, binaries.macho(binaries.X86_64, (10, 12))),
                                (binaries.ARM64, binaries.macho(binaries.ARM64))),
     "_native.abi3.so"),
    ("x86_64-pc-windows-msvc", binaries.pe(), "_native.pyd"),
]


# Named by their targets: an id spelling out a library's bytes is longer
# than Windows lets pytest's PYTEST_CURRENT_TEST variable be.
@pytest.mark.parametrize("target, library, extension", TAGGED_LIBRARIES,
                         ids=[target for target, _, _ in TAGGED_LIBRARIES])
def test_wheels_tagged_by_their_library_pass_the_audit(tmp_path, target, library, extension):
    tags = [f"cp312-abi3-{platform}" for platform in _tags.platform_tags(target, library)]
    name = wheel_of(tmp_path, library, extension, tags)
    assert _audit.wheel_problems(tmp_path / name) == []


def test_a_tag_claiming_more_than_the_library_earns_is_found(tmp_path):
    library = glibc_library(("GLIBC_2.28", False))
    name = wheel_of(tmp_path, library, "_native.abi3.so", ["cp312-abi3-manylinux_2_17_x86_64"])
    assert _audit.wheel_problems(tmp_path / name) == [
        "the extension does not earn manylinux_2_17_x86_64 (it earns manylinux_2_28_x86_64)"]


def wheel_of(directory, library, extension, tags):
    """A wheel of pltrs holding *library* as *extension*, tagged *tags*."""
    wheel = WheelWriter("pltrs", "1.0")
    wheel.add(f"pltrs/{extension}", library, executable=True)
    wheel.add_dist_info("METADATA", b"Metadata-Version: 2.4\nName: pltrs\nVersion: 1.0\n")
    return wheel.write(directory, tags, pure=False)
