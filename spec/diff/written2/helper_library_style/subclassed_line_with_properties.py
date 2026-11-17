# A Line2D subclass with an extra property (set_band/get_band) that setp, set() and ArtistInspector discover; drawn with a band.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import ArtistInspector
from matplotlib.lines import Line2D


class BandLine(Line2D):
    def __init__(self, *args, band=0.0, **kwargs):
        self._band = band
        super().__init__(*args, **kwargs)

    def set_band(self, band):
        """Set the half-width of the shaded band around the line."""
        self._band = float(band)
        self.stale = True

    def get_band(self):
        return self._band

    def draw(self, renderer):
        if self._band and self.axes is not None:
            x, y = self.get_data()
            poly = self.axes.fill_between(x, np.asarray(y) - self._band, np.asarray(y) + self._band, alpha=0.2,
                                          color=self.get_color())
            poly.draw(renderer)
            poly.remove()
        super().draw(renderer)


fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
x = np.linspace(0, 5, 30)
ln = BandLine(x, np.sin(x), band=0.2, color="C1")
ax.add_line(ln)
ax.autoscale_view()
fig.canvas.draw()
print(ln.get_band(), "band" in ArtistInspector(ln).get_setters(), ArtistInspector(ln).get_valid_values("band"))
plt.setp(ln, band=0.5)
print(ln.get_band(), plt.getp(ln, "band"))
ln.set(band=0.1, lw=3)
print(ln.get_band(), ln.get_linewidth(), "band" in ln.properties())
ln2 = BandLine([0, 1], [0, 1])
ln2.update_from(ln)
print(ln2.get_band(), ln2.get_linewidth())
fig.savefig("band.png")
print(len(ax.collections), len(ax.lines))
