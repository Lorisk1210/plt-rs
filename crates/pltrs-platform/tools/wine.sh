#!/bin/sh
# pltrs for Windows, built and run on Linux: Wine 10 and the MinGW-w64
# runtime, installed without root from the distribution's packages, and
# cargo for x86_64-pc-windows-gnu linked by rust-lld.
#
#   tools/wine.sh install               download and unpack them into $WINE_HOME, make the prefix
#   tools/wine.sh cargo ARGS...         cargo for x86_64-pc-windows-gnu; tests and runs go through Wine
#   tools/wine.sh run PROGRAM [ARGS...] run a Windows program (cargo's runner)
#   tools/wine.sh python                install CPython for Windows (C:\python) and pltrs's requirements
#   tools/wine.sh x11 PROGRAM [ARGS...] run xdotool or xwd, to drive and capture Wine's windows
#
# For example, with an X server for the windows (Xvfb :121 -screen 0 1280x800x24 &):
#
#   DISPLAY=:121 tools/wine.sh cargo test -p pltrs-platform
#   DISPLAY=:121 tools/wine.sh cargo run -p pltrs-platform --example window
#   DISPLAY=:121 tools/wine.sh x11 xdotool search --name 'pltrs-platform demo' key t
#   DISPLAY=:121 tools/wine.sh x11 xwd -root -out screen.xwd
#   tools/wine.sh cargo build -p pltrs-python   # then copy pltrs_python.dll to python/pltrs/_native.pyd
#   PYTHONPATH='Z:\path\to\python' tools/wine.sh run 'C:\python\python.exe' -B -m pytest python/tests
#
# (-B: bytecode written next to the sources would name them by Windows paths,
# which pytest's rewritten tests keep when Linux's Python reads them back.)
#
# $WINE_HOME defaults to ~/.local/wine, the prefix (drive C:) to $WINE_HOME/prefix.
# The package list is for Debian 13 (trixie). Nothing here changes the Linux
# build: the target's linker, flags and runner are set for this cargo only.
set -eu

home=${WINE_HOME:-$HOME/.local/wine}
root=$home/root
mingw=$home/mingw
target=x86_64-pc-windows-gnu
tools=$(dirname "$(realpath "$0")")
# The Windows CPython: the version of the workspace's venvs.
python_version=3.14.8

# Wine (64-bit only), its fonts (the title bars' symbols) and the zlib its
# user32 loads, and the X tools.
packages="wine wine64 libwine fonts-wine libz-mingw-w64 xdotool libxdo3 x11-apps"
# The MinGW-w64 import libraries and C runtime for the UCRT, the C runtime
# MSVC builds use (the msvcrt flavour links MinGW-w64's own sin, cos and
# pow, x87 code wrong in the last digits, or entirely for large angles);
# from the cross compiler only libgcc, and from binutils dlltool and the
# assembler it runs, which make the import libraries of `raw-dylib` links.
mingw_packages="mingw-w64-ucrt64-dev gcc-mingw-w64-ucrt64 binutils-mingw-w64-x86-64"

# On GNU targets LLVM fuses the sine and cosine of one angle into a call
# of `sincos`, which MinGW-w64 also computes with the x87 (cos(pi/2) is
# 6.12303e-17, not 6.12323e-17); MSVC builds call the UCRT's `sin` and
# `cos`. This library, linked first, does too.
winemath_source='#![no_std]
//! `sincos` and `sincosf` as MSVC builds compute them.

unsafe extern "C" {
    safe fn sin(x: f64) -> f64;
    safe fn cos(x: f64) -> f64;
    safe fn sinf(x: f32) -> f32;
    safe fn cosf(x: f32) -> f32;
}

/// Stores the sine and the cosine of `x`.
///
/// # Safety
///
/// `s` and `c` point to writable values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sincos(x: f64, s: *mut f64, c: *mut f64) {
    // SAFETY: the caller passes writable pointers.
    unsafe {
        s.write(sin(x));
        c.write(cos(x));
    }
}

/// Stores the sine and the cosine of `x`.
///
/// # Safety
///
/// `s` and `c` point to writable values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sincosf(x: f32, s: *mut f32, c: *mut f32) {
    // SAFETY: the caller passes writable pointers.
    unsafe {
        s.write(sinf(x));
        c.write(cosf(x));
    }
}
'

