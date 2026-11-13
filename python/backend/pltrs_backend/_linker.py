"""
How the extension is linked, for Linux wheels that install widely:

* ``-C glibc=2.17`` (any ``2.<n>``): Zig links it (``zig cc -target
  <arch>-linux-gnu.2.17``) against that glibc's symbols, on any Linux or
  macOS host, without root, containers or a sysroot: with 2.17, a
  manylinux2014 wheel. ``-C target=aarch64-unknown-linux-gnu`` with it
  cross-links for aarch64.
* musl targets (``-C target=x86_64-unknown-linux-musl``): an extension
  must use the musl C library of the Python that loads it, so it is
  built without ``crt-static``, and without the startup files and static
  C library ``.cargo/config.toml`` gives musl programs. Zig links it, or
  on a musl host (in a musllinux container) the system's ``cc``.
* Otherwise cargo links it as it is configured to.

The host's own target is never named to cargo (``--target``): cargo then
gives the crates other metadata, which renames every function of the
workspace's crates, and the order the x86-64 Linux extension's
functions are laid out in (``crates/pltrs-python/symbol-order.txt``,
written from a build for the host) would name none of them.

Zig is the ``ZIG`` program, else ``zig`` on ``PATH``
(``python/tools/zig.sh install`` puts a pinned release in
``~/.local/zig``). It links only the extension (``cargo rustc -- -C
linker=...``): build scripts are linked for the host as usual.
"""

import dataclasses
import os
import pathlib
import re
import shlex
import shutil
import sys

#: Zig's names of Rust's Linux architectures.
ZIG_ARCHITECTURES = {
    "x86_64": "x86_64", "aarch64": "aarch64", "i686": "x86", "i586": "x86",
    "armv7": "arm", "powerpc64le": "powerpc64le", "s390x": "s390x",
    "riscv64gc": "riscv64", "loongarch64": "loongarch64",
}

#: The flags of a musl extension: musl's C library loaded dynamically.
MUSL_RUSTFLAGS = ("-C", "target-feature=-crt-static", "-C", "link-self-contained=no")


@dataclasses.dataclass(frozen=True)
class Link:
    """How cargo builds the extension: the target to pass (None for the
    host, named or not), arguments for rustc's link of the extension
    itself, and variables to set."""

    target: str | None
    rustc_args: tuple = ()
    environment: dict = dataclasses.field(default_factory=dict)


def zig_program():
    """The zig program: ``ZIG``, else ``zig`` on ``PATH``, else None."""
    return os.environ.get("ZIG") or shutil.which("zig")


def _zig_target(target, glibc):
    """Zig's name of the Rust Linux target *target*, at the glibc version
    *glibc* (None for Zig's default, or musl)."""
    arch, _, rest = target.partition("-")
    abi = rest.rsplit("-", 1)[-1]
    if arch not in ZIG_ARCHITECTURES or not abi.startswith(("gnu", "musl")):
        raise ValueError(f"pltrs: Zig cannot link the Rust target {target!r}")
    name = f"{ZIG_ARCHITECTURES[arch]}-linux-{abi}"
    return f"{name}.{glibc}" if glibc else name


def _zig_shim(directory, zig, zig_target):
    """Writes the program rustc runs as its linker into *directory*: the
    ``_zigcc`` script with this Python, for *zig* and *zig_target*."""
    script = pathlib.Path(__file__).resolve().with_name("_zigcc.py")
    command = " ".join(shlex.quote(str(part)) for part in (
        sys.executable, "-I", script, zig, zig_target))
    shim = pathlib.Path(directory) / f"zig-cc-{zig_target}"
    shim.write_text(f'#!/bin/sh\nexec {command} "$@"\n', encoding="utf-8")
    shim.chmod(0o755)
    return shim


def _rustflags_with(flags):
    """``CARGO_ENCODED_RUSTFLAGS`` of the flags the environment gives
    (``CARGO_ENCODED_RUSTFLAGS`` or ``RUSTFLAGS``) followed by *flags*."""
    encoded = os.environ.get("CARGO_ENCODED_RUSTFLAGS")
    given = encoded.split("\x1f") if encoded else os.environ.get("RUSTFLAGS", "").split()
    return "\x1f".join([*filter(None, given), *flags])


def settings(target, host, glibc, directory):
    """
    How to link the extension for the Rust target *target* (None for the
    host *host*), against glibc *glibc* (``2.<n>``, or None for the
    toolchain's own), with *directory* for the linker program.
    """
    musl = "musl" in (target or host)
    if glibc is not None:
        if not re.fullmatch(r"2\.\d+", glibc):
            raise ValueError(f"pltrs: glibc={glibc!r} is not a glibc version such as 2.17")
        if musl or "linux-gnu" not in (target or host):
            raise ValueError("pltrs: glibc= applies to Linux targets with glibc only")
    target = target or host
    cargo_target = None if target == host else target
    if glibc is None and not musl:
        return Link(cargo_target)
    environment = {"CARGO_ENCODED_RUSTFLAGS": _rustflags_with(MUSL_RUSTFLAGS)} if musl else {}
    zig = zig_program()
    if zig:
        shim = _zig_shim(directory, zig, _zig_target(target, glibc))
        return Link(cargo_target, ("-C", f"linker={shim}"), environment)
    if musl and target == host:
        return Link(None, ("-C", "linker=cc"), environment)
    raise RuntimeError(
        f"pltrs: linking for {target} {'against glibc ' + glibc if glibc else 'here'} needs "
        "Zig: python/tools/zig.sh install, then ZIG=$(python/tools/zig.sh path)")
