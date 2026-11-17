# Check renderer.points_to_pixels and an Artist's measured line width across dpi values and backends' renderers.
import io
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist


class Probe(Artist):
    def __init__(self):
        super().__init__()
        self.seen = []

    def draw(self, renderer):
        self.seen.append((type(renderer).__name__, renderer.points_to_pixels(1.0), renderer.points_to_pixels(10)))


fig, ax = plt.subplots(figsize=(2, 2))
probe = Probe()
fig.add_artist(probe)
for dpi in (50, 72, 100, 150.5):
    fig.savefig(io.BytesIO(), format="png", dpi=dpi)
for fmt in ("svg", "pdf", "ps"):
    fig.savefig(io.BytesIO(), format=fmt)
for name, one, ten in probe.seen:
    print(name, repr(one), repr(ten))
r = fig.canvas.get_renderer()
print(type(r).__name__, r.width, r.height, r.dpi)
fig.set_dpi(120)
fig.canvas.draw()
r2 = fig.canvas.get_renderer()
print(r2 is r, r2.dpi, repr(r2.points_to_pixels(1.5)))
print(len(probe.seen))
