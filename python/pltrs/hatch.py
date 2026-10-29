"""
Contains classes for generating hatch patterns (matplotlib's ``hatch``).

pltrs (Rust) generates the hatch paths (``pltrs_core::hatch``, through
``_native.path.hatch``); the validation of patterns is matplotlib's.
"""

from . import _api
from .path import Path


def _validate_hatch_pattern(hatch):
    valid_hatch_patterns = set(r'-+|/\xXoO.*')
    if hatch is not None:
        invalids = set(hatch).difference(valid_hatch_patterns)
        if invalids:
            valid = ''.join(sorted(valid_hatch_patterns))
            invalids = ''.join(sorted(invalids))
            _api.warn_deprecated(
                '3.4',
                removal='3.13',  # one release after custom hatches (#20690)
                message=f'hatch must consist of a string of "{valid}" or '
                        'None, but found the following invalid values '
                        f'"{invalids}". Passing invalid values is deprecated '
                        'since %(since)s and will become an error in %(removal)s.'
            )


def get_path(hatchpattern, density=6):
    """
    Given a hatch specifier, *hatchpattern*, generates Path to render
    the hatch in a unit square.  *density* is the number of lines per
    unit square.
    """
    return Path.hatch(hatchpattern, density)
