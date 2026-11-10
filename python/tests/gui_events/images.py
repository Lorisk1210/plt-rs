"""Pixels the harness keeps as files: snapshots, goldens, failures."""


def save_png(path, size, pixels, optimize=False):
    """Write *pixels* (RGB bytes, rows from the top) of an image of *size*
    ``(width, height)`` to *path* as PNG, its directory made if missing."""
    from PIL import Image
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.frombytes("RGB", size, pixels).save(path, optimize=optimize)
