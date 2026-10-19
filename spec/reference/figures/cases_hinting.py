"""Hinted text: matplotlib's default ``text.hinting`` ("default", FreeType's
``FT_LOAD_DEFAULT``), "native" (``FT_LOAD_NO_AUTOHINT``) and
"force_autohint"/"auto" (``FT_LOAD_FORCE_AUTOHINT``) instead of the
reference configuration's unhinted text. The bundled TrueType fonts with
bytecode (DejaVu, the BaKoMa Computer Modern fonts) are hinted by FreeType's
version 40 interpreter under "default" and "native"; fonts without bytecode
(STIX) get FreeType's autohinter under "default" and the interpreter under
"native"; "force_autohint" autohints every glyph drawn upright or at a
multiple of 90 degrees.

Plain text, tick labels, titles, labels, legends, rotated text and mathtext
are drawn at several sizes (6-24 pt) and resolutions (72, 100 and 150 dpi),
also without anti-aliasing (FreeType's monochrome rasterizer).
"""

import numpy as np

from spec_support import case

DEFAULT = {"text.hinting": "default"}
NATIVE = {"text.hinting": "native"}
AUTOHINT = {"text.hinting": "force_autohint"}

SAMPLE = "Hamburgefonstiv 0123456789 ÅÉçß ffi Wa.To, (x)"
SIZES = [6, 7, 8, 9, 10, 11, 12, 14, 17, 20, 24]


def _sizes(fig, sizes=SIZES, family=None):
    """One line of the sample per size, top to bottom."""
    for i, size in enumerate(sizes):
        fig.text(0.02, 0.96 - 0.085 * i, f"{size} pt {SAMPLE}", fontsize=size,
                 va="top", family=family)


def _axes(fig):
    """A plot with title, labels, tick labels and a legend."""
    ax = fig.subplots()
    x = np.linspace(0.0, 10.0, 40)
    ax.plot(x, np.sin(x), label="sin(x)")
    ax.plot(x, 0.5 * np.cos(x), label="0.5 cos(x)")
    ax.set_title("Hinted title: Waveform AVA")
    ax.set_xlabel("time [s]")
    ax.set_ylabel("amplitude")
    ax.legend(loc="upper right")


def _rotated(fig):
    """Labels at eight angles."""
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.set_axis_off()
    for i, angle in enumerate([0, 15, 30, 45, 90, 135, 180, 270]):
        ax.text(0.6 + 1.2 * i, 5, f"rot {angle} Wa", rotation=angle, fontsize=10 + i,
                ha="center", va="center")


def _math(fig):
    """The formula at four sizes."""
    for i, size in enumerate([8, 11, 15, 20]):
        fig.text(0.03, 0.9 - 0.23 * i, r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b} - \pi r^2$",
                 fontsize=size, va="top")


def _stix(fig, formula):
    """The sample in STIX General at three sizes, then a formula."""
    for i, size in enumerate([7, 10, 14]):
        fig.text(0.03, 0.92 - 0.2 * i, f"{size} pt {SAMPLE}", fontsize=size, family="STIXGeneral",
                 va="top")
    fig.text(0.03, 0.3, formula, fontsize=12, va="top")


@case("hinting/sizes_100dpi", figsize=(6, 4.5), rc=DEFAULT)
def _(fig):
    _sizes(fig)


@case("hinting/sizes_72dpi", figsize=(8, 6), dpi=72, rc=DEFAULT)
def _(fig):
    _sizes(fig)


@case("hinting/sizes_150dpi", figsize=(4.5, 3.4), dpi=150, rc=DEFAULT)
def _(fig):
    _sizes(fig, sizes=[6, 8, 10, 12, 14, 18, 24])


@case("hinting/serif_and_mono", figsize=(6, 3), rc=DEFAULT)
def _(fig):
    for i, family in enumerate(["DejaVu Serif", "DejaVu Sans Mono"]):
        for j, (size, weight, style) in enumerate([(9, "normal", "normal"),
                                                   (11, "bold", "normal"),
                                                   (13, "normal", "italic")]):
            fig.text(0.02 + 0.5 * i, 0.9 - 0.3 * j, f"{family} {size}", fontsize=size,
                     family=family, weight=weight, style=style, va="top")


@case("hinting/axes_100dpi", figsize=(5, 3.75), rc=DEFAULT)
def _(fig):
    _axes(fig)


