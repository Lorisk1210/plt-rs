"""
The platform tags of pltrs' wheel, worked out from what the extension is
built for, never from the Python running the build.

* Windows: ``win_amd64``, ``win_arm64`` or ``win32``, by architecture.
* macOS: ``macosx_<deployment target>_<arch>``, ``arm64`` or ``x86_64``,
  or ``universal2`` for a library holding both. The deployment target is
  ``MACOSX_DEPLOYMENT_TARGET`` (which rustc honours too), else rustc's own
  for the architecture (10.12 for x86_64, 11.0 for arm64), and never
  below the architecture's first macOS; from macOS 11 on, tags name the
  major version only (``macosx_11_0``), as pip matches them.
* Linux: ``manylinux_2_<N>_<arch>`` when the library needs nothing but
  the libraries every manylinux system has (libc, libm, libgcc_s, ...),
  ``N`` being the newest glibc version its symbols ask for, read from its
  ELF version needs (with the legacy alias ``manylinux2014`` when ``N`` is
  17); else ``linux_<arch>``, which pip installs on the machine it was
  built for.
"""

import struct

#: The macOS each architecture first ran, and rustc's default
#: deployment target for it.
MACOS_MINIMUM = {"x86_64": (10, 12), "arm64": (11, 0)}

#: The libraries a manylinux wheel may need from the system (PEP 600's
#: policies, as auditwheel lists them).
MANYLINUX_LIBRARIES = frozenset({
    "libgcc_s.so.1", "libstdc++.so.6", "libm.so.6", "libdl.so.2",
    "librt.so.1", "libc.so.6", "libnsl.so.1", "libutil.so.1",
    "libpthread.so.0", "libresolv.so.2", "libX11.so.6", "libXext.so.6",
    "libXrender.so.1", "libICE.so.6", "libSM.so.6", "libGL.so.1",
    "libgobject-2.0.so.0", "libgthread-2.0.so.0", "libglib-2.0.so.0",
    "ld-linux-x86-64.so.2", "ld-linux-aarch64.so.1", "ld-linux.so.2",
})

#: The manylinux architecture names of Rust's.
LINUX_ARCHITECTURES = {
    "x86_64": "x86_64", "aarch64": "aarch64", "i686": "i686",
    "powerpc64le": "ppc64le", "s390x": "s390x", "armv7": "armv7l",
}


def target_os_and_arch(target):
    """The system (``linux``, ``macos``, ``windows``) and architecture of
    the Rust target triple *target*."""
    arch, _, rest = target.partition("-")
    if "windows" in rest:
        return "windows", arch
    if "apple-darwin" in rest:
        return "macos", arch
    if "linux" in rest:
        return "linux", arch
    raise ValueError(f"pltrs: no wheel platform for the Rust target {target!r}")


def windows_tag(arch):
    """The platform tag of a Windows library of *arch*."""
    tags = {"x86_64": "win_amd64", "aarch64": "win_arm64", "i686": "win32"}
    if arch not in tags:
        raise ValueError(f"pltrs: no wheel platform for Windows on {arch}")
    return tags[arch]


def _parse_macos_version(text):
    """``(major, minor)`` of a ``MACOSX_DEPLOYMENT_TARGET`` value."""
    parts = text.strip().split(".")
    try:
        major = int(parts[0])
        minor = int(parts[1]) if len(parts) > 1 else 0
    except ValueError:
        raise ValueError(
            f"pltrs: MACOSX_DEPLOYMENT_TARGET={text!r} is not a version") from None
    return major, minor


def macos_tag(arch, deployment_target=None):
    """
    The platform tag of a macOS library of *arch* (``x86_64``,
    ``aarch64``, or ``universal2`` for both), for the deployment target
    *deployment_target* (``MACOSX_DEPLOYMENT_TARGET``, or None for
    rustc's default).
    """
    name = {"aarch64": "arm64"}.get(arch, arch)
    # A universal2 library starts where its x86_64 half does.
    lowest = MACOS_MINIMUM.get("x86_64" if name == "universal2" else name)
    if lowest is None:
        raise ValueError(f"pltrs: no wheel platform for macOS on {arch}")
    version = lowest
    if deployment_target:
        version = max(_parse_macos_version(deployment_target), lowest)
    major, minor = version
    if major >= 11:
        minor = 0
    return f"macosx_{major}_{minor}_{name}"


