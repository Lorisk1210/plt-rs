"""
Whether two directories hold the same drawings: every PNG of THEIRS has a
PNG of the same name in OURS with the same pixels (RGBA). Run by
``check_dist.py --compare-drawings`` with the interpreter of a venv that has
Pillow and numpy (pltrs requires both):

    python same_pixels.py THEIRS OURS

prints a line per drawing and exits with 1 if one differs or is missing.
"""

import pathlib
import sys

import numpy as np
from PIL import Image


def pixels(path):
    """The RGBA pixels of the PNG at *path*."""
    with Image.open(path) as image:
        return np.asarray(image.convert("RGBA"), dtype=np.int16)


def main(argv):
    """Compares the drawings and returns the exit status."""
    theirs, ours = map(pathlib.Path, argv)
    status = 0
    for reference in sorted(theirs.glob("*.png")):
        drawn = ours / reference.name
        if not drawn.exists():
            print(f"{reference.name}: not drawn")
            status = 1
            continue
        expected, got = pixels(reference), pixels(drawn)
        if expected.shape != got.shape:
            print(f"{reference.name}: {got.shape[1]}x{got.shape[0]}, not "
                  f"{expected.shape[1]}x{expected.shape[0]}")
            status = 1
            continue
        difference = np.abs(expected - got)
        if difference.any():
            rms = float(np.sqrt(np.mean(difference.astype(float) ** 2)))
            print(f"{reference.name}: {int((difference.max(axis=2) > 0).sum())} pixels differ "
                  f"(RMS {rms:.4f})")
            status = 1
        else:
            print(f"{reference.name}: the same pixels")
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
