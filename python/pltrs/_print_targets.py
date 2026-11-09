"""
What a figure printed in a built-in format is written to, as matplotlib's
writer of the format takes it: a path, or a file object.

matplotlib's writers differ: Pillow (PNG, JPEG, TIFF, WebP) also opens a
path given as bytes, and writes JPEG through the descriptor of a file
that has one, so that a file opened as text gets the bytes too; the text
formats (SVG, PGF and PostScript) write their text into a text file; the
others write bytes through `cbook.to_filehandle`, which takes paths and
objects with a ``seek``, and PostScript takes paths and objects with a
``write``. A target the writer cannot take raises its `ValueError`
before the figure is drawn; a text file given bytes raises its own
`TypeError`.
"""

import io
import os

from . import cbook

#: The formats matplotlib writes through Pillow.
_PILLOW = frozenset({"png", "jpeg", "tiff", "webp"})

#: The text formats, and the encoding matplotlib writes each in.
_TEXT_ENCODINGS = {"svg": "utf-8", "pgf": "utf-8", "ps": "latin-1", "eps": "latin-1"}


def check(fmt, target):
    """Raise matplotlib's `ValueError` when its writer of *fmt* cannot
    write to *target*."""
    if fmt in _PILLOW or isinstance(target, str):
        return
    if fmt in ("ps", "eps"):
        if not (isinstance(target, os.PathLike) or cbook.is_writable_file_like(target)):
            raise ValueError("outfile must be a path or a file-like object")
    elif isinstance(target, os.PathLike):
        if not isinstance(os.fspath(target), str):
            raise ValueError("fname must be a PathLike or file handle")
    elif not hasattr(target, "seek"):
        raise ValueError("fname must be a PathLike or file handle")


def write(data, target, fmt=None):
    """Write the file *data*, of the format *fmt* (bytes written as they
    are where None), to *target* (checked by `check`)."""
    if isinstance(target, (str, bytes, os.PathLike)):
        with open(target, "wb") as file:
            file.write(data)
    elif fmt in _TEXT_ENCODINGS and cbook.file_requires_unicode(target):
        target.write(data.decode(_TEXT_ENCODINGS[fmt]))
    elif fmt == "jpeg" and cbook.file_requires_unicode(target):
        _write_to_descriptor(data, target)
    else:
        if fmt == "tiff":
            # Pillow's TIFF writer asks the file where it stands first.
            target.tell  # noqa: B018
        # matplotlib writes raw pixels as the renderer's buffer.
        target.write(memoryview(data) if fmt == "raw" else data)


def _write_to_descriptor(data, file):
    """Write *data* through the descriptor of the text *file*, as Pillow
    does, or (without one) to the file itself, which refuses bytes."""
    try:
        descriptor = file.fileno()
    except (AttributeError, io.UnsupportedOperation):
        file.write(data)
        return
    file.flush()
    view = memoryview(data)
    while view:
        view = view[os.write(descriptor, view):]
