#!/bin/sh
# What matplotlib's test suite looks for beyond Python, Ghostscript and TeX,
# installed without root, so that its tests skipping for a missing tool
# run (`cargo xtask mpl-suite`, both modes):
#
#   python/tools/suite-tools.sh install [PART...]  download, verify and unpack into
#                                                  $SUITE_TOOLS_PREFIX (inkscape fonts tex)
#   python/tools/suite-tools.sh env                print the PATH line and the suite's variables
#   python/tools/suite-tools.sh run CMD [ARG...]   run CMD with both (pytest by hand, say)
#
# * Inkscape 1.4.4 (dcaf3e7), the official AppImage, verified against its
#   published SHA-256 and unpacked with `--appimage-extract` into
#   $SUITE_TOOLS_PREFIX/inkscape: `testing.compare` rasterises SVG files
#   with it, so every SVG image comparison runs. Only its `bin` goes on
#   PATH (the AppImage's own `usr/bin` holds a Python and more):
#
#     export PATH="$HOME/.local/mpl-suite/inkscape/bin:$PATH"
#
# * Fonts the tests ask for by name, into $SUITE_TOOLS_PREFIX/share/fonts:
#   WenQuanYi Zen Hei (wqy-zenhei.ttc, GPL 2 with a font exception), FreeMono
#   (FreeMono.otf, GPL 3 with a font exception), Noto Sans CJK
#   (NotoSansCJK-Regular.ttc, SIL OFL 1.1), from Debian 13's packages
#   (`apt-get download` checks them against the archive's signed index),
#   and Noto Sans (NotoSans-Regular.otf 2.015, SIL OFL 1.1) from the Noto
#   project's repository at a fixed commit, checked against its git blob id.
#   The licences go to $SUITE_TOOLS_PREFIX/share/licenses.
# * TeX packages through TinyTeX's `tlmgr`, in user mode into a tree of
#   their own ($SUITE_TOOLS_PREFIX/texmf) with font maps of their own
#   ($SUITE_TOOLS_PREFIX/texmf-var): pgf and koma-script (the PGF backend's
#   tests), and ucs, heuristica, dejavu, concmath and chemformula, which
#   single tests load. The TeX installation itself is left as it is.
#
# Nothing goes where other programs look: no font directory fontconfig,
# matplotlib or pltrs scans, no TeX tree TeX reads by default. The suite's
# runs get the fonts and the TeX tree from the variables in
# $SUITE_TOOLS_PREFIX/environment (XDG_DATA_HOME, TEXMFHOME, TEXMFVAR,
# TEXMFCONFIG), which `cargo xtask mpl-suite` sets for both modes when the
# file exists; Inkscape it finds on PATH, like Ghostscript and TeX.
#
# $SUITE_TOOLS_PREFIX defaults to ~/.local/mpl-suite (the runner reads the
# same variable). TinyTeX's `tlmgr` must be on PATH. Rerunning `install` is
# harmless.
set -eu

prefix=${SUITE_TOOLS_PREFIX:-$HOME/.local/mpl-suite}

inkscape_version=1.4.4
inkscape_url=https://inkscape.org/gallery/item/59506/Inkscape-1.4.4.AppImage
inkscape_sha256=d217fe05c92faaed5c3af46856b6299c98933dc722819cb5bc2ca052c5522166

# Debian packages and the font files of each the suite asks for.
font_packages="fonts-wqy-zenhei fonts-freefont-otf fonts-noto-cjk"
font_files="usr/share/fonts/truetype/wqy/wqy-zenhei.ttc
usr/share/fonts/opentype/freefont/FreeMono.otf
usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"

noto_commit=28b15b4b43b7bed62b5cf6e6b0b5ff5846270535
noto_path=fonts/NotoSans/unhinted/otf/NotoSans-Regular.otf
noto_blob=b8f04a5918e1a2ebb17c60e5aefda6a61127bf47

# TeX packages, with those of their dependencies the installation lacks:
# pgf (fp), koma-script (xpatch; the PGF backend's preamble sizes fonts with
# its scrextend), and what single tests load: ucs, heuristica (cyrillic,
# ly1, fontaxes, mweights), dejavu, concmath (concmath-fonts, concrete),
# chemformula (units, xfrac).
tex_packages="pgf fp koma-script xpatch ucs heuristica cyrillic ly1 fontaxes mweights
dejavu concmath concmath-fonts concrete chemformula units xfrac"

# A new, empty directory for one download.
fresh() {
    rm -rf "$1"
    mkdir -p "$1"
}

