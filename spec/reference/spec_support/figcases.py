"""Figure cases: scripted figures rendered to PNG plus structural facts."""

from __future__ import annotations

import dataclasses
import io
import pathlib
from typing import Any, Callable

from .golden import GOLDEN_ROOT, to_jsonable

FIGURE_ROOT = GOLDEN_ROOT / "figures"

_CASES: dict[str, "FigureCase"] = {}


@dataclasses.dataclass(frozen=True)
class FigureCase:
    """A named figure recipe.

    ``build`` receives a fresh ``Figure`` (default size 6.4x4.8in at 100 dpi)
    and populates it. ``figsize``/``dpi`` override the canvas when the case
    needs it. ``rc`` holds rcParams applied while the case is built and drawn.
    """

    name: str
    build: Callable[[Any], None]
    figsize: tuple[float, float] = (6.4, 4.8)
    dpi: float = 100.0
    rc: dict[str, Any] = dataclasses.field(default_factory=dict)

    @property
    def png_path(self) -> pathlib.Path:
        return FIGURE_ROOT / f"{self.name}.png"

    @property
    def facts_path(self) -> pathlib.Path:
        return FIGURE_ROOT / f"{self.name}.json"

    def render(self) -> tuple[bytes, dict[str, Any]]:
        """Build the figure with matplotlib; return PNG bytes and facts."""
        import matplotlib as mpl
        import matplotlib.pyplot as plt

        with mpl.rc_context(self.rc):
            fig = plt.figure(figsize=self.figsize, dpi=self.dpi)
            try:
                self.build(fig)
                buffer = io.BytesIO()
                fig.savefig(buffer, format="png", dpi=self.dpi, metadata={"Software": None})
                facts = figure_facts(fig)
            finally:
                plt.close(fig)
        return buffer.getvalue(), facts


def case(name: str, **options: Any) -> Callable[[Callable[[Any], None]], Callable[[Any], None]]:
    """Register a figure case under ``<group>/<name>``."""
    if name.count("/") != 1:
        raise ValueError(f"case name {name!r} must look like 'group/name'")

    def register(build: Callable[[Any], None]) -> Callable[[Any], None]:
        if name in _CASES:
            raise ValueError(f"figure case {name!r} registered twice")
        _CASES[name] = FigureCase(name, build, **options)
        return build

    return register


def all_cases() -> list[FigureCase]:
    return [_CASES[k] for k in sorted(_CASES)]


def _rounded(value: Any, digits: int = 6) -> Any:
    if isinstance(value, float):
        return round(value, digits)
    if isinstance(value, list):
        return [_rounded(v, digits) for v in value]
    if isinstance(value, dict):
        return {k: _rounded(v, digits) for k, v in value.items()}
    return value


def _drawn_ticks(axis: Any) -> list[list[Any]]:
    """Major ticks actually drawn: ``[location, label]`` pairs."""
    ticks = axis._update_ticks()  # the ticks matplotlib draws this frame
    major = set(map(id, axis.get_major_ticks()))
    return [
        [tick.get_loc(), tick.label1.get_text() or tick.label2.get_text()]
        for tick in ticks
        if id(tick) in major
    ]


def _axes_facts(ax: Any) -> dict[str, Any]:
    facts: dict[str, Any] = {
        "kind": ax.name,
        "position": list(ax.get_position().bounds),
        "title": ax.get_title(),
    }
    if ax.name in ("rectilinear", "polar", "aitoff", "hammer", "lambert", "mollweide"):
        facts["xlim"] = list(ax.get_xlim())
        facts["ylim"] = list(ax.get_ylim())
        facts["xscale"] = ax.get_xscale()
        facts["yscale"] = ax.get_yscale()
        facts["xlabel"] = ax.get_xlabel()
        facts["ylabel"] = ax.get_ylabel()
        facts["xticks"] = _drawn_ticks(ax.xaxis)
        facts["yticks"] = _drawn_ticks(ax.yaxis)
    if ax.name == "3d":
        facts["xlim"] = list(ax.get_xlim())
        facts["ylim"] = list(ax.get_ylim())
        facts["zlim"] = list(ax.get_zlim())
    legend = ax.get_legend()
    if legend is not None:
        facts["legend"] = [t.get_text() for t in legend.get_texts()]
    return facts


def figure_facts(fig: Any) -> dict[str, Any]:
    """Structural facts about a drawn figure, used for precise comparisons."""
    width, height = fig.canvas.get_width_height()
    facts = {
        "size_px": [width, height],
        "suptitle": fig._suptitle.get_text() if fig._suptitle is not None else "",
        "axes": [_axes_facts(ax) for ax in fig.axes],
    }
    return _rounded(to_jsonable(facts))
