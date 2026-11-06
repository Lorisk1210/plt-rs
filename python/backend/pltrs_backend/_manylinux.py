"""
The platform tags a Linux library earns, manylinux (PEP 600) or
musllinux (PEP 656), checked the way auditwheel checks a wheel against
its policies (whose data this module condenses; see THIRD_PARTY.md):

* each library it needs (``DT_NEEDED``) is one a policy lets a wheel
  take from the system: glibc's, libgcc_s, ...;
* each symbol version it needs from them is one every distribution of
  the policy has: ``GLIBC_2.<n>`` up to the policy's glibc, ``GCC_<x>``
  of libgcc_s up to the policy's GCC; versions only weak references need
  are left out, as the library runs without those symbols;
* it refers to none of the symbols manylinux2014 forbids (CentOS 7's
  glibc has them, glibc 2.17 not), but weakly.

The tag is the oldest policy's that holds, ``manylinux_2_<n>_<arch>``,
with the legacy alias ``manylinux2014_<arch>`` for glibc 2.17. A library
that fails every policy is tagged ``linux_<arch>``, which pip installs
on the system it was built on only, and the backend says why.

Each symbol a glibc library leaves undefined must be versioned, but
Python's (``Py``, ``_Py``; the interpreter defines them) and the weak
ones: an unversioned one is a symbol the linker found in no library, so
binding it fails when the library loads (it is linked ``-z now``), and
building stops with its name.

musl has no symbol versions: a library linked against musl, needing
nothing but musl's C library, is ``musllinux_1_2_<arch>``.
"""

import re
import sys

from . import _elf

#: The manylinux names of Rust's architectures.
LINUX_ARCHITECTURES = {
    "x86_64": "x86_64", "aarch64": "aarch64", "i686": "i686", "i586": "i686",
    "powerpc64le": "ppc64le", "s390x": "s390x", "armv7": "armv7l",
    "riscv64gc": "riscv64", "loongarch64": "loongarch64",
}

#: The libraries a manylinux wheel may need from the system (auditwheel's
#: ``lib_whitelist`` but ``libz.so.1``, some of whose symbols its policies
#: forbid: pltrs needs no zlib), and the dynamic loaders.
LIBRARIES = frozenset({
    "libgcc_s.so.1", "libstdc++.so.6", "libatomic.so.1", "libm.so.6", "libanl.so.1",
    "libdl.so.2", "librt.so.1", "libc.so.6", "libnsl.so.1", "libutil.so.1",
    "libpthread.so.0", "libresolv.so.2", "libX11.so.6", "libXext.so.6",
    "libXrender.so.1", "libICE.so.6", "libSM.so.6", "libGL.so.1", "libexpat.so.1",
    "libgobject-2.0.so.0", "libgthread-2.0.so.0", "libglib-2.0.so.0",
    "ld-linux-x86-64.so.2", "ld-linux-aarch64.so.1", "ld-linux.so.2",
    "ld-linux-armhf.so.3", "ld64.so.2", "ld64.so.1", "ld-linux-riscv64-lp64d.so.1",
    "ld-linux-loongarch-lp64d.so.1",
})

#: Libraries the policies allow from glibc 2.28 on.
LIBRARIES_FROM_2_28 = frozenset({"libmvec.so.1"})

#: The glibc minor versions of auditwheel's policies from 2.17 on.
POLICIES = (17, 24, 26, 27, 28, 31, 34, 35, 36, 37, 38, 39, 40, 41, 42)

#: For each architecture, the newest ``GCC_`` version of libgcc_s its
#: policies allow, as ``(first glibc minor, version)`` steps; the first
#: step is the architecture's first policy.
NEWEST_GCC = {
    "x86_64": ((17, (4, 8, 0)), (27, (7, 0, 0)), (35, (12, 0, 0)), (39, (14, 0, 0))),
    "i686": ((17, (4, 8, 0)), (26, (7, 0, 0)), (35, (12, 0, 0)), (39, (14, 0, 0))),
    "aarch64": ((17, (4, 7, 0)), (26, (7, 0, 0)), (34, (11, 0)), (39, (14, 0, 0))),
    "ppc64le": ((17, (4, 7, 0)), (26, (7, 0, 0)), (39, (14, 0, 0))),
    "s390x": ((17, (4, 7, 0)), (26, (7, 0, 0)), (39, (14, 0, 0))),
    "armv7l": ((17, (4, 7, 0)), (26, (7, 0, 0)), (39, (14, 0, 0))),
    "riscv64": ((31, (7, 0, 0)), (39, (14, 0, 0))),
    "loongarch64": ((36, (7, 0, 0)), (39, (14, 0, 0))),
}

