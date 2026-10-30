"""
The SVG backend (matplotlib's ``backend_svg``): `FigureCanvasSVG`, whose
``print_svg`` and ``print_svgz`` write the figure as an SVG document,
plain or gzip-compressed. pltrs (Rust) writes the documents.

The checks of the metadata are matplotlib's (from matplotlib 3.11.2, under
the Matplotlib License).
"""

import datetime

import numpy as np

from .. import __version__
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase

#: The metadata keys an SVG document holds.
_METADATA_KEYS = frozenset([
    'Title', 'Coverage', 'Date', 'Description', 'Format', 'Identifier',
    'Language', 'Relation', 'Source', 'Type', 'Creator', 'Contributor',
    'Publisher', 'Rights', 'Keywords'])


def _check_is_str(info, key):
    if not isinstance(info, str):
        raise TypeError(f'Invalid type for {key} metadata. Expected str, not '
                        f'{type(info)}.')


def _check_is_iterable_of_str(infos, key):
    if np.iterable(infos):
        for info in infos:
            if not isinstance(info, str):
                raise TypeError(f'Invalid type for {key} metadata. Expected '
                                f'iterable of str, not {type(info)}.')
    else:
        raise TypeError(f'Invalid type for {key} metadata. Expected str or '
                        f'iterable of str, not {type(infos)}.')


def _date_text(date):
    """The ``Date`` metadata *date* as matplotlib writes it: ISO 8601,
    several dates joined by slashes."""
    if isinstance(date, str):
        return date
    if isinstance(date, (datetime.datetime, datetime.date)):
        return date.isoformat()
    if np.iterable(date):
        dates = []
        for d in date:
            if isinstance(d, str):
                dates.append(d)
            elif isinstance(d, (datetime.datetime, datetime.date)):
                dates.append(d.isoformat())
            else:
                raise TypeError(
                    f'Invalid type for Date metadata. '
                    f'Expected iterable of str, date, or datetime, '
                    f'not {type(d)}.')
        return '/'.join(dates)
    raise TypeError(f'Invalid type for Date metadata. '
                    f'Expected str, date, datetime, or iterable '
                    f'of the same, not {type(date)}.')


def _svg_metadata(metadata):
    """*metadata* checked as matplotlib's SVG writer checks it, with
    pltrs (Python) as default creator and the dates written out; the
    entries left to pltrs (Rust)'s defaults (format, type, date) are not
    added."""
    from .._native import backends as _backends
    metadata = {
        'Creator': f'{_backends.default_creator()}, matplotlib {__version__} API',
        **(metadata or {}),
    }
    unknown = [key for key in metadata if key not in _METADATA_KEYS]
    if unknown:
        raise ValueError('Unknown metadata key(s) passed to SVG writer: ' +
                         ','.join(unknown))
    for key, value in metadata.items():
        if value is None:
            continue
        if key == 'Date':
            metadata[key] = _date_text(value)
        elif key in ('Creator', 'Contributor', 'Publisher', 'Rights',
                     'Keywords'):
            if not isinstance(value, str):
                _check_is_iterable_of_str(value, key)
                metadata[key] = list(value)
        else:
            _check_is_str(value, key)
    return metadata


class FigureCanvasSVG(FigureCanvasBase):
    filetypes = {'svg': 'Scalable Vector Graphics',
                 'svgz': 'Scalable Vector Graphics'}

    fixed_dpi = 72

    def print_svg(self, filename, *, bbox_inches_restore=None, metadata=None,
                  facecolor=None, edgecolor=None):
        """
        Write the figure as an SVG document.

        Parameters
        ----------
        filename : str or path-like or file-like
            Output target; if a string, a file will be opened for writing.
        metadata : dict[str, Any], optional
            Metadata in the SVG file defined as key-value pairs of strings,
            datetimes, or lists of strings.
        """
        self._print_svg("svg", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def print_svgz(self, filename, *, bbox_inches_restore=None, metadata=None,
                   facecolor=None, edgecolor=None):
        """Write the figure as a gzip-compressed SVG document."""
        self._print_svg("svgz", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def _print_svg(self, fmt, filename, bbox_inches_restore, metadata,
                   facecolor, edgecolor):
        """Write the figure as *fmt*, ``svg`` or ``svgz``."""
        self._print_native(fmt, filename, metadata=_svg_metadata(metadata),
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    def get_default_filetype(self):
        return 'svg'

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()


FigureManagerSVG = FigureManagerBase


@_Backend.export
class _BackendSVG(_Backend):
    backend_version = __version__
    FigureCanvas = FigureCanvasSVG
