# A typed ChartSpec dataclass and a builder that renders it to Figure objects without pyplot; specs varied and outputs compared.
from __future__ import annotations

import io
from dataclasses import dataclass, field, replace
from typing import Sequence

import numpy as np
from matplotlib.figure import Figure


@dataclass(frozen=True)
class Series:
    name: str
    x: Sequence[float]
    y: Sequence[float]
    style: str = "-"


@dataclass(frozen=True)
class ChartSpec:
    title: str
    series: tuple[Series, ...]
    size: tuple[float, float] = (4.0, 3.0)
    log_y: bool = False
    legend_loc: str = "best"
    extras: dict = field(default_factory=dict)


def render(spec: ChartSpec) -> Figure:
    fig = Figure(figsize=spec.size, dpi=60)
    ax = fig.add_subplot()
    for s in spec.series:
        ax.plot(s.x, s.y, s.style, label=s.name)
    ax.set_title(spec.title)
    if spec.log_y:
        ax.set_yscale("log")
    ax.legend(loc=spec.legend_loc)
    ax.set(**spec.extras)
    return fig


def main() -> None:
    x = np.linspace(1, 10, 20)
    base = ChartSpec("growth", (Series("lin", tuple(x), tuple(x)), Series("sq", tuple(x), tuple(x ** 2), "o--")))
    variants = [base, replace(base, log_y=True), replace(base, size=(6, 2), legend_loc="upper left"),
                replace(base, extras={"xlim": (2, 8), "ylabel": "value"})]
    for spec in variants:
        fig = render(spec)
        buf = io.BytesIO()
        fig.savefig(buf, format="png")
        ax = fig.axes[0]
        print(spec.log_y, spec.size, ax.get_yscale(), [round(v, 2) for v in ax.get_xlim()], ax.get_ylabel(),
              ax.get_legend()._loc, len(buf.getvalue()) > 1000)
    print(base == replace(base), replace(base, log_y=True) == base, base.extras)


if __name__ == "__main__":
    main()
