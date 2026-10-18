"""Figures for the vector backends: rasterized artists, a colorbar with
enough colours to be rasterized, and text to embed fonts for."""

from __future__ import annotations

import numpy as np

from spec_support import case


def _cells():
    """An 8 x 12 field of smooth values."""
    return np.fromfunction(lambda i, j: np.sin(0.4 * i) + np.cos(0.3 * j), (8, 12))


@case("vectors/rasterized_artists", figsize=(2.4, 1.6), dpi=50)
def _(fig):
    ax = fig.subplots()
    ax.pcolormesh(_cells(), rasterized=True)
    x = np.linspace(0, 11, 30)
    ax.plot(x, 0.6 * x, color="k", linewidth=2)
    ax.scatter(x, 7 - 0.5 * x, c=x, rasterized=True, zorder=3)
    ax.plot(x, 4 + 2 * np.sin(x), rasterized=True, zorder=4)
    ax.set_title("rasterized")


@case("vectors/rasterization_zorder", figsize=(2.4, 1.6), dpi=50)
def _(fig):
    ax = fig.subplots()
    ax.set_rasterization_zorder(1.5)
    ax.pcolormesh(_cells())
    x = np.linspace(0, 11, 30)
    ax.plot(x, 0.6 * x, color="k", zorder=1.5)
    ax.plot(x, 4 + 2 * np.sin(x))


def _half_opacity(image, dpi):
    """The image at half opacity, dpi/25 to the right."""
    out = image.copy()
    out[..., 3] = out[..., 3] * 0.5
    return out, dpi / 25, 0


@case("vectors/agg_filter", figsize=(2.4, 1.6), dpi=50)
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 11, 30)
    filtered, = ax.plot(x, 0.6 * x, linewidth=4, rasterized=True)
    filtered.set_agg_filter(_half_opacity)
    ignored, = ax.plot(x, 6 - 0.5 * x, linewidth=4)
    ignored.set_agg_filter(_half_opacity)


@case("vectors/colorbar_many_colors", figsize=(2.4, 1.6), dpi=50)
def _(fig):
    ax = fig.subplots()
    image = ax.imshow(_cells())
    fig.colorbar(image, ax=ax)


@case("vectors/text_fonts", figsize=(3, 1.2))
def _(fig):
    fig.text(0.04, 0.65, "office fit caf\N{LATIN SMALL LETTER E WITH ACUTE}", size=12)
    fig.text(0.04, 0.2, r"$\alpha^2+\frac{1}{2}$", size=12)
    fig.text(0.6, 0.2, "\N{MINUS SIGN}1.5", size=12)
