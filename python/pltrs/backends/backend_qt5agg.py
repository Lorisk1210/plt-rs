"""
Render to qt from agg, with the Qt 5 bindings only (matplotlib's
``backend_qt5agg``, from matplotlib 3.11.2, under the Matplotlib License,
kept as it is).
"""
from .. import backends

backends._QT_FORCE_QT5_BINDING = True
from .backend_qtagg import (    # noqa: F401, E402 # pylint: disable=W0611
    _BackendQTAgg, FigureCanvasQTAgg, FigureManagerQT, NavigationToolbar2QT,
    FigureCanvasAgg, FigureCanvasQT)


@_BackendQTAgg.export
class _BackendQT5Agg(_BackendQTAgg):
    pass