# The UCRT functions numpy calls when imported that Wine 10.0 lacks; the
# shim DLL holding them forwards the rest of the math API set to ucrtbase
# (see wine_numpy.py).
crtshim_source='//! The complex number accessors of the UCRT that Wine 10.0 lacks.

/// `_Fcomplex`.
#[repr(C)]
pub struct Fcomplex {
    re: f32,
    im: f32,
}

/// `_Dcomplex`, and `_Lcomplex` (a long double is a double).
#[repr(C)]
pub struct Dcomplex {
    re: f64,
    im: f64,
}

#[unsafe(no_mangle)]
pub extern "C" fn crealf(z: Fcomplex) -> f32 {
    z.re
}

#[unsafe(no_mangle)]
pub extern "C" fn cimagf(z: Fcomplex) -> f32 {
    z.im
}

#[unsafe(no_mangle)]
pub extern "C" fn creal(z: Dcomplex) -> f64 {
    z.re
}

#[unsafe(no_mangle)]
pub extern "C" fn cimag(z: Dcomplex) -> f64 {
    z.im
}

#[unsafe(no_mangle)]
pub extern "C" fn creall(z: Dcomplex) -> f64 {
    z.re
}

#[unsafe(no_mangle)]
pub extern "C" fn cimagl(z: Dcomplex) -> f64 {
    z.im
}
'

# The flags linking for Windows here, as rustc and rustdoc take them.
link_flags() {
    libgcc=$(dirname "$(ls "$mingw"/usr/lib/gcc/x86_64-w64-mingw32ucrt/*/libgcc.a)")
    printf '%s' "-C linker-flavor=ld.lld -C link-self-contained=yes \
-C dlltool=$mingw/usr/bin/x86_64-w64-mingw32-dlltool -L native=$mingw/winemath \
-L native=$mingw/usr/x86_64-w64-mingw32ucrt/lib -L native=$libgcc"
}

# rustc for Windows, with the workspace's toolchain.
rustc_windows() {
    (cd "$tools" && rustc --edition 2024 --target $target -C opt-level=2 -C panic=abort \
        -C linker=rust-lld $(link_flags) "$@")
}

# Builds $mingw/winemath/libwinemath.a.
build_winemath() {
    (cd "$tools" && rustup target add $target)
    host=$(cd "$tools" && rustc -vV | sed -n 's/^host: //p')
    lld=$(cd "$tools" && rustc --print sysroot)/lib/rustlib/$host/bin/rust-lld
    mkdir -p "$mingw/winemath"
    printf '%s' "$winemath_source" >"$mingw/winemath/winemath.rs"
    rustc_windows --crate-type=lib --emit=obj "$mingw/winemath/winemath.rs" \
        -o "$mingw/winemath/winemath.o"
    "$lld" -flavor link /lib /out:"$mingw/winemath/libwinemath.a" "$mingw/winemath/winemath.o"
}

install() {
    mkdir -p "$home/debs" "$home/mingw-debs" "$root" "$mingw"
    # `apt-get download` writes into the current directory.
    (cd "$home/debs" && apt-get download $packages)
    (cd "$home/mingw-debs" && apt-get download $mingw_packages)
    for deb in "$home"/debs/*.deb; do
        dpkg-deb -x "$deb" "$root"
    done
    dpkg-deb --fsys-tarfile "$home"/mingw-debs/mingw-w64-ucrt64-dev_*.deb |
        tar -x -C "$mingw" ./usr/x86_64-w64-mingw32ucrt/lib
    dpkg-deb --fsys-tarfile "$home"/mingw-debs/gcc-mingw-w64-ucrt64_*.deb |
        tar -x -C "$mingw" --wildcards './usr/lib/gcc/x86_64-w64-mingw32ucrt/*/libgcc*.a'
    dpkg-deb --fsys-tarfile "$home"/mingw-debs/binutils-mingw-w64-x86-64_*.deb |
        tar -x -C "$mingw" ./usr/bin/x86_64-w64-mingw32-dlltool ./usr/bin/x86_64-w64-mingw32-as
    build_winemath
    # Debian's `wineserver` is a script that runs /usr/lib/wine/wineserver64;
    # Wine starts the one next to its loader.
    ln -sf wineserver64 "$root/usr/lib/wine/wineserver"
    # Unpacked elsewhere than /, Wine looks for its data (nls, wine.inf,
    # fonts) in usr/share/wine/wine, relative to itself.
    ln -sfn . "$root/usr/share/wine/wine"
    wine_env
    # Debian's user32 loads zlib1.dll from /usr/x86_64-w64-mingw32/lib;
    # from here only system32 has it.
    mkdir -p "$WINEPREFIX/drive_c/windows/system32"
    cp "$root/usr/x86_64-w64-mingw32/lib/zlib1.dll" "$WINEPREFIX/drive_c/windows/system32/"
    # wineboot fails on the 32-bit half of the prefix, which isn't installed.
    "$WINELOADER" wineboot --init ||
        test -f "$WINEPREFIX/drive_c/windows/system32/kernel32.dll"
    "$WINESERVER" --wait
    echo "Wine installed in $home"
}

