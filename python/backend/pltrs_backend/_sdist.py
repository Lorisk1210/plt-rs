"""
Source distributions: the part of the source tree a wheel is built from
(the Cargo workspace with every member, the Python package, the alias,
this backend, the licences), in a reproducible ``.tar.gz`` with its
``PKG-INFO``. The conformance spec's goldens (``spec/``) and the
benchmarks stay out: building needs neither.
"""

import calendar
import gzip
import io
import os
import tarfile

from ._wheel import archive_date, normalise

#: The files and directories of the tree an sdist holds.
INCLUDED = (
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rustfmt.toml",
    "clippy.toml", "pyproject.toml", "README.md", "THIRD_PARTY.md",
    ".cargo", "LICENSES", "crates", "examples/rust", "xtask", "python",
)

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
            dirs[:] = sorted(d for d in dirs if d not in EXCLUDED_NAMES)
            files.extend(
                os.path.join(directory, name) for name in names
                if not name.endswith(".pyc") and not name.startswith("_native."))
    return sorted(os.path.relpath(file, root).replace(os.sep, "/") for file in files)


def write_sdist(project, directory):
    """Writes the sdist of *project* into *directory* and returns its file
    name."""
    stem = f"{normalise(project.name)}-{project.version}"
    filename = f"{stem}.tar.gz"
    mtime = calendar.timegm((*archive_date(), 0, 0, 0))

    def entry(name, data, mode=0o644):
        info = tarfile.TarInfo(f"{stem}/{name}")
        info.size, info.mode, info.mtime = len(data), mode, mtime
        info.uname = info.gname = ""
        return info, io.BytesIO(data)

    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for name in _included(project.root):
            path = project.root / name
            executable = os.access(path, os.X_OK)
            archive.addfile(*entry(name, path.read_bytes(), 0o755 if executable else 0o644))
        archive.addfile(*entry("PKG-INFO", project.metadata().encode()))
    with open(os.path.join(directory, filename), "wb") as file:
        with gzip.GzipFile(fileobj=file, mode="wb", mtime=mtime, filename="") as zipped:
            zipped.write(buffer.getvalue())
    return filename