#: The symbols manylinux2014 forbids, by library.
FORBIDDEN_2_17 = {
    "libc.so.6": frozenset({
        "__cxa_thread_atexit_impl", "__issignaling", "__issignalingf",
        "__issignalingl", "pthread_getattr_default_np", "pthread_setattr_default_np"}),
    "libm.so.6": frozenset({"__issignaling", "__issignalingf", "__issignalingl"}),
    "libpthread.so.0": frozenset({"pthread_getattr_default_np", "pthread_setattr_default_np"}),
}

#: The first glibc with ``GLIBC_ABI_DT_RELR`` (packed relative relocations).
DT_RELR_GLIBC = 36


def _numbers(text):
    """``(4, 8, 0)`` of ``4.8.0``."""
    return tuple(int(part) for part in text.split("."))


def _glibc_minor(version):
    """The glibc minor version a ``GLIBC_`` symbol version needs, or None
    for one no glibc release exports (``GLIBC_PRIVATE``)."""
    if version == "GLIBC_ABI_DT_RELR":
        return DT_RELR_GLIBC
    found = re.fullmatch(r"GLIBC_2\.(\d+)(?:\.\d+)*", version)
    return int(found.group(1)) if found else None


def _newest_gcc(arch, minor):
    """The newest ``GCC_`` version the policy of glibc 2.*minor* allows on
    *arch*, or None if it has no policy for *arch*."""
    newest = None
    for first, version in NEWEST_GCC.get(arch, ()):
        if first <= minor:
            newest = version
    return newest


def _version_problem(version, minor, newest_gcc):
    """Why the policy of glibc 2.*minor* rules out the symbol version
    *version* (an :class:`_elf.Version`), or None."""
    name = version.name
    if name.startswith("GLIBC_"):
        needs = _glibc_minor(name)
        if needs is None or needs > minor:
            return f"{name} of {version.library}"
        return None
    if name.startswith("GCC_") and re.fullmatch(r"\d+(\.\d+)*", name[4:]):
        return None if _numbers(name[4:]) <= newest_gcc else f"{name} of {version.library}"
    return f"{name} of {version.library} (a version this check does not know)"


def policy_problems(elf, minor):
    """What the library *elf* (an :class:`_elf.Elf`) needs beyond the
    manylinux policy of glibc 2.*minor*, as sentences; empty if it holds."""
    newest_gcc = _newest_gcc(elf.arch, minor)
    if newest_gcc is None:
        return [f"no manylinux_2_{minor} policy for {elf.arch}"]
    allowed = LIBRARIES | (LIBRARIES_FROM_2_28 if minor >= 28 else frozenset())
    problems = [f"the library {library}" for library in elf.needed if library not in allowed]
    problems += [problem for version in elf.versions if not version.weak
                 if (problem := _version_problem(version, minor, newest_gcc))]
    if minor <= 17:
        problems += [
            f"{symbol.name} of {symbol.version.library}" for symbol in elf.undefined
            if not symbol.weak and symbol.version is not None
            and symbol.name in FORBIDDEN_2_17.get(symbol.version.library, ())]
    return problems


def unbound_symbols(elf):
    """The symbols the glibc library *elf* leaves undefined, unversioned
    and not weak, but Python's: those no library it was linked against
    defines."""
    return sorted(
        symbol.name for symbol in elf.undefined
        if symbol.version is None and not symbol.weak
        and not symbol.name.startswith(("Py", "_Py")))


def _is_musl_library(name):
    """Whether *name* is musl's C library (``libc.so`` as linkers name it,
    ``libc.musl-<arch>.so.1`` as Alpine does, or its loader)."""
    return name == "libc.so" or name.startswith(("libc.musl-", "ld-musl-"))


def linux_tags(arch, library, musl=False):
    """
    The platform tags of the Linux shared library whose bytes are
    *library*, built for the Rust architecture *arch* against glibc, or
    against musl when *musl* (see the module's description).
    """
    name = LINUX_ARCHITECTURES.get(arch, arch)
    elf = _elf.parse(library)
    if elf.arch != name:
        raise ValueError(f"pltrs: the extension is built for {elf.arch}, not {name}")
    if musl:
        if all(_is_musl_library(needed) for needed in elf.needed):
            return [f"musllinux_1_2_{name}"]
        print(f"pltrs: the extension needs {', '.join(elf.needed)}: tagged linux_{name}",
              file=sys.stderr)
        return [f"linux_{name}"]
    unbound = unbound_symbols(elf)
    if unbound:
        raise RuntimeError(
            "pltrs: the extension uses symbols the C library it was linked against "
            f"lacks, so it would not load: {', '.join(unbound)}")
    problems = []
    for minor in POLICIES:
        problems = policy_problems(elf, minor)
        if not problems:
            tags = [f"manylinux_2_{minor}_{name}"]
            if minor == 17:
                tags.append(f"manylinux2014_{name}")
            return tags
    print(f"pltrs: no manylinux policy allows what the extension needs "
          f"({'; '.join(problems)}): tagged linux_{name}", file=sys.stderr)
    return [f"linux_{name}"]
