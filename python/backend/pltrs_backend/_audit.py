"""
Checking built archives as an index and an installer read them, before
anyone installs them:

* a wheel (PEP 427, PEP 491): its file name; ``WHEEL``, whose tags must
  be the file name's; ``RECORD``, listing every file with its digest and
  size and nothing else; its metadata (``_core_metadata``), with the
  licence files it names; and its platform tags against the extension it
  holds (``_tags``), so that no tag claims more than the library
  delivers;
* an sdist (PEP 625): its file name; one top directory with
  ``PKG-INFO`` and ``pyproject.toml``; plain files on safe paths; the
  licence files its metadata names; and none of what stays out (build
  output, byte code, a built extension, the conformance spec).

The backend checks each archive it writes, and
``python/tools/check_dist.py`` the archives built elsewhere.
"""

import csv
import io
import re
import tarfile
import zipfile

from . import _core_metadata, _tags
from ._manylinux import LINUX_ARCHITECTURES
from ._wheel import normalise, record_digest

#: A wheel's file name: distribution, version, optional build, tags.
WHEEL_NAME = re.compile(
    r"(?P<name>[^-]+)-(?P<version>[^-]+)(-(?P<build>\d[^-]*))?"
    r"-(?P<python>[^-]+)-(?P<abi>[^-]+)-(?P<platform>[^-]+)\.whl")

#: The Rust targets of Windows and macOS platform tags.
TARGETS = {
    "win_amd64": "x86_64-pc-windows-msvc", "win_arm64": "aarch64-pc-windows-msvc",
    "win32": "i686-pc-windows-msvc", "universal2": "universal2",
    "arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin",
}

#: Names no sdist holds, wherever they are.
SDIST_EXCLUDED = {"target", "__pycache__", ".pytest_cache", "result_images"}


def _unsafe(path):
    """Whether the archive path *path* could land outside its directory."""
    return path.startswith("/") or "\\" in path or ".." in path.split("/")


def rust_target(platform):
    """The Rust target an extension of the platform tag *platform* is
    built for."""
    macos = re.fullmatch(r"macosx_\d+_\d+_(.+)", platform)
    if macos:
        return TARGETS[macos.group(1)]
    if platform in TARGETS:
        return TARGETS[platform]
    found = re.fullmatch(r"(manylinux_\d+_\d+|manylinux2014|musllinux_\d+_\d+|linux)_(.+)",
                         platform)
    if found is None:
        raise ValueError(f"pltrs: unknown platform tag {platform}")
    arches = {tag: rust for rust, tag in LINUX_ARCHITECTURES.items() if rust != "i586"}
    arch = arches.get(found.group(2), found.group(2))
    libc = "musl" if found.group(1).startswith("musl") else "gnu"
    if arch == "armv7":
        libc += "eabihf"
    return f"{arch}-unknown-linux-{libc}"


def _record_problems(archive, record_path):
    """What ``RECORD`` gets wrong about the wheel *archive*."""
    rows = list(csv.reader(io.StringIO(archive.read(record_path).decode("utf-8"))))
    listed = {}
    found = []
    for row in rows:
        if len(row) != 3:
            found.append(f"RECORD row {row!r} has not three fields")
            continue
        if row[0] in listed:
            found.append(f"RECORD lists {row[0]} twice")
        listed[row[0]] = row
    for name in archive.namelist():
        if name.endswith("/"):
            continue
        row = listed.pop(name, None)
        if row is None:
            found.append(f"RECORD misses {name}")
        elif name == record_path:
            if row[1:] != ["", ""]:
                found.append("RECORD lists a digest of itself")
        else:
            data = archive.read(name)
            if row[1:] != [record_digest(data), str(len(data))]:
                found.append(f"RECORD has the wrong digest or size of {name}")
    found += [f"RECORD lists {name}, which the wheel lacks" for name in listed]
    return found


def _tag_problems(archive, tags, pure):
    """What is wrong with the platform tags *tags* of a wheel holding
    *archive*'s extension (if *pure* is false)."""
    extensions = [name for name in archive.namelist()
                  if name.rsplit("/", 1)[-1] in ("_native.abi3.so", "_native.pyd")]
    if pure:
        return [f"a pure wheel holds {name}" for name in extensions] + [
            f"a pure wheel is tagged {tag}" for tag in tags if not tag.endswith("-any")]
    if len(extensions) != 1:
        return [f"the wheel holds {len(extensions)} extensions"]
    platforms = {tag.split("-")[2] for tag in tags}
    library = archive.read(extensions[0])
    found = []
    for platform in sorted(platforms):
        earned = set(_tags.platform_tags(rust_target(platform), library))
        if platform not in earned:
            found.append(f"the extension does not earn {platform} (it earns "
                         f"{', '.join(sorted(earned))})")
    return found