wine_env() {
    export WINEPREFIX="${WINEPREFIX:-$home/prefix}"
    export WINELOADER="$root/usr/lib/wine/wine64"
    export WINESERVER="$root/usr/lib/wine/wineserver64"
    export WINEDEBUG="${WINEDEBUG:--all}"
}

run() {
    wine_env
    exec "$WINELOADER" "$@"
}

# CPython for Windows (python.org's embeddable package) in C:\python, with
# pip, pltrs's requirements and the numpy shim.
python_install() {
    wine_env
    downloads=$home/downloads
    python=$WINEPREFIX/drive_c/python
    archive=python-$python_version-embed-amd64.zip
    mkdir -p "$downloads"
    (cd "$downloads" &&
        curl -sSfLO "https://www.python.org/ftp/python/$python_version/$archive" &&
        curl -sSfLO https://bootstrap.pypa.io/get-pip.py)
    rm -rf "$python"
    python3 -I -m zipfile -e "$downloads/$archive" "$python"
    # Without its ._pth file the interpreter reads PYTHONPATH and has a
    # site-packages directory, as an installed one.
    rm "$python"/python*._pth
    # Wine resolves the Linux paths given to Windows programs on drive Z:
    # when started from a directory outside drive C:.
    cd "$home"
    "$WINELOADER" "$python/python.exe" "$downloads/get-pip.py" --no-warn-script-location
    "$WINELOADER" "$python/python.exe" -m pip install --no-warn-script-location \
        -r "$tools/../../../python/requirements-pltrs.txt"
    site=$python/Lib/site-packages
    mkdir -p "$mingw/crtshim"
    printf '%s' "$crtshim_source" >"$mingw/crtshim/crtshim.rs"
    shim=$(python3 -I "$tools/wine_numpy.py" def "$site" "$mingw/crtshim/crtshim.def")
    rustc_windows --crate-type=cdylib -C link-arg="$mingw/crtshim/crtshim.def" \
        "$mingw/crtshim/crtshim.rs" -o "$mingw/crtshim/$shim"
    cp "$mingw/crtshim/$shim" "$python/"
    python3 -I "$tools/wine_numpy.py" patch "$site"
    "$WINESERVER" --wait
    echo "Python $python_version installed in $python"
}

cargo_windows() {
    link=$(link_flags)
    export CARGO_BUILD_TARGET=$target
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=rust-lld
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="$link -l static:-bundle=winemath"
    # Doctests are linked by rustdoc, which takes no per-target flags (and
    # finds winemath in the libraries' metadata).
    export RUSTDOCFLAGS="${RUSTDOCFLAGS:-} $link"
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER="$(realpath "$0") run"
    exec cargo "$@"
}

x11() {
    program=$1
    shift
    LD_LIBRARY_PATH=$root/usr/lib/x86_64-linux-gnu exec "$root/usr/bin/$program" "$@"
}

case ${1:-} in
install) install ;;
cargo)
    shift
    cargo_windows "$@"
    ;;
run)
    shift
    run "$@"
    ;;
python) python_install ;;
x11)
    shift
    x11 "$@"
    ;;
*)
    sed -n '2,26p' "$0" >&2
    exit 2
    ;;
esac
