"""
The colours of a legend's labels as matplotlib's legend gives them.

pltrs (Rust) colours the labels of a legend (``legend.labelcolor``, the
colour of each entry's handle); matplotlib's legend makes each label with
:rc:`text.color` and then sets the colour its *labelcolor* picks, as that
colour comes (a handle's colour as its getter gives it, a row of
``to_rgba_array``). `remember` keeps those values, which the labels'
``get_color`` reads back while pltrs (Rust) draws them in that colour.
"""

import itertools

import numpy as np

from . import _val_or_rc, _values, cbook, colors, rcParams

#: The getters of the handles' colour a *labelcolor* name picks from, in
#: order (matplotlib's ``color_getters``).
_COLOR_GETTERS = {
    'linecolor': ['get_markerfacecolor', 'get_facecolor', 'get_markeredgecolor',
                  'get_edgecolor', 'get_color'],
    'markerfacecolor': ['get_markerfacecolor', 'get_facecolor'],
    'mfc': ['get_markerfacecolor', 'get_facecolor'],
    'markeredgecolor': ['get_markeredgecolor', 'get_edgecolor'],
    'mec': ['get_markeredgecolor', 'get_edgecolor'],
}


def remember(legend, labelcolor):
    """Keep the colour matplotlib's legend gives each label of *legend*
    for *labelcolor* (`Legend` given it, else None) as the colour its
    ``get_color`` reads back."""
    texts = legend.texts
    for text in texts:
        _values.remember(text, "color", rcParams["text.color"])
    labelcolor = _val_or_rc(_val_or_rc(labelcolor, 'legend.labelcolor'), 'text.color')
    if isinstance(labelcolor, str) and labelcolor in _COLOR_GETTERS:
        for handle, text in zip(legend.legend_handles, texts):
            color = _picked(handle, _COLOR_GETTERS[labelcolor])
            if color is not None:
                _values.remember(text, "color", color)
    elif cbook._str_equal(labelcolor, 'none'):
        for text in texts:
            _values.remember(text, "color", labelcolor)
    elif np.iterable(labelcolor):
        for text, color in zip(texts, itertools.cycle(colors.to_rgba_array(labelcolor))):
            _values.remember(text, "color", color)


def _picked(handle, getter_names):
    """The colour of *handle* the first of *getter_names* it has gives, as
    matplotlib's legend takes it (the first of an array of one colour), or
    None when it takes none (a handle mapping values to colours, colours
    of several kinds, no colour)."""
    try:
        if handle.get_array() is not None:
            return None
    except AttributeError:
        pass
    for getter_name in getter_names:
        try:
            color = getattr(handle, getter_name)()
        except AttributeError:
            continue
        if isinstance(color, np.ndarray):
            if color.size == 0:
                continue
            if color.shape[0] == 1 or np.isclose(color, color[0]).all():
                return color[0]
            return None
        if cbook._str_lower_equal(color, 'none') or colors.to_rgba(color)[3] == 0:
            continue
        return color
    return None