def wheel_problems(path):
    """What is wrong with the wheel at *path*, as sentences: empty when
    it holds."""
    filename = str(path).replace("\\", "/").rsplit("/", 1)[-1]
    parts = WHEEL_NAME.fullmatch(filename)
    if parts is None:
        return [f"{filename} is not a wheel's file name"]
    tags = [f"{python}-{abi}-{platform}" for python in parts["python"].split(".")
            for abi in parts["abi"].split(".") for platform in parts["platform"].split(".")]
    dist_info = f"{parts['name']}-{parts['version']}.dist-info"
    found = []
    with zipfile.ZipFile(path) as archive:
        names = set(archive.namelist())
        found += [f"unsafe path {name}" for name in names if _unsafe(name)]
        required = [f"{dist_info}/{file}" for file in ("METADATA", "WHEEL", "RECORD")]
        missing = [name for name in required if name not in names]
        if missing:
            return found + [f"no {name}" for name in missing]
        wheel = archive.read(f"{dist_info}/WHEEL").decode("utf-8").splitlines()
        if "Wheel-Version: 1.0" not in wheel:
            found.append("WHEEL names no Wheel-Version 1.0")
        if sorted(line[5:] for line in wheel if line.startswith("Tag: ")) != sorted(tags):
            found.append("WHEEL's tags are not the file name's")
        pure = "Root-Is-Purelib: true" in wheel
        if pure != (parts["platform"] == "any"):
            found.append("Root-Is-Purelib does not match the platform tag")
        found += _record_problems(archive, f"{dist_info}/RECORD")
        metadata = archive.read(f"{dist_info}/METADATA").decode("utf-8")
        found += [f"METADATA: {problem}" for problem in _core_metadata.problems(metadata)]
        found += _identity_problems(metadata, parts["name"], parts["version"])
        found += [f"no licence file {dist_info}/licenses/{name}"
                  for name in _license_files(metadata)
                  if f"{dist_info}/licenses/{name}" not in names]
        found += _tag_problems(archive, tags, pure)
    return found


def _license_files(metadata):
    """The ``License-File`` values of *metadata*."""
    return _core_metadata.parse(metadata).get_all("License-File") or []


def _identity_problems(metadata, name, version):
    """Whether *metadata* names the distribution *name* (normalised) at
    *version*."""
    message = _core_metadata.parse(metadata)
    found = []
    if normalise(message.get("Name", "")) != name:
        found.append(f"the metadata names {message.get('Name')}, the file name {name}")
    if message.get("Version") != version:
        found.append(f"the metadata's version {message.get('Version')} is not the file "
                     f"name's {version}")
    return found


def _out_of_place(inner):
    """Whether the sdist path *inner* (its parts below the top directory)
    is something no sdist holds."""
    if not inner:
        return False
    return (bool(SDIST_EXCLUDED & set(inner)) or inner[0] == "spec"
            or inner[-1].endswith(".pyc") or inner[-1].startswith("_native."))


def sdist_problems(path):
    """What is wrong with the sdist at *path*, as sentences: empty when
    it holds."""
    filename = str(path).replace("\\", "/").rsplit("/", 1)[-1]
    parts = re.fullmatch(r"(?P<name>[^-]+)-(?P<version>[^-]+)\.tar\.gz", filename)
    if parts is None:
        return [f"{filename} is not an sdist's file name"]
    stem = f"{parts['name']}-{parts['version']}"
    found = []
    with tarfile.open(path, "r:gz") as archive:
        members = archive.getmembers()
        names = {member.name for member in members}
        for member in members:
            if not member.name.startswith(f"{stem}/") or _unsafe(member.name):
                found.append(f"{member.name} is outside {stem}/")
            elif not (member.isfile() or member.isdir()):
                found.append(f"{member.name} is not a plain file")
            if _out_of_place(member.name.split("/")[1:]):
                found.append(f"{member.name} does not belong in an sdist")
        for required in ("PKG-INFO", "pyproject.toml"):
            if f"{stem}/{required}" not in names:
                found.append(f"no {required}")
        if f"{stem}/PKG-INFO" in names:
            metadata = archive.extractfile(f"{stem}/PKG-INFO").read().decode("utf-8")
            found += [f"PKG-INFO: {problem}" for problem in _core_metadata.problems(metadata)]
            found += _identity_problems(metadata, parts["name"], parts["version"])
            found += [f"no licence file {name}" for name in _license_files(metadata)
                      if f"{stem}/{name}" not in names]
    return found
