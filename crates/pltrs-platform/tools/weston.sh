#!/bin/sh
# A headless Wayland compositor (weston 14) for pltrs-platform's Wayland live
# tests, installed without root from the distribution's packages.
#
#   tools/weston.sh install          download and unpack weston into $WESTON_PREFIX
#   tools/weston.sh run [ARGS...]    start it headless on $XDG_RUNTIME_DIR/wayland-pltrs
#   tools/weston.sh env              print the variables a client needs
#   tools/weston.sh screenshot       save ./wayland-screenshot-*.png (needs `run --debug`)
#
# Then, with the compositor running:
#
#   WAYLAND_DISPLAY=wayland-pltrs cargo test -p pltrs-platform wayland
#
# $WESTON_PREFIX defaults to ~/.local/weston, $WESTON_SOCKET to wayland-pltrs.
# The package list is for Debian 13 (trixie): weston and libweston plus
# the libraries a desktop without a compositor lacks.
set -eu

prefix=${WESTON_PREFIX:-$HOME/.local/weston}
socket=${WESTON_SOCKET:-wayland-pltrs}
lib=$prefix/root/usr/lib/x86_64-linux-gnu
runtime=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}

packages="weston libweston-14-0 libcairo2 libevdev2 libinput10 libpango-1.0-0
libpangocairo-1.0-0 libpangoft2-1.0-0 libfribidi0 libharfbuzz0b libmtdev1t64
libthai0 libdatrie1 libwacom9 libgudev-1.0-0 libgraphite2-3 libxcb-render0
libwayland-cursor0"

library_path() {
    printf '%s' "$lib:$lib/weston:$lib/libweston-14"
}

install() {
    mkdir -p "$prefix/debs" "$prefix/root"
    # `apt-get download` writes into the current directory.
    (cd "$prefix/debs" && apt-get download $packages)
    for deb in "$prefix"/debs/*.deb; do
        dpkg-deb -x "$deb" "$prefix/root"
    done
    missing=$(LD_LIBRARY_PATH=$(library_path) ldd "$prefix/root/usr/bin/weston" \
        "$lib/libweston-14/headless-backend.so" "$lib/weston/desktop-shell.so" \
        "$prefix/root/usr/libexec/weston-desktop-shell" \
        "$prefix/root/usr/bin/weston-screenshooter" | grep 'not found' || true)
    if [ -n "$missing" ]; then
        echo "weston still lacks libraries:" >&2
        echo "$missing" >&2
        exit 1
    fi
    echo "weston installed in $prefix"
}

run() {
    mkdir -p "$runtime"
    chmod 700 "$runtime"
    # Modules are found through the module map, not the packaged paths.
    modules="headless-backend.so=$lib/libweston-14/headless-backend.so"
    modules="$modules;desktop-shell.so=$lib/weston/desktop-shell.so"
    modules="$modules;kiosk-shell.so=$lib/weston/kiosk-shell.so"
    modules="$modules;weston-desktop-shell=$prefix/root/usr/libexec/weston-desktop-shell"
    modules="$modules;weston-screenshooter=$prefix/root/usr/bin/weston-screenshooter"
    config=$prefix/weston.ini
    cat >"$config" <<EOF
[core]
idle-time=0
[input-method]
path=
[shell]
client=$prefix/root/usr/libexec/weston-desktop-shell
background-color=0xff2e3440
panel-position=none
EOF
    XDG_RUNTIME_DIR=$runtime LD_LIBRARY_PATH=$(library_path) WESTON_MODULE_MAP=$modules \
        exec "$prefix/root/usr/bin/weston" --backend=headless --socket="$socket" \
        --renderer=pixman --config="$config" --width=1024 --height=768 "$@"
}

screenshot() {
    XDG_RUNTIME_DIR=$runtime WAYLAND_DISPLAY=$socket LD_LIBRARY_PATH=$(library_path) \
        exec "$prefix/root/usr/bin/weston-screenshooter" "$@"
}

case ${1:-} in
install) install ;;
run)
    shift
    run "$@"
    ;;
env) printf 'XDG_RUNTIME_DIR=%s WAYLAND_DISPLAY=%s\n' "$runtime" "$socket" ;;
screenshot)
    shift
    screenshot "$@"
    ;;
*)
    sed -n '2,15p' "$0" >&2
    exit 2
    ;;
esac
