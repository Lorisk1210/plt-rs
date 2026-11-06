"""
The platform tags of pltrs' wheel, worked out from the extension the
wheel holds, never from the Python running the build, so that a tag
never claims more than the library delivers:

* Windows: ``win_amd64``, ``win_arm64`` or ``win32`` (``_pe``);
* macOS: ``macosx_<version>_<arch>`` (``_macho``);
* Linux: ``manylinux_2_<n>_<arch>`` or ``musllinux_1_2_<arch>``, else
  ``linux_<arch>`` (``_manylinux``).

Each check also makes sure the library is built for the architecture
the target names.
"""

from ._macho import macos_tag
from ._manylinux import linux_tags
from ._pe import windows_tag


def target_parts(target):
    """The system (``linux``, ``macos``, ``windows``), architecture and
    whether the C library is musl, of the Rust target triple *target*."""
    arch, _, rest = target.partition("-")
    if "windows" in rest:
        return "windows", arch, False
    if "apple-darwin" in rest:
        return "macos", arch, False
    if "linux" in rest:
        return "linux", arch, "musl" in rest
    raise ValueError(f"pltrs: no wheel platform for the Rust target {target!r}")


def platform_tags(target, library):
    """The platform tags of the extension built for the Rust target
    *target* (or ``universal2``) whose bytes are *library*."""
    if target == "universal2":
        return [macos_tag("universal2", library)]
    system, arch, musl = target_parts(target)
    if system == "windows":
        return [windows_tag(arch, library)]
    if system == "macos":
        return [macos_tag(arch, library)]
    return linux_tags(arch, library, musl)
