"""
The Qt backend drawing with Cairo, with the Qt 5 bindings only
(matplotlib's ``backend_qt5cairo``): `.backend_qtcairo`'s canvas, which
draws with pltrs (Rust)'s raster as pltrs has no Cairo.
"""

from .. import backends

backends._QT_FORCE_QT5_BINDING = True
from .backend_qtcairo import (  # noqa: F401, E402 # pylint: disable=W0611
    _BackendQTCairo, FigureCanvasQTCairo
)


@_BackendQTCairo.export
class _BackendQT5Cairo(_BackendQTCairo):
    pass
