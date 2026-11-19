"""
The pictures the two sides drew: whether they are the same, and one
image of a run's figures for the clip.
"""

import math

from PIL import Image, ImageChops

#: The largest RMS (over 0 to 255) at which pltrs counts two pictures as
#: the same, as its checks against matplotlib do.
SAME_RMS = 1.0
#: The gap between figures put side by side in one picture.
GAP = 16


def rms(first, second):
    """The root mean square of the difference of two images of one size,
    over their RGB channels, on 0 to 255."""
    difference = ImageChops.difference(first.convert("RGB"), second.convert("RGB"))
    histogram = difference.histogram()
    squares = sum(count * (value % 256) ** 2 for value, count in enumerate(histogram))
    return math.sqrt(squares / (first.width * first.height * 3))


def compare(matplotlib_paths, pltrs_paths):
    """Whether the two sides' pictures are the same: a dictionary with the
    largest ``rms`` and a ``verdict`` line, or the reason they cannot be
    compared."""
    if not matplotlib_paths or not pltrs_paths:
        return {"rms": None, "verdict": "no picture to compare"}
    if len(matplotlib_paths) != len(pltrs_paths):
        verdict = f"{len(matplotlib_paths)} figures against {len(pltrs_paths)}"
        return {"rms": None, "verdict": verdict}
    worst = 0.0
    for first_path, second_path in zip(matplotlib_paths, pltrs_paths):
        with Image.open(first_path) as first, Image.open(second_path) as second:
            if first.size != second.size:
                verdict = (f"sizes differ: {first.width}×{first.height} "
                           f"against {second.width}×{second.height}")
                return {"rms": None, "verdict": verdict}
            worst = max(worst, rms(first, second))
    return {"rms": worst, "verdict": verdict_of(worst)}


def verdict_of(value):
    """The table's words for the RMS *value* of two pictures."""
    if value == 0:
        return "the same picture"
    if value <= SAME_RMS:
        return f"the same picture (RMS {value:.3f})"
    return f"RMS {value:.2f}"


def combined(paths):
    """The pictures at *paths* side by side on white, as one RGBA image."""
    images = [Image.open(path).convert("RGBA") for path in paths]
    width = sum(image.width for image in images) + GAP * (len(images) - 1)
    height = max(image.height for image in images)
    canvas = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    left = 0
    for image in images:
        canvas.alpha_composite(image, (left, (height - image.height) // 2))
        left += image.width + GAP
    return canvas
