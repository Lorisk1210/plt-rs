"""Text in the standard PostScript fonts: the PDF backend's 14 core fonts
(``pdf.use14corefonts``) and the PostScript backend's AFM fonts
(``ps.useafm``) measure, lay out and draw text that is not mathtext with
Adobe Font Metrics. The PNG renderings ignore these settings; the vector
documents (``test_vectors.py``) show them."""

import numpy as np

from spec_support import case

CORE_FONTS = {"pdf.use14corefonts": True}
AFM_FONTS = {"ps.useafm": True}


def _gallery(fig):
    """Families, weights and styles, kerning pairs, Latin-1 and other
    characters, alignment, rotation, two lines and a mathtext line."""
    fig.text(0.02, 0.92, "AVATAR Toyota WAVE", size=14)
    fig.text(0.02, 0.8, "serif: AVATAR Toyota", family="serif", size=12)
    fig.text(0.02, 0.69, "monospace bold", family="monospace", weight="bold")
    fig.text(0.02, 0.59, "Helvetica bold oblique", family="Helvetica", weight="bold",
             style="oblique")
    fig.text(0.02, 0.49, "Times italic", family="Times", style="italic", size=11)
    fig.text(0.02, 0.38, "café naïve Ångström ±½×©",
             size=11)
    fig.text(0.02, 0.27, "€ – “quotes” → α −", size=11)
    fig.text(0.02, 0.1, r"math $\alpha^2+\frac{1}{2}$", size=11)
    fig.text(0.98, 0.95, "top right", ha="right", va="top")
    fig.text(0.75, 0.62, "centred\ntwo lines", ha="center", va="center", ma="center", size=12)
    fig.text(0.7, 0.25, "rotated", rotation=30, ha="center")
    fig.text(0.92, 0.1, "anchor", rotation=90, rotation_mode="anchor", va="bottom")


def _layout(fig):
    """A title, labels, tick labels, a legend and a suptitle placed by the
    constrained layout engine, which measures text at draw time."""
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = np.linspace(-3, 3, 40)
    ax.plot(x, np.sin(x), label="sin AVATAR")
    ax.plot(x, np.cos(x), label="cos Toyota")
    ax.set_title("Title: WAVE")
    ax.set_xlabel("time (s)")
    ax.set_ylabel("amplitude")
    ax.legend(loc="upper left")
    fig.suptitle("Suptitle")


@case("afm/core_fonts_text", figsize=(5, 3), rc=CORE_FONTS)
def _(fig):
    _gallery(fig)


@case("afm/core_fonts_layout", figsize=(4, 3), rc=CORE_FONTS)
def _(fig):
    _layout(fig)


@case("afm/ps_fonts_text", figsize=(5, 3), rc=AFM_FONTS)
def _(fig):
    _gallery(fig)


@case("afm/ps_fonts_layout", figsize=(4, 3), rc=AFM_FONTS)
def _(fig):
    _layout(fig)
