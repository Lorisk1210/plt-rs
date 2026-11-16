# A small custom Artist subclass (a crosshair) that draws two lines through the renderer.
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.lines import Line2D


class Crosshair(Artist):
    def __init__(self, x, y, size=0.5, **kw):
        super().__init__()
        self.h = Line2D([x - size, x + size], [y, y], **kw)
        self.v = Line2D([x, x], [y - size, y + size], **kw)

    def set_figure(self, fig):
        super().set_figure(fig)
        self.h.set_figure(fig)
        self.v.set_figure(fig)

    def draw(self, renderer):
        for line in (self.h, self.v):
            line.set_transform(self.axes.transData)
            line.draw(renderer)


fig, ax = plt.subplots(figsize=(4, 4))
ax.set_xlim(0, 4)
ax.set_ylim(0, 4)
for x, y in [(1, 1), (2, 3), (3, 1.5)]:
    ax.add_artist(Crosshair(x, y, color="red", lw=2))
print(len(ax.artists))
fig.savefig("crosshairs.png")
