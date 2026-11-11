"""
The data of matplotlib's built-in colormaps under the names of its ``_cm``
module, which programs import them from (``from matplotlib._cm import
_Set3_data``).

pltrs (Rust) holds the definitions (``pltrs::colors::colormaps``), and
`.cm` makes matplotlib's ``datad`` of them. This module gives ``datad`` and
each ``_<name>_data`` value it is made of when it is first read: the
segment data of a `.LinearSegmentedColormap`, or the colours of a colour
sequence (the ``'listed'`` entry of its ``datad`` item).
"""


def __getattr__(name):
    from . import cm
    if name == "datad":
        return cm.datad
    if name.startswith("_") and name.endswith("_data"):
        entry = cm.datad.get(name[1:-len("_data")])
        if entry is not None:
            listed = isinstance(entry, dict) and set(entry) == {"listed"}
            value = globals()[name] = entry["listed"] if listed else entry
            return value
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
