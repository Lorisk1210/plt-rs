#!/bin/sh
# GTK 3 and GTK 4 with PyGObject and pycairo, and wxPython, for testing
# pltrs (Python)'s GTK and wx backends against matplotlib's, installed
# without root.
#
#   python/tools/toolkits.sh install         download, unpack and build into $TOOLKITS_PREFIX
#   python/tools/toolkits.sh env             print the variables a program needs
#   python/tools/toolkits.sh run CMD [ARG]   run CMD with them
#
# GTK's libraries, typelibs and data come from the distribution's packages
# (`apt-get download`, unpacked with `dpkg-deb -x` into $TOOLKITS_PREFIX/root),
# with the headers PyGObject and pycairo build against. Both build from
# their sources (PyPI has no Linux wheels) into $TOOLKITS_PREFIX/site-<tag>,
# for $TOOLKITS_PYTHON (default: the reference venv's, `.venv`);
# `.venv-pltrs` has the same CPython 3.14, so the site serves both.
# wxPython is the wheel its project builds for Ubuntu 24.04 and CPython
# 3.14 (glibc 2.39, older than Debian 13's), whose one library Debian
# lacks, libjpeg.so.8, comes from Ubuntu's archive.
#
# With a display (`Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &`, then
# DISPLAY=:117), matplotlib's suite runs with the toolkits:
#
#   DISPLAY=:117 python/tools/toolkits.sh run cargo xtask mpl-suite test_backend_gtk3
#
# $TOOLKITS_PREFIX defaults to ~/.local/gtk. The package list is for
# Debian 13 (trixie): what a system without a desktop lacks. Rerunning
# `install` is harmless.
set -eu

prefix=${TOOLKITS_PREFIX:-$HOME/.local/gtk}
root=$prefix/root
lib=$root/usr/lib/x86_64-linux-gnu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
python=${TOOLKITS_PYTHON:-$repo/.venv/bin/python}

# GTK 3 and 4, their typelibs for PyGObject and the libraries they need.
gtk_packages="libgtk-3-0t64 libgtk-3-common gir1.2-gtk-3.0 libgtk-4-1
libgtk-4-common gir1.2-gtk-4.0 libgraphene-1.0-0 gir1.2-graphene-1.0
libgirepository-2.0-0 gir1.2-glib-2.0 gir1.2-freedesktop gir1.2-gdkpixbuf-2.0
gir1.2-pango-1.0 gir1.2-harfbuzz-0.0 gir1.2-atk-1.0 libcairo2
libcairo-gobject2 libcairo-script-interpreter2 libpango-1.0-0
libpangocairo-1.0-0 libpangoft2-1.0-0 libpangoxft-1.0-0 libharfbuzz0b
libharfbuzz-gobject0 libharfbuzz-subset0 libfribidi0 libthai0 libthai-data
libdatrie1 libgraphite2-3 libepoxy0 libxcb-render0 libwayland-cursor0
libwayland-egl1 libcolord2 liblcms2-2 libcups2t64 libavahi-client3
libavahi-common3 libavahi-common-data libcloudproviders0 librsvg2-2
librsvg2-common libdav1d7 liblzo2-2 libpcre2-16-0 libpcre2-32-0
libpcre2-posix3 adwaita-icon-theme hicolor-icon-theme"

# What PyGObject and pycairo compile against: pkg-config and the headers
# of glib, girepository, cairo and of what their pkg-config files require.
build_packages="pkgconf pkgconf-bin libpkgconf3 libglib2.0-dev
libglib2.0-dev-bin libgio-2.0-dev libgio-2.0-dev-bin libgirepository-2.0-dev
libcairo2-dev libffi-dev libpcre2-dev libmount-dev libblkid-dev
libselinux1-dev libsepol-dev libsysprof-capture-4-dev zlib1g-dev
libfontconfig-dev libfreetype-dev libpng-dev libbrotli-dev libbz2-dev
libexpat1-dev uuid-dev libpixman-1-dev libx11-dev libxau-dev libxdmcp-dev
libxcb1-dev libxcb-render0-dev libxcb-shm0-dev libxext-dev libxrender-dev
libice-dev libsm-dev x11proto-dev xtrans-dev"

# wxPython's own libraries need SDL 2 besides GTK 3. Its `wx.lib.wxcairo`
# (matplotlib's wxcairo) finds GDK and pangocairo with
# `ctypes.util.find_library`, which looks for their development links.
wx_packages="libsdl2-2.0-0 libdecor-0-0 libsamplerate0 libxss1 libpulse0
libasyncns0 libsndfile1 libflac14 libogg0 libvorbis0a libvorbisenc2 libopus0
libmpg123-0t64 libmp3lame0 libgtk-3-dev libpango1.0-dev"

pycairo=pycairo==1.29.2
pygobject=PyGObject==3.58.1
wx_wheel=wxpython-4.2.5-cp314-cp314-linux_x86_64.whl
wx_url=https://extras.wxpython.org/wxPython4/extras/linux/gtk3/ubuntu-24.04/$wx_wheel
wx_sha256=619d3a48225f072ab7de35267af13c7b801d2ce5c71ba1806a8f8e0ffeea69ec
jpeg_deb=libjpeg-turbo8_2.1.5-2ubuntu2_amd64.deb
jpeg_url=http://archive.ubuntu.com/ubuntu/pool/main/libj/libjpeg-turbo/$jpeg_deb
jpeg_sha256=f68b5b23bc8a1688fb787d2aed7e2cdf895a73022f6a5025e183162dac4500b2

# The site directory of the packages built for $python.
site() {
    printf '%s/site-%s' "$prefix" "$("$python" -c 'import sys; print(sys.implementation.cache_tag)')"
}