install_inkscape() {
    downloads=$prefix/downloads/inkscape
    fresh "$downloads"
    appimage=$downloads/Inkscape-$inkscape_version.AppImage
    curl -fsSL -o "$appimage" "$inkscape_url"
    echo "$inkscape_sha256  $appimage" | sha256sum -c -
    chmod +x "$appimage"
    unpack=$prefix/downloads/inkscape-unpacked
    fresh "$unpack"
    # The AppImage's runtime unpacks into ./squashfs-root.
    (cd "$unpack" && "$appimage" --appimage-extract >/dev/null)
    rm -rf "$prefix/inkscape"
    mkdir -p "$prefix/inkscape/bin"
    mv "$unpack/squashfs-root" "$prefix/inkscape/app"
    rm -rf "$unpack" "$downloads"
    # Inkscape is a GApplication: on a session bus, instances started at
    # once (pytest's workers) race to register its name and lose with
    # Gio::DBus::Error. Without a bus GLib runs each on its own.
    cat >"$prefix/inkscape/bin/inkscape" <<EOF
#!/bin/sh
DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent exec "$prefix/inkscape/app/AppRun" "\$@"
EOF
    chmod +x "$prefix/inkscape/bin/inkscape"
    "$prefix/inkscape/bin/inkscape" --version 2>/dev/null
}

install_fonts() {
    downloads=$prefix/downloads/fonts
    fresh "$downloads"
    # `apt-get download` writes into the current directory.
    (cd "$downloads" && apt-get download $font_packages)
    unpack=$prefix/downloads/fonts-unpacked
    fresh "$unpack"
    for deb in "$downloads"/*.deb; do
        dpkg-deb -x "$deb" "$unpack"
    done
    fonts=$prefix/share/fonts
    licenses=$prefix/share/licenses
    fresh "$fonts"
    fresh "$licenses"
    for file in $font_files; do
        cp "$unpack/$file" "$fonts/"
    done
    for package in $font_packages; do
        cp "$unpack/usr/share/doc/$package/copyright" "$licenses/$package"
    done
    noto=$downloads/NotoSans-Regular.otf
    curl -fsSL -o "$noto" \
        "https://raw.githubusercontent.com/notofonts/notofonts.github.io/$noto_commit/$noto_path"
    curl -fsSL -o "$licenses/noto-sans" \
        "https://raw.githubusercontent.com/notofonts/notofonts.github.io/$noto_commit/fonts/LICENSE"
    blob=$(git hash-object "$noto")
    if [ "$blob" != "$noto_blob" ]; then
        echo "NotoSans-Regular.otf: git blob $blob, expected $noto_blob" >&2
        exit 1
    fi
    cp "$noto" "$fonts/"
    rm -rf "$unpack" "$downloads"
    ls "$fonts"
}

# The variables giving the suite its fonts and its TeX tree.
environment() {
    cat <<EOF
XDG_DATA_HOME=$prefix/share
TEXMFHOME=$prefix/texmf
TEXMFVAR=$prefix/texmf-var
TEXMFCONFIG=$prefix/texmf-config
EOF
}

install_tex() {
    for tree in texmf texmf-var texmf-config; do
        fresh "$prefix/$tree"
    done
    # tlmgr's logs and the font maps updmap writes go to the tree's own
    # TEXMFVAR and TEXMFCONFIG.
    export TEXMFHOME="$prefix/texmf" TEXMFVAR="$prefix/texmf-var" \
        TEXMFCONFIG="$prefix/texmf-config"
    tlmgr init-usertree --usertree "$TEXMFHOME"
    # The packages' dependencies the installation lacks are in the list:
    # with them, user mode would also copy those it has. tlmgr's own
    # follow-up would re-index the installation's trees (mktexlsr), so the
    # user tree's font maps are enabled here instead: its updmap.cfg lists
    # them, and updmap-user writes the installation's maps plus those.
    tlmgr --usermode --usertree "$TEXMFHOME" --no-execute-actions install --no-depends \
        $tex_packages
    mkdir -p "$TEXMFHOME/web2c"
    sed -n 's/^execute add\(Map\|MixedMap\) \(.*\)/\1 \2/p' \
        "$TEXMFHOME"/tlpkg/tlpobj/*.tlpobj >"$TEXMFHOME/web2c/updmap.cfg"
    updmap-user >/dev/null
}

install() {
    mkdir -p "$prefix"
    for part in ${*:-inkscape fonts tex}; do
        case $part in
        inkscape) install_inkscape ;;
        fonts) install_fonts ;;
        tex) install_tex ;;
        *)
            echo "unknown part $part (inkscape, fonts, tex)" >&2
            exit 2
            ;;
        esac
    done
    environment >"$prefix/environment"
    echo "installed in $prefix; put $prefix/inkscape/bin on PATH"
}

case ${1:-} in
install)
    shift
    install "$@"
    ;;
env)
    echo "PATH=$prefix/inkscape/bin:\$PATH"
    environment
    ;;
run)
    shift
    export PATH="$prefix/inkscape/bin:$PATH"
    while IFS='=' read -r name value; do
        export "$name=$value"
    done <<EOF
$(environment)
EOF
    exec "$@"
    ;;
*)
    sed -n '2,8p' "$0" >&2
    exit 2
    ;;
esac
