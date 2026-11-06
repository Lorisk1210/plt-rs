"""
Building pltrs' native extension with cargo: the optimised build of
``pltrs-python`` (``cargo rustc --profile extension -p pltrs-python``,
the release profile with fat LTO and one codegen unit, which is ``cargo
build`` with the library's debug information left out, as a wheel ships
without it), for the host or for the Rust target
``-C target=...`` (or ``CARGO_BUILD_TARGET``) names, or for both macOS
architectures at once (``-C target=universal2``: the two libraries joined
by ``lipo``), linked as ``_linker`` decides (``-C glibc=...``).

The library cargo made is found from cargo's own report of what it
built (``--message-format=json``), wherever the target directory is.
"""

import json
import os
import pathlib
import shutil
import subprocess

from . import _linker

#: The two architectures of a universal2 library.
UNIVERSAL2 = ("x86_64-apple-darwin", "aarch64-apple-darwin")

#: The file name the extension is imported by, on Windows and elsewhere.
EXTENSION_NAMES = {"windows": "_native.pyd", "other": "_native.abi3.so"}


def _cargo():
    """The cargo program: ``CARGO``, else ``cargo`` on ``PATH``."""
    cargo = os.environ.get("CARGO") or shutil.which("cargo")
    if not cargo:
        raise RuntimeError(
            "pltrs: building pltrs needs Rust's cargo on PATH "
            "(https://rustup.rs; the toolchain rust-toolchain.toml names)")
    return cargo


def host_target(root):
    """The Rust target cargo builds for by default (``rustc -vV``'s
    host)."""
    rustc = os.environ.get("RUSTC") or shutil.which("rustc") or "rustc"
    output = subprocess.run([rustc, "-vV"], cwd=root, check=True,
                            capture_output=True, text=True).stdout
    for line in output.splitlines():
        if line.startswith("host: "):
            return line[len("host: "):].strip()
    raise RuntimeError(f"pltrs: {rustc} -vV names no host")


def requested_target(config_settings):
    """The Rust target asked for (``-C target=...``, else
    ``CARGO_BUILD_TARGET``), or None for the host."""
    return (config_settings or {}).get("target") or os.environ.get("CARGO_BUILD_TARGET")


def requested_glibc(config_settings):
    """The glibc version to link against (``-C glibc=...``), or None for
    the toolchain's own."""
    return (config_settings or {}).get("glibc")


def _build_one(root, link):
    """Builds the extension as *link* (a :class:`_linker.Link`) says and
    returns the library cargo wrote."""
    command = [_cargo(), "rustc", "--profile", "extension", "-p", "pltrs-python", "--lib",
               "--message-format=json-render-diagnostics"]
    if link.target:
        command += ["--target", link.target]
    command += ["--", "-C", "strip=debuginfo", *link.rustc_args]
    # cargo's progress and diagnostics go to the frontend's error stream.
    process = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, text=True,
                             env={**os.environ, **link.environment})
    if process.returncode != 0:
        raise RuntimeError(f"pltrs: {' '.join(command)} failed")
    for line in process.stdout.splitlines():
        message = json.loads(line) if line.startswith("{") else {}
        is_extension = (message.get("reason") == "compiler-artifact"
                        and message["target"]["name"] == "pltrs_python"
                        and "cdylib" in message["target"]["kind"])
        if is_extension:
            for filename in message["filenames"]:
                if filename.endswith((".so", ".dylib", ".dll")):
                    return pathlib.Path(filename)
    raise RuntimeError("pltrs: cargo reported no pltrs_python library")


def target_directory(root):
    """cargo's target directory for the workspace at *root*."""
    output = subprocess.run(
        [_cargo(), "metadata", "--format-version", "1", "--no-deps", "--offline"],
        cwd=root, check=True, capture_output=True, text=True).stdout
    return pathlib.Path(json.loads(output)["target_directory"])


def build_extension(root, target, scratch, glibc=None):
    """
    Builds the extension for *target* (a Rust target, ``universal2``, or
    None for the host), against glibc *glibc* if given, and returns the
    library, in *scratch* when it was joined there.
    """
    if target != "universal2":
        # The linker program stays where it is from build to build, so
        # that cargo does not take a new path for a change and relink.
        programs = target_directory(root) / "pltrs-backend"
        programs.mkdir(parents=True, exist_ok=True)
        link = _linker.settings(target, host_target(root), glibc, programs)
        return _build_one(root, link)
    if glibc is not None:
        raise ValueError("pltrs: glibc= applies to Linux targets only")
    halves = [str(_build_one(root, _linker.Link(half))) for half in UNIVERSAL2]
    joined = pathlib.Path(scratch) / "libpltrs_python.dylib"
    subprocess.run(["lipo", "-create", "-output", str(joined), *halves], check=True)
    return joined


def extension_name(target):
    """The file the package imports the extension built for *target*
    from (``_native``: ``.pyd`` on Windows, ``.abi3.so`` elsewhere)."""
    return EXTENSION_NAMES["windows" if "windows" in target else "other"]
