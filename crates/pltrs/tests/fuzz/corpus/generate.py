"""Generate the image seed corpus of the fuzz tests (crates/pltrs/tests/fuzz).

Run from anywhere with the reference environment:

    .venv/bin/python -I crates/pltrs/tests/fuzz/corpus/generate.py

The fuzz tests corrupt these files and hand them to `imread`. They are the
variants pltrs' own encoders never write, so that corruption reaches
every decoder path: progressive, greyscale, CMYK and subsampled JPEG with
restart markers; compressed, 16-bit, palette, bilevel, CMYK, tiled and
multi-page TIFF; interlaced, transparent and animated GIF; palette,
16-bit and interlaced PNG; lossy and animated WebP. Images are tiny and
drawn from formulas, so the files are small and deterministic.
"""

from __future__ import annotations

import pathlib

import numpy as np
from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent


def gradient(width: int, height: int) -> np.ndarray:
    """An RGB gradient with a diagonal stripe."""
    y, x = np.mgrid[0:height, 0:width]
    rgb = np.stack([x * 255 // max(width - 1, 1), y * 255 // max(height - 1, 1), (x + y) % 7 * 36], axis=-1)
    return rgb.astype(np.uint8)


def rgb(width: int = 13, height: int = 9) -> Image.Image:
    return Image.fromarray(gradient(width, height), "RGB")


def rgba(width: int = 13, height: int = 9) -> Image.Image:
    image = rgb(width, height).convert("RGBA")
    alpha = Image.fromarray((np.arange(width * height).reshape(height, width) * 9 % 256).astype(np.uint8))
    image.putalpha(alpha)
    return image


def save(name: str, image: Image.Image, **options) -> None:
    image.save(HERE / name, **options)


def main() -> None:
    # JPEG: progressive 4:2:0 with restarts, baseline 4:2:2, 4:4:4
    # optimised, greyscale progressive, CMYK.
    save("progressive.jpg", rgb(19, 13), progressive=True, subsampling=2, restart_marker_blocks=2)
    save("baseline422.jpg", rgb(21, 9), subsampling=1, quality=60)
    save("optimized444.jpg", rgb(17, 11), subsampling=0, optimize=True, quality=90)
    save("grey.jpg", rgb(11, 10).convert("L"), progressive=True)
    save("cmyk.jpg", rgb(7, 5).convert("CMYK"))
    # TIFF: LZW with a predictor, PackBits, Deflate, 16-bit grey, palette,
    # bilevel Group 4, CMYK, tiled, two pages.
    save("lzw.tif", rgba(), compression="tiff_lzw", tiffinfo={317: 2})
    save("packbits.tif", rgb(), compression="packbits")
    save("deflate.tif", rgb(), compression="tiff_adobe_deflate")
    grey16 = Image.fromarray((np.arange(63).reshape(7, 9) * 997).astype(np.uint16))
    save("grey16.tif", grey16)
    save("palette.tif", rgb().convert("P"))
    save("group4.tif", rgb().convert("1"), compression="group4")
    save("cmyk.tif", rgb().convert("CMYK"))
    save("tiled.tif", rgb(40, 36), tile=(16, 16))
    save("pages.tif", rgb(), save_all=True, append_images=[rgb(5, 4)])
    # GIF: interlaced with transparency, animated with disposal.
    save("interlaced.gif", rgba().convert("P"), interlace=True, transparency=0)
    frames = [rgb(9, 7).convert("P"), rgb(9, 7).transpose(Image.Transpose.FLIP_LEFT_RIGHT).convert("P")]
    save("animated.gif", frames[0], save_all=True, append_images=frames[1:], duration=40, loop=0, disposal=2)
    # PNG: palette with transparency, 16-bit grey, interlaced.
    save("palette.png", rgba().convert("P"), transparency=3)
    save("grey16.png", Image.fromarray(gradient(6, 5).astype(np.uint16)[..., 0] * 257))
    save("interlaced.png", rgba(), interlace=True)
    # WebP: lossy with alpha, animated.
    save("lossy.webp", rgba(), quality=50)
    save("animated.webp", rgb(9, 7), save_all=True, append_images=[rgb(9, 7).rotate(90)], duration=40)


if __name__ == "__main__":
    main()