@case("hinting/axes_72dpi", figsize=(5, 3.75), dpi=72, rc=DEFAULT)
def _(fig):
    _axes(fig)


@case("hinting/axes_150dpi", figsize=(4, 3), dpi=150, rc=DEFAULT)
def _(fig):
    _axes(fig)


@case("hinting/rotated", figsize=(6, 3), rc=DEFAULT)
def _(fig):
    _rotated(fig)


@case("hinting/mathtext_dejavusans", figsize=(6, 3), rc={**DEFAULT, "mathtext.fontset": "dejavusans"})
def _(fig):
    _math(fig)


@case("hinting/mathtext_cm", figsize=(6, 3), rc={**DEFAULT, "mathtext.fontset": "cm"})
def _(fig):
    _math(fig)


@case("hinting/native_stix", figsize=(6, 3), rc={**NATIVE, "mathtext.fontset": "stix"})
def _(fig):
    _stix(fig, r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b}$ and DejaVu text")


@case("hinting/native_axes", figsize=(5, 3.75), rc=NATIVE)
def _(fig):
    _axes(fig)


@case("hinting/autohint_sizes_100dpi", figsize=(6, 4.5), rc=AUTOHINT)
def _(fig):
    _sizes(fig)


@case("hinting/autohint_sizes_150dpi", figsize=(4.5, 3.4), dpi=150, rc={"text.hinting": "auto"})
def _(fig):
    _sizes(fig, sizes=[6, 8, 10, 12, 14, 18, 24])


@case("hinting/autohint_axes_72dpi", figsize=(5, 3.75), dpi=72, rc=AUTOHINT)
def _(fig):
    _axes(fig)


@case("hinting/autohint_serif_and_mono", figsize=(6, 3), rc=AUTOHINT)
def _(fig):
    for i, family in enumerate(["DejaVu Serif", "DejaVu Sans Mono"]):
        for j, (size, weight, style) in enumerate([(9, "normal", "normal"),
                                                   (11, "bold", "normal"),
                                                   (13, "normal", "italic")]):
            fig.text(0.02 + 0.5 * i, 0.9 - 0.3 * j, f"{family} {size} 0123 Ãñ",
                     fontsize=size, family=family, weight=weight, style=style, va="top")


@case("hinting/autohint_rotated", figsize=(6, 3), rc=AUTOHINT)
def _(fig):
    _rotated(fig)


@case("hinting/mathtext_stix", figsize=(6, 3), rc={**DEFAULT, "mathtext.fontset": "stix"})
def _(fig):
    _math(fig)


@case("hinting/stix_default", figsize=(6, 3), rc={**DEFAULT, "mathtext.fontset": "stix"})
def _(fig):
    _stix(fig, r"$\sum_{i=0}^{n} \sqrt{x_i} \int f\,dx$ and DejaVu text")


# --- without anti-aliasing: FreeType's monochrome rasterizer, with the
# drop-out rules each loader sets (bytecode scan control, none, autohinter)

ALIASED = {"text.antialiased": False}


@case("hinting/aliased_sizes", figsize=(6, 4.5), rc={**DEFAULT, **ALIASED})
def _(fig):
    _sizes(fig)


@case("hinting/aliased_unhinted_sizes", figsize=(6, 4.5), rc=ALIASED)
def _(fig):
    _sizes(fig)


@case("hinting/aliased_unhinted_rotated", figsize=(6, 3), rc=ALIASED)
def _(fig):
    _rotated(fig)


@case("hinting/aliased_autohint_rotated", figsize=(6, 3), rc={**AUTOHINT, **ALIASED})
def _(fig):
    _rotated(fig)


@case("hinting/aliased_mathtext_cm", figsize=(6, 3),
      rc={**DEFAULT, **ALIASED, "mathtext.fontset": "cm"})
def _(fig):
    _math(fig)


@case("hinting/aliased_native_stix", figsize=(6, 3),
      rc={**NATIVE, **ALIASED, "mathtext.fontset": "stix"})
def _(fig):
    _stix(fig, r"$x^2 + y_i = \alpha\beta\gamma\,\frac{a}{b}$ and DejaVu text")


@case("hinting/aliased_large", figsize=(6, 3), rc=ALIASED)
def _(fig):
    # Glyphs too tall for high precision, and a W whose profiles overflow
    # FreeType's render pool, so that it is rendered in bands.
    fig.text(0.02, 0.1, "BW%", fontsize=150)
    fig.text(0.62, 0.05, "W", fontsize=300)