# Downloads $1 from $2 into the download directory unless it is there,
# and checks its SHA-256 against $3.
fetch() {
    file=$prefix/debs/$1
    if [ ! -f "$file" ]; then
        curl -sSfL -o "$file.part" "$2"
        mv "$file.part" "$file"
    fi
    echo "$3  $file" | sha256sum -c --quiet -
}

unpack() {
    mkdir -p "$prefix/debs" "$root"
    # `apt-get download` writes into the current directory.
    (cd "$prefix/debs" && apt-get download $gtk_packages $build_packages $wx_packages)
    fetch "$jpeg_deb" "$jpeg_url" "$jpeg_sha256"
    for deb in "$prefix"/debs/*.deb; do
        dpkg-deb -x "$deb" "$root"
    done
    # The development packages' `libX.so` links name libraries the
    # system has, not the prefix: point them at the system's.
    find "$lib" -maxdepth 1 -name '*.so' -type l | while read -r link; do
        target=$(readlink "$link")
        if [ ! -e "$lib/$target" ] && [ -e "/usr/lib/x86_64-linux-gnu/$target" ]; then
            ln -sf "/usr/lib/x86_64-linux-gnu/$target" "$link"
        fi
    done
    # The pkg-config files' directories (prefix, libdir, ...) are under /usr.
    sed -i "s#^\([a-z_]*\)=/usr\(/\|\$\)#\1=$root/usr\2#" \
        "$lib"/pkgconfig/*.pc "$root"/usr/share/pkgconfig/*.pc
}

# The settings schemas and the image loaders' cache, which the packages'
# install scripts would make, with the system's glib and gdk-pixbuf tools.
configure() {
    system=/usr/lib/x86_64-linux-gnu
    "$system/glib-2.0/glib-compile-schemas" "$root/usr/share/glib-2.0/schemas"
    LD_LIBRARY_PATH=$lib "$system/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders" \
        "$system"/gdk-pixbuf-2.0/2.10.0/loaders/*.so \
        "$lib"/gdk-pixbuf-2.0/2.10.0/loaders/*.so >"$prefix/loaders.cache"
}

# pkg-config over the prefix's files only (and those meson adds, such as
# Python's own).
pkg_config_wrapper() {
    mkdir -p "$prefix/bin"
    cat >"$prefix/bin/pkg-config" <<EOF
#!/bin/sh
export LD_LIBRARY_PATH="$lib"
export PKG_CONFIG_LIBDIR="$lib/pkgconfig:$root/usr/share/pkgconfig"
exec "$root/usr/bin/pkgconf" "\$@"
EOF
    chmod +x "$prefix/bin/pkg-config"
}

build_python() {
    pkg_config_wrapper
    fetch "$wx_wheel" "$wx_url" "$wx_sha256"
    uv=${UV:-$HOME/.local/bin/uv}
    # The multiarch headers (libffi's) are where the compiler looks only
    # under /usr.
    PKG_CONFIG=$prefix/bin/pkg-config CPATH=$root/usr/include/x86_64-linux-gnu \
        LIBRARY_PATH=$lib LD_LIBRARY_PATH=$lib \
        "$uv" pip install --python "$python" --target "$(site)" --no-deps \
        "$pycairo" "$pygobject" "$prefix/debs/$wx_wheel"
}

# Fails naming every library the toolkits' modules cannot find.
check() {
    missing=$(env $(variables) sh -c 'for so in "$@"; do ldd "$so"; done' check \
        "$lib"/libgtk-3.so.0 "$lib"/libgtk-4.so.1 "$(site)"/gi/_gi*.so \
        "$(site)"/cairo/_cairo*.so "$(site)"/wx/_core*.so "$(site)"/wx/_adv*.so \
        "$(site)"/wx/svg/_nanosvg*.so | grep 'not found' | sort -u || true)
    if [ -n "$missing" ]; then
        echo "the toolkits still lack libraries:" >&2
        echo "$missing" >&2
        exit 1
    fi
    env $(variables) "$python" -c '
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
print("GTK 3:", gi.Repository.get_default().enumerate_versions("Gtk"))
import cairo, wx
print("pycairo", cairo.version, "wxPython", wx.version())
'
}

install() {
    unpack
    configure
    build_python
    check
    echo "toolkits installed in $prefix"
}

# The environment a program using the toolkits runs with, as NAME=value
# words.
variables() {
    # PulseAudio's client library (SDL's) finds its own in a subdirectory.
    printf 'LD_LIBRARY_PATH=%s ' "$lib:$lib/pulseaudio${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    printf 'PYTHONPATH=%s ' "$(site)${PYTHONPATH:+:$PYTHONPATH}"
    printf 'GI_TYPELIB_PATH=%s ' "$lib/girepository-1.0"
    printf 'XDG_DATA_DIRS=%s ' "$root/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
    printf 'GSETTINGS_SCHEMA_DIR=%s ' "$root/usr/share/glib-2.0/schemas"
    printf 'GSETTINGS_BACKEND=memory '
    printf 'GDK_PIXBUF_MODULE_FILE=%s ' "$prefix/loaders.cache"
    printf 'GTK_PATH=%s ' "$lib/gtk-3.0:$lib/gtk-4.0"
    # There is no accessibility bus to warn about.
    printf 'NO_AT_BRIDGE=1 GTK_A11Y=none\n'
}

case ${1:-} in
install) install ;;
env) variables ;;
run)
    shift
    exec env $(variables) "$@"
    ;;
*)
    sed -n '2,27p' "$0" >&2
    exit 2
    ;;
esac
