"""
pltrs' build backend (PEP 517), in Python's standard library only: no
setuptools, maturin or other build requirement, so that
``pip install .`` and ``pip wheel .`` need nothing but Python and cargo.

It builds one of two distributions of the source tree, chosen by the
config setting ``distribution`` (see ``_project``):

* ``pltrs`` (the default): ``cargo build --profile extension -p pltrs-python``
  (``-C target=<Rust target>`` or ``CARGO_BUILD_TARGET`` to cross-build,
  ``-C target=universal2`` for both macOS architectures), and a wheel of
  ``python/pltrs`` holding the library as ``pltrs/_native.abi3.so``
  (``_native.pyd`` on Windows), tagged ``cp312-abi3-<platform>``: the
  stable ABI of CPython 3.12 and later, on the platform the library was
  built for (see ``_tags``);
* ``matplotlib``: a pure-Python wheel (``py3-none-any``) of the aliases
  in ``python/alias``, ``pip install . -C distribution=matplotlib``.

The source tree is the working directory, as PEP 517 runs hooks.
"""

import os
import pathlib
import tempfile

from . import _cargo, _tags
from ._project import Project
from ._sdist import write_sdist
from ._wheel import WheelWriter, dist_info_name

__all__ = [
    "build_sdist", "build_wheel", "get_requires_for_build_sdist",
    "get_requires_for_build_wheel", "prepare_metadata_for_build_wheel",
]

#: The Python and ABI tags of pltrs' extension: CPython's stable ABI
#: from 3.12 on.
ABI3 = "cp312-abi3"


def _project(config_settings):
    """The distribution the config settings ask for, of the source tree in
    the working directory."""
    name = (config_settings or {}).get("distribution", "pltrs")
    return Project(pathlib.Path.cwd(), name)


def get_requires_for_build_wheel(config_settings=None):
    """Nothing: the backend needs no package to build."""
    return []


def get_requires_for_build_sdist(config_settings=None):
    """Nothing: the backend needs no package to build."""
    return []


def prepare_metadata_for_build_wheel(metadata_directory, config_settings=None):
    """Writes the ``.dist-info`` directory's ``METADATA`` without building
    anything, and returns the directory's name."""
    project = _project(config_settings)
    name = dist_info_name(project.name, project.version)
    directory = pathlib.Path(metadata_directory) / name
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "METADATA").write_text(project.metadata(), encoding="utf-8")
    return name


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    """Builds the wheel of the distribution asked for into
    *wheel_directory* and returns its file name."""
    project = _project(config_settings)
    wheel = WheelWriter(project.name, project.version)
    for archive_path, path in project.package_files():
        wheel.add(archive_path, path.read_bytes())
    wheel.add_dist_info("METADATA", project.metadata().encode())
    for relative in project.license_files():
        wheel.add_dist_info(f"licenses/{relative}", (project.root / relative).read_bytes())
    if project.name == "matplotlib":
        return wheel.write(wheel_directory, ["py3-none-any"], pure=True)
    requested = _cargo.requested_target(config_settings)
    with tempfile.TemporaryDirectory() as scratch:
        library = _cargo.build_extension(project.root, requested, scratch).read_bytes()
    target = requested or _cargo.host_target(project.root)
    wheel.add(f"pltrs/{_cargo.extension_name(target)}", library, executable=True)
    platforms = _tags.platform_tags(
        target, library, os.environ.get("MACOSX_DEPLOYMENT_TARGET"))
    return wheel.write(wheel_directory, [f"{ABI3}-{platform}" for platform in platforms],
                       pure=False)


def build_sdist(sdist_directory, config_settings=None):
    """Builds the source distribution into *sdist_directory* and returns
    its file name."""
    return write_sdist(_project(config_settings), sdist_directory)
