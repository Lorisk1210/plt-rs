# A Dashboard class keeping its figure, axes and artists, updating them with new data instead of replotting; artist identity kept.
import numpy as np
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure


class Dashboard:
    def __init__(self, n_channels: int = 3) -> None:
        self.fig = Figure(figsize=(5, 4), dpi=60)
        self.axes = self.fig.subplots(n_channels, 1, sharex=True)
        self.lines = [ax.plot([], [], lw=1)[0] for ax in self.axes]
        self.texts = [ax.text(0.98, 0.9, "", transform=ax.transAxes, ha="right") for ax in self.axes]
        self.updates = 0

    def update(self, t: np.ndarray, values: np.ndarray) -> None:
        for ln, txt, ax, v in zip(self.lines, self.texts, self.axes, values):
            ln.set_data(t, v)
            txt.set_text(f"max {v.max():.2f}")
            ax.relim()
            ax.autoscale_view()
        self.updates += 1

    def snapshot(self) -> bytes:
        import io
        buf = io.BytesIO()
        self.fig.savefig(buf, format="png")
        return buf.getvalue()


dash = Dashboard()
first_lines = list(dash.lines)
t = np.linspace(0, 1, 50)
for step in range(4):
    values = np.array([np.sin(2 * np.pi * (k + 1) * t + step) * (step + 1) for k in range(3)])
    dash.update(t, values)
    png = dash.snapshot()
    print(step, [tuple(round(v, 2) for v in ax.get_ylim()) for ax in dash.axes], [txt.get_text() for txt in dash.texts])
print(dash.updates, all(a is b for a, b in zip(dash.lines, first_lines)), len(png) > 1000, len(dash.fig.axes))
print([len(ax.lines) for ax in dash.axes], dash.axes[0].get_shared_x_axes().joined(dash.axes[0], dash.axes[2]))
