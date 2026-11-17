"""
Source distributions: the part of the source tree a wheel is built from,
in a reproducible ``.tar.gz`` with its ``PKG-INFO``:

* the Cargo workspace the extension needs, with every file of its crates
  (the fonts and data they embed, their tests), ``Cargo.lock``,
  ``rust-toolchain.toml`` and cargo's configuration;
* the Python package, the alias, this backend, pltrs' smoke tests and
  tools, and the licences.

The conformance harness (the crate ``pltrs-spec`` and ``spec/`` with its
goldens), ``xtask``, the Rust examples and the benchmarks stay out:
building needs none of them. The workspace's manifest and lock file are
rewritten to leave those members out, and checked to name none of them
afterwards, so that ``cargo build --locked`` works offline in the
unpacked archive.
"""

import calendar
import gzip
import io
import os
import re
import tarfile
import tomllib

from ._wheel import archive_date, normalise

#: The files and directories of the tree an sdist holds.
INCLUDED = (
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml",
    "clippy.toml", "pyproject.toml", "README.md", "LICENSE-MIT", "LICENSE-APACHE",
    "THIRD_PARTY.md", "RELEASING.md", "DIFFERENCES.md",
    ".cargo", "LICENSES", "crates", "python/pltrs", "python/alias", "python/backend",
    "python/share",
    "python/tests", "python/tools", "python/DESIGN.md",
)

#: The workspace members an sdist leaves out.
DROPPED_MEMBERS = ("crates/pltrs-spec", "xtask", "examples/rust")

#: Names never included, wherever they are.
EXCLUDED_NAMES = {"target", "__pycache__", ".pytest_cache", "result_images"}


def _included(root):
    """The files of the sdist, relative to *root*, sorted."""
    files = []
    for entry in INCLUDED:
        path = root / entry
        if path.is_file():
            files.append(path)
            continue
        for directory, dirs, names in os.walk(path):
            relative = os.path.relpath(directory, root).replace(os.sep, "/")
            dirs[:] = sorted(d for d in dirs if d not in EXCLUDED_NAMES
                             and f"{relative}/{d}" not in DROPPED_MEMBERS)
            files.extend(
                os.path.join(directory, name) for name in names
                if not name.endswith(".pyc") and not name.startswith("_native."))
    return sorted(os.path.relpath(file, root).replace(os.sep, "/") for file in files)


def _executable(path, data):
    """Whether the file at *path*, holding *data*, is executable: by its
    mode, or on Windows, which keeps none, by its ``#!`` line (the tree's
    executables are its scripts)."""
    if os.name == "nt":
        return data.startswith(b"#!")
    return os.access(path, os.X_OK)


def _dropped_packages(root):
    """The package names of :data:`DROPPED_MEMBERS` (those *root* has:
    an unpacked sdist has none)."""
    names = []
    for member in DROPPED_MEMBERS:
        manifest = root / member / "Cargo.toml"
        if manifest.is_file():
            with open(manifest, "rb") as file:
                names.append(tomllib.load(file)["package"]["name"])
    return names


def workspace_manifest(text, dropped):
    """The workspace's ``Cargo.toml`` *text* without the members
    :data:`DROPPED_MEMBERS` and the workspace dependencies on the
    packages *dropped*."""
    members = tomllib.loads(text)["workspace"]["members"]
    kept = ", ".join(f'"{member}"' for member in members if member not in DROPPED_MEMBERS)
    text, count = re.subn(r"^members = \[[^\]]*\]$", f"members = [{kept}]", text,
                          flags=re.MULTILINE)
    for name in dropped:
        text = re.sub(rf"^{re.escape(name)} = \{{[^\n]*\}}\n", "", text, flags=re.MULTILINE)
    workspace = tomllib.loads(text)["workspace"]
    if count != 1 or set(workspace.get("dependencies", {})) & set(dropped):
        raise ValueError("pltrs: cannot leave the conformance harness out of Cargo.toml")
    return text


def lock_file(text, dropped):
    """The ``Cargo.lock`` *text* without the packages *dropped*."""
    blocks = text.split("\n\n")
    kept = [block for block in blocks
            if not any(f'\nname = "{name}"\n' in f"{block}\n" for name in dropped)]
    text = "\n\n".join(kept)
    packages = tomllib.loads(text).get("package", [])
    named = {package["name"] for package in packages}
    used = {dependency.split()[0] for package in packages
            for dependency in package.get("dependencies", [])}
    if named & set(dropped) or used & set(dropped):
        raise ValueError("pltrs: cannot leave the conformance harness out of Cargo.lock")
    return text


def write_sdist(project, directory):
    """Writes the sdist of *project* into *directory* and returns its file
    name."""
    stem = f"{normalise(project.name)}-{project.version}"
    filename = f"{stem}.tar.gz"
    mtime = calendar.timegm((*archive_date(), 0, 0, 0))
    dropped = _dropped_packages(project.root)
    rewrites = {"Cargo.toml": workspace_manifest, "Cargo.lock": lock_file}

    def entry(name, data, mode=0o644):
        info = tarfile.TarInfo(f"{stem}/{name}")
        info.size, info.mode, info.mtime = len(data), mode, mtime
        info.uname = info.gname = ""
        return info, io.BytesIO(data)

    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for name in _included(project.root):
            data = (project.root / name).read_bytes()
            executable = _executable(project.root / name, data)
            if name in rewrites:
                data = rewrites[name](data.decode("utf-8"), dropped).encode("utf-8")
            archive.addfile(*entry(name, data, 0o755 if executable else 0o644))
        archive.addfile(*entry("PKG-INFO", project.metadata().encode()))
    with open(os.path.join(directory, filename), "wb") as file:
        with gzip.GzipFile(fileobj=file, mode="wb", mtime=mtime, filename="") as zipped:
            zipped.write(buffer.getvalue())
    return filename
