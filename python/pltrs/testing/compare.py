"""
Utilities for comparing image results (matplotlib's ``testing.compare``).

plt-rs compares PNG images by the RMS of their difference, as matplotlib
does; it has no converter for PDF, PostScript or SVG, as matplotlib has
none without Ghostscript and Inkscape.
"""

import os

from .._native import testing as _testing

__all__ = ['comparable_formats', 'compare_images']

#: The converters to PNG of the formats other than PNG, by extension.
#: matplotlib fills it from the Ghostscript and Inkscape it finds; plt-rs
#: converts no format, so there are none.
converter = {}


def make_test_filename(fname, purpose):
    """
    Make a new filename by inserting *purpose* before the file's extension.
    """
    base, ext = os.path.splitext(fname)
    return f'{base}-{purpose}{ext}'


def comparable_formats():
    """
    Return the list of file formats that `.compare_images` can compare
    on this system.
    """
    return ['png']


def compare_images(expected, actual, tol, in_decorator=False):
    """
    Compare two "image" files checking differences within a tolerance.

    Returns
    -------
    None or dict or str
        Return *None* if the images are equal within the given tolerance.

        If the images differ, the return value depends on  *in_decorator*.
        If *in_decorator* is true, a dict with the entries *rms*,
        *expected*, *actual*, *diff* and *tol* is returned; otherwise, a
        human-readable multi-line string representation of this
        information.
    """
    actual = os.fspath(actual)
    if not os.path.exists(actual):
        raise Exception(f"Output image {actual} does not exist.")
    if os.stat(actual).st_size == 0:
        raise Exception(f"Output image file {actual} is empty.")
    expected = os.fspath(expected)
    if not os.path.exists(expected):
        raise OSError(f'Baseline image {expected!r} does not exist.')
    suffix = os.path.splitext(expected)[1]
    if suffix != '.png':
        # matplotlib converts other formats to PNG with Ghostscript or
        # Inkscape when it has them, and skips the test when it has not.
        import pytest
        pytest.skip(f"Don't know how to convert {suffix} files to png")

    rms, diff = _testing.compare_images(expected, actual, float(tol))
    if diff is None:
        return None

    results = dict(rms=rms, expected=str(expected),
                   actual=str(actual), diff=str(diff), tol=tol)
    if not in_decorator:
        template = ['Error: Image files did not match.',
                    'RMS Value: {rms}',
                    'Expected:  \n    {expected}',
                    'Actual:    \n    {actual}',
                    'Difference:\n    {diff}',
                    'Tolerance: \n    {tol}', ]
        results = '\n  '.join([line.format(**results) for line in template])
    return results
