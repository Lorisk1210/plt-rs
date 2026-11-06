#!/bin/sh
# Zig, the C toolchain pltrs' build backend links Linux wheels with, so
# that a wheel built on a new distribution runs on old ones: `zig cc
# -target x86_64-linux-gnu.2.17` links against glibc 2.17's symbols
# (manylinux2014), and `-target aarch64-linux-gnu.2.17` cross-links
# for aarch64, without root, containers or a sysroot. It is a build tool
# only: nothing of Zig ends up in the wheel.
#
#   python/tools/zig.sh install    download, check and unpack Zig into $ZIG_PREFIX
#   python/tools/zig.sh path       print the zig program's path
#
# Then:
#
#   ZIG=$(python/tools/zig.sh path) pip wheel . -C glibc=2.17
#   ZIG=$(python/tools/zig.sh path) pip wheel . -C glibc=2.17 -C target=aarch64-unknown-linux-gnu
#
# (`rustup target add aarch64-unknown-linux-gnu` first for the second.)
# $ZIG_PREFIX defaults to ~/.local/zig. The release is pinned with the
# SHA-256 ziglang.org publishes for it (https://ziglang.org/download/index.json);
# rerunning `install` is harmless.
set -eu

version=0.17.1
prefix=${ZIG_PREFIX:-$HOME/.local/zig}

host() {
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64) echo x86_64-linux ;;
        Linux-aarch64 | Linux-arm64) echo aarch64-linux ;;
        Darwin-arm64) echo aarch64-macos ;;
        Darwin-x86_64) echo x86_64-macos ;;
        *) echo "zig.sh: no Zig release pinned for $(uname -s) $(uname -m)" >&2; exit 1 ;;
    esac
}

sha256() {
    case "$1" in
        x86_64-linux) echo 3581863e3cbcfd0fce0efc682ce7e9e93da96668a977791d356a26d2a7e9b17a ;;
        aarch64-linux) echo a1546d5646eb184437918d6635ca6b762a0e889eac09cef720ee7a1ed3fe7795 ;;
        aarch64-macos) echo 75f1bc7a1e9852f17ec9a0c1895f767c4ec56d46bc8a834920eece1d0359dd8d ;;
        x86_64-macos) echo 9af07b20319fbd0c92ee46ad23e81d6a782b1aaea2cfb8dcbff35f1d2c7ddbb7 ;;
    esac
}

directory() {
    printf '%s' "$prefix/zig-$(host)-$version"
}

digest() {
    if command -v sha256sum >/dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

install() {
    platform=$(host)
    if [ -x "$(directory)/zig" ]; then
        echo "zig $version is installed in $(directory)"
        return
    fi
    name=zig-$platform-$version
    mkdir -p "$prefix"
    # The archive is unpacked in a directory of its own, then moved in place.
    download=$(mktemp -d "$prefix/.download.XXXXXX")
    trap 'rm -rf "$download"' EXIT
    curl -fsSL "https://ziglang.org/download/$version/$name.tar.xz" -o "$download/$name.tar.xz"
    if [ "$(digest "$download/$name.tar.xz")" != "$(sha256 "$platform")" ]; then
        echo "zig.sh: $name.tar.xz does not have the SHA-256 pinned for it" >&2
        exit 1
    fi
    mkdir "$download/unpacked"
    tar -xJf "$download/$name.tar.xz" -C "$download/unpacked"
    mv "$download/unpacked/$name" "$(directory)"
    echo "zig $version installed in $(directory)"
}

case "${1:-}" in
    install) install ;;
    path) printf '%s\n' "$(directory)/zig" ;;
    *) echo "usage: $0 install | path" >&2; exit 2 ;;
esac
