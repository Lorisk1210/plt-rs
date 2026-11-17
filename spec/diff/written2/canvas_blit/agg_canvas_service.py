# A small rendering service without pyplot: functions build Figures on FigureCanvasAgg and return PNG bytes and RGBA arrays.
from __future__ import annotations

import io
from dataclasses import dataclass

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure


@dataclass
class Chart:
    title: str
    values: list[float]


def render(chart: Chart, dpi: int = 72) -> tuple[bytes, np.ndarray]:
    fig = Figure(figsize=(3, 2), dpi=dpi)
    canvas = FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax.bar(range(len(chart.values)), chart.values, color="steelblue")
    ax.set_title(chart.title)
    canvas.draw()
    rgba = np.asarray(canvas.buffer_rgba()).copy()
    buf = io.BytesIO()
    canvas.print_png(buf)
    return buf.getvalue(), rgba


def main() -> None:
    charts = [Chart("a", [1, 3, 2]), Chart("b", [4, 1, 5, 2]), Chart("c", [0.5])]
    for chart in charts:
        png, rgba = render(chart)
        print(chart.title, png[:8], rgba.shape, rgba.dtype, int((rgba[..., :3] < 128).any(axis=2).sum()) > 0)
    png, rgba = render(charts[1], dpi=144)
    print(rgba.shape)
    with open("service.png", "wb") as f:
        f.write(png)


if __name__ == "__main__":
    main()
