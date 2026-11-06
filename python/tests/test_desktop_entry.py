"""
pltrs' desktop entry (`pltrs.backends.backend_pltrs.install_desktop_entry`),
which GNOME's shell needs to show pltrs' icon for its windows, and the
copy Linux wheels install (``python/share``). pltrs-only: matplotlib's
windows are its toolkits', whose icons are theirs.
"""

import pathlib
import struct
import zipfile

import pytest

from pltrs.backends import backend_pltrs

#: The sides of the icons, in pixels.
SIDES = [16, 24, 32, 48, 64, 128, 256, 512]

#: The source tree, when the tests run from it.
ROOT = pathlib.Path(__file__).resolve().parents[2]


def _png_size(path):
    """The width and height of the PNG file at *path*."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n" and data[12:16] == b"IHDR"
    return struct.unpack(">II", data[16:24])


def _relative(paths, base):
    return [path.relative_to(base).as_posix() for path in paths]


def test_install_writes_the_entry_and_the_icons(tmp_path):
    written = backend_pltrs.install_desktop_entry(tmp_path)
    assert _relative(written, tmp_path) == ["applications/pltrs.desktop"] + [
        f"icons/hicolor/{side}x{side}/apps/pltrs.png" for side in SIDES]
    lines = written[0].read_text(encoding="utf-8").splitlines()
    assert lines[0] == "[Desktop Entry]"
    for line in ["Type=Application", "Name=pltrs", "Icon=pltrs",
                 "StartupWMClass=pltrs", "NoDisplay=true"]:
        assert line in lines
    assert [_png_size(path) for path in written[1:]] == [(side, side) for side in SIDES]
    assert backend_pltrs.install_desktop_entry(str(tmp_path)) == written, "replaced"
    assert backend_pltrs.remove_desktop_entry(tmp_path) == written
    assert backend_pltrs.remove_desktop_entry(tmp_path) == []
    assert not (tmp_path / "applications" / "pltrs.desktop").exists()


def test_the_default_is_the_users_data_directory(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "data"))
    written = backend_pltrs.install_desktop_entry()
    assert written[0] == tmp_path / "data" / "applications" / "pltrs.desktop"
    assert backend_pltrs.remove_desktop_entry() == written
    # A relative $XDG_DATA_HOME is ignored, as the XDG specification says.
    monkeypatch.setenv("XDG_DATA_HOME", "relative")
    monkeypatch.setenv("HOME", str(tmp_path / "home"))
    written = backend_pltrs.install_desktop_entry()
    assert written[0] == tmp_path / "home" / ".local" / "share" / "applications" / "pltrs.desktop"


def test_failures_raise_oserror(tmp_path, monkeypatch):
    (tmp_path / "applications").write_text("")
    with pytest.raises(OSError):
        backend_pltrs.install_desktop_entry(tmp_path)
    monkeypatch.delenv("XDG_DATA_HOME", raising=False)
    monkeypatch.delenv("HOME", raising=False)
    with pytest.raises(OSError, match="HOME"):
        backend_pltrs.install_desktop_entry()


@pytest.mark.skipif(not (ROOT / "python" / "share").is_dir(),
                    reason="the wheels' files are in the source tree only")
def test_the_wheels_files_are_what_install_writes(tmp_path):
    written = backend_pltrs.install_desktop_entry(tmp_path)
    shipped = ROOT / "python" / "share"
    for path in written:
        relative = path.relative_to(tmp_path)
        assert (shipped / relative).read_bytes() == path.read_bytes(), (
            f"python/share/{relative.as_posix()} is stale: run "
            "cargo run -p pltrs --example install_desktop_entry -- python/share")
    assert sorted(_relative(shipped.rglob("*.*"), shipped)) == sorted(
        _relative(written, tmp_path))


@pytest.mark.skipif(not (ROOT / "python" / "backend").is_dir(),
                    reason="the build backend is in the source tree only")
def test_linux_wheels_carry_them_under_share(tmp_path, monkeypatch):
    monkeypatch.syspath_prepend(str(ROOT / "python" / "backend"))
    from pltrs_backend._project import Project
    from pltrs_backend._wheel import WheelWriter

    files = Project(ROOT).data_files()
    assert [relative for relative, _ in files] == sorted(
        ["share/applications/pltrs.desktop"]
        + [f"share/icons/hicolor/{side}x{side}/apps/pltrs.png" for side in SIDES])
    assert Project(ROOT, "matplotlib").data_files() == []
    wheel = WheelWriter("pltrs", "1.0")
    entry, path = files[0]
    wheel.add_data(entry, path.read_bytes())
    filename = wheel.write(tmp_path, ["py3-none-any"], pure=True)
    with zipfile.ZipFile(tmp_path / filename) as archive:
        assert "pltrs-1.0.data/data/share/applications/pltrs.desktop" in archive.namelist()
