"""
A module for parsing and generating `fontconfig patterns`_ (matplotlib's
``_fontconfig_pattern``), by pltrs (Rust) (``pltrs_text::fontconfig``), where
matplotlib uses pyparsing.

.. _fontconfig patterns:
   https://www.freedesktop.org/software/fontconfig/fontconfig-user.html
"""

from functools import lru_cache

from ._native import font_manager as _font_manager


@lru_cache
def parse_fontconfig_pattern(pattern):
    """
    Parse a fontconfig *pattern* into a dict that can initialize a
    `.font_manager.FontProperties` object.
    """
    if not isinstance(pattern, str):
        # matplotlib's pyparsing parser expands the tabs of what it parses
        # first, failing on what has no ``expandtabs``.
        pattern.expandtabs()
    return {key: list(values)
            for key, values in _font_manager.parse_pattern(pattern)}


def generate_fontconfig_pattern(d):
    """Convert a `.FontProperties` to a fontconfig pattern string."""
    kvs = [(k, getattr(d, f"get_{k}")())
           for k in ["style", "variant", "weight", "stretch", "file", "size"]]
    # Families is given first without a leading keyword.  Other entries (which
    # are necessarily scalar) are given as key=value, skipping Nones.
    return (",".join(_font_manager.escape_family(f) for f in d.get_family())
            + "".join(f":{k}={_font_manager.escape_value(str(v))}"
                      for k, v in kvs if v is not None))
