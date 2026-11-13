# The type stub of matplotlib.testing.widgets: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from typing import Any, Literal

from pltrs.axes import Axes
from pltrs.backend_bases import Event, MouseButton
from pltrs.widgets import AxesWidget, Widget

def get_ax() -> Axes: ...
def noop(*args: Any, **kwargs: Any) -> None: ...
def mock_event(
    ax: Axes,
    button: MouseButton | int | Literal["up", "down"] | None = ...,
    xdata: float = ...,
    ydata: float = ...,
    key: str | None = ...,
    step: int = ...,
) -> Event: ...
def do_event(
    tool: AxesWidget,
    etype: str,
    button: MouseButton | int | Literal["up", "down"] | None = ...,
    xdata: float = ...,
    ydata: float = ...,
    key: str | None = ...,
    step: int = ...,
) -> None: ...
def click_and_drag(
    tool: Widget,
    start: tuple[float, float],
    end: tuple[float, float],
    key: str | None = ...,
) -> None: ...
