"""
The plan of a run of the wheels workflow: checks that a version tag names
Cargo.toml's version, and writes the jobs' matrices to ``GITHUB_OUTPUT``:

* ``linux``: manylinux2014 and musllinux_1_2, for x86_64 and aarch64,
  each with the Rust target it is built for, the build backend's
  settings for it (``--config-settings``: Zig links against glibc 2.17,
  or musl) and the runner it is tested on (it is built on an x86_64
  one, Zig cross-linking aarch64);
* ``windows``: the architectures built, x86_64, and aarch64 when the run
  asks for it (the input ``win_arm64``, or the repository variable
  ``WIN_ARM64`` set to ``true``);
* ``windows_tests``: each built architecture with the Pythons it is
  tested with.

The arm64 runners are GitHub's own (``ubuntu-24.04-arm``,
``windows-11-arm``) unless the repository variables
``LINUX_ARM64_RUNNER`` and ``WINDOWS_ARM64_RUNNER`` name others. It reads
the event from ``EVENT``, ``TAG``, ``WIN_ARM64`` and those variables.
"""

import json
import os
import sys
import tomllib


def main():
    """Checks the tag and writes the matrices."""
    with open("Cargo.toml", "rb") as file:
        version = tomllib.load(file)["workspace"]["package"]["version"]
    print(f"pltrs {version}")
    tag = os.environ.get("TAG", "")
    if os.environ.get("EVENT") == "push" and tag != f"v{version}":
        sys.exit(f"the tag {tag} is not v{version}, the version of Cargo.toml")
    runners = {
        "x86_64": "ubuntu-24.04",
        "aarch64": os.environ.get("LINUX_ARM64_RUNNER") or "ubuntu-24.04-arm",
    }
    libraries = {"manylinux2014": ("gnu", ["glibc=2.17"]), "musllinux_1_2": ("musl", [])}
    linux = []
    for platform, (library, settings) in libraries.items():
        for arch, runner in runners.items():
            target = f"{arch}-unknown-linux-{library}"
            linux.append({"platform": platform, "arch": arch, "runner": runner, "target": target,
                          "settings": " ".join(f"--config-settings {setting}"
                                               for setting in [f"target={target}", *settings])})
    windows = ["x86_64"]
    if os.environ.get("WIN_ARM64") == "true":
        windows.append("aarch64")
    windows_runners = {
        "x86_64": "windows-2025",
        "aarch64": os.environ.get("WINDOWS_ARM64_RUNNER") or "windows-11-arm",
    }
    pythons = {"x86_64": ["3.12", "3.14"], "aarch64": ["3.14"]}
    windows_tests = [{"arch": arch, "runner": windows_runners[arch], "python": python}
                     for arch in windows for python in pythons[arch]]
    outputs = {"linux": linux, "windows": windows, "windows_tests": windows_tests}
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as file:
        for name, value in outputs.items():
            print(f"{name}={json.dumps(value)}", file=file)
            print(f"{name}: {json.dumps(value)}")


if __name__ == "__main__":
    main()
