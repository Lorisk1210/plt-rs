"""
The linker rustc runs for a Linux wheel linked by Zig (see ``_linker``):
``zig cc`` for one Zig target, such as ``x86_64-linux-gnu.2.17``,
given the arguments rustc gives a C compiler driver, less those Zig's
driver does not take. Run as

    python -I _zigcc.py ZIG ZIG_TARGET ARGUMENT...

It is a script with no imports from the backend, as rustc starts it
afresh for every link.

* ``-fuse-ld=lld`` and ``-B<dir>`` point a GCC driver at the LLD that
  rustc ships (rustc's default on x86_64 Linux): Zig links with its own.
* ``-lgcc_s`` asks for GCC's unwinder, which Zig's targets have in its
  own ``libunwind``, linked into the library.
* ``--target=...`` and ``-m64``/``-m32`` would contradict ``-target``.
* ``-Wl,-O1`` is an optimisation Zig's linker ignores, saying so.

Zig's driver refuses ``-Wl,--fix-cortex-a53-843419``, the workaround
for Cortex-A53 erratum 843419 (the Raspberry Pi 3's core has it) rustc
asks for on aarch64, and the order of the extension's functions
(``-Wl,--symbol-ordering-file=...``, see ``crates/pltrs-python/build.rs``),
but the LLD Zig links with applies them. So such a library is linked by
Zig, which reports the LLD command it ran (``-v``), then relinked by that
command with the flags (``zig ld.lld``); the build fails if the command
cannot be found.

Response files (``@file``, which rustc writes when a command line is too
long) are read, filtered and passed on as arguments.
"""

import subprocess
import sys

#: Arguments dropped whole.
DROPPED = {"-fuse-ld=lld", "-m64", "-m32", "-Wl,-O1"}

#: Prefixes of arguments dropped whole.
DROPPED_PREFIXES = ("-B", "--target=")

#: Arguments replaced by others.
REPLACED = {"-lgcc_s": "-lunwind"}

#: Arguments Zig's driver refuses, and the LLD flags they stand for.
RELINKED = {"-Wl,--fix-cortex-a53-843419": "--fix-cortex-a53-843419",
            "-Wl,--no-warn-symbol-ordering": "--no-warn-symbol-ordering"}

#: Prefixes of arguments Zig's driver refuses, which stand for the LLD
#: flag after ``-Wl,``.
RELINKED_PREFIXES = ("-Wl,--symbol-ordering-file=",)


def relinked(argument):
    """The LLD flag *argument* stands for, if Zig's driver refuses it."""
    if argument.startswith(RELINKED_PREFIXES):
        return argument.removeprefix("-Wl,")
    return RELINKED.get(argument)


def response_file(text):
    """The arguments of a response file rustc wrote: one per line, with
    ``\\`` escaping the characters GCC's reader would split on."""
    arguments = []
    for line in text.splitlines():
        out, escaped = [], False
        for char in line:
            if escaped or char != "\\":
                out.append(char)
                escaped = False
            else:
                escaped = True
        arguments.append("".join(out))
    return arguments


def expanded(arguments):
    """*arguments* with every response file replaced by its contents."""
    for argument in arguments:
        if argument.startswith("@"):
            with open(argument[1:], encoding="utf-8") as file:
                yield from response_file(file.read())
        else:
            yield argument


def filtered(arguments):
    """The arguments Zig's driver gets for rustc's *arguments*."""
    for argument in arguments:
        if argument in DROPPED or relinked(argument) or argument.startswith(DROPPED_PREFIXES):
            continue
        yield REPLACED.get(argument, argument)


def lld_command(report, output):
    """The arguments of the LLD command Zig's ``-v`` *report* shows
    writing *output*, or None."""
    for line in reversed(report.splitlines()):
        words = line.split(" ")
        if words[0] == "ld.lld" and f" -o {output} " in f"{line} ":
            return words[1:]
    return None


def main(argv):
    """Links, and exits with the linker's status."""
    zig, target, *rest = argv
    arguments = list(expanded(rest))
    command = [zig, "cc", "-target", target, *filtered(arguments)]
    flags = [flag for flag in map(relinked, arguments) if flag]
    if not flags:
        sys.exit(subprocess.run(command).returncode)
    linked = subprocess.run([*command, "-v"], stderr=subprocess.PIPE, text=True)
    if linked.returncode != 0:
        sys.stderr.write(linked.stderr)
        sys.exit(linked.returncode)
    output = arguments[arguments.index("-o") + 1]
    lld = lld_command(linked.stderr, output)
    if lld is None:
        sys.exit(f"pltrs: Zig reported no LLD command writing {output}, so the library "
                 f"cannot be relinked with {' '.join(flags)}")
    sys.exit(subprocess.run([zig, "ld.lld", *lld, *flags]).returncode)


if __name__ == "__main__":
    main(sys.argv[1:])