def _elf_needs(data):
    """
    The libraries an ELF shared library needs (``DT_NEEDED``) and the
    versions of their symbols it cannot do without (``.gnu.version_r``,
    but the weak ones), as ``(libraries, {library: [version, ...]})``.
    """
    if data[:4] != b"\x7fELF":
        raise ValueError("pltrs: the extension is not an ELF file")
    is64 = data[4] == 2
    end = "<" if data[5] == 1 else ">"
    if is64:
        shoff, = struct.unpack_from(end + "Q", data, 0x28)
        shentsize, shnum, shstrndx = struct.unpack_from(end + "HHH", data, 0x3A)
    else:
        shoff, = struct.unpack_from(end + "I", data, 0x20)
        shentsize, shnum, shstrndx = struct.unpack_from(end + "HHH", data, 0x2E)
    sections = []
    for index in range(shnum):
        base = shoff + index * shentsize
        if is64:
            _, kind, _, _, offset, size, link, info, _, entsize = struct.unpack_from(
                end + "IIQQQQIIQQ", data, base)
        else:
            _, kind, _, _, offset, size, link, info, _, entsize = struct.unpack_from(
                end + "IIIIIIIIII", data, base)
        sections.append((kind, offset, size, link, info, entsize))

    def string(table, offset):
        start = sections[table][1] + offset
        return data[start:data.index(b"\0", start)].decode()

    needed, versions = [], {}
    for kind, offset, size, link, info, entsize in sections:
        if kind == 6:  # SHT_DYNAMIC
            step = 16 if is64 else 8
            for pos in range(offset, offset + size, step):
                tag, value = struct.unpack_from(end + ("qQ" if is64 else "iI"), data, pos)
                if tag == 1:  # DT_NEEDED
                    needed.append(string(link, value))
                elif tag == 0:
                    break
        elif kind == 0x6FFFFFFE:  # SHT_GNU_verneed
            pos = offset
            for _ in range(info):
                _, count, file, aux, following = struct.unpack_from(end + "HHIII", data, pos)
                library = string(link, file)
                apos = pos + aux
                for _ in range(count):
                    _, flags, _, name, anext = struct.unpack_from(end + "IHHII", data, apos)
                    # A version only weak references need (VER_FLG_WEAK) may
                    # be missing: the library runs without the symbols.
                    if not flags & 2:
                        versions.setdefault(library, []).append(string(link, name))
                    apos += anext
                pos += following
    return needed, versions


def linux_tags(arch, library):
    """
    The platform tags of the Linux shared library whose bytes are
    *library*, built for *arch* (see the module's description).
    """
    name = LINUX_ARCHITECTURES.get(arch, arch)
    needed, versions = _elf_needs(library)
    if not set(needed) <= MANYLINUX_LIBRARIES:
        return [f"linux_{name}"]
    glibc = [(2, 17)]
    for version in (v for vs in versions.values() for v in vs):
        if version.startswith("GLIBC_2."):
            minor = version[len("GLIBC_2."):].split(".")[0]
            if minor.isdigit():
                glibc.append((2, int(minor)))
    major, minor = max(glibc)
    tags = [f"manylinux_{major}_{minor}_{name}"]
    if (major, minor) == (2, 17):
        tags.append(f"manylinux2014_{name}")
    return tags


def platform_tags(target, library, deployment_target=None):
    """
    The platform tags of the extension built for the Rust target *target*
    (or ``universal2``) whose bytes are *library*, with macOS'
    *deployment_target*.
    """
    if target == "universal2":
        return [macos_tag("universal2", deployment_target)]
    system, arch = target_os_and_arch(target)
    if system == "windows":
        return [windows_tag(arch)]
    if system == "macos":
        return [macos_tag(arch, deployment_target)]
    return linux_tags(arch, library)
