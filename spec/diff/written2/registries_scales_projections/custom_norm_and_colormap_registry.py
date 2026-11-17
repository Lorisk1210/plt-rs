# A Normalize subclass with an inverse (piecewise log) and a colormap registered with mpl.colormaps; used by image and colorbar.
import numpy as np
import matplotlib as mpl
mpl.use("agg")
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize, ListedColormap


class SymLogishNorm(Normalize):
    def __init__(self, vmin=None, vmax=None, linthresh=1.0, clip=False):
        super().__init__(vmin, vmax, clip)
        self.linthresh = linthresh

    def _fwd(self, v):
        v = np.asarray(v, dtype=float)
        return np.sign(v) * np.log1p(np.abs(v) / self.linthresh)

    def __call__(self, value, clip=None):
        result, is_scalar = self.process_value(value)
        self.autoscale_None(result)
        lo, hi = self._fwd(self.vmin), self._fwd(self.vmax)
        out = (self._fwd(result) - lo) / (hi - lo)
        return out[0] if is_scalar else out

    def inverse(self, value):
        lo, hi = self._fwd(self.vmin), self._fwd(self.vmax)
        f = np.asarray(value) * (hi - lo) + lo
        return np.sign(f) * np.expm1(np.abs(f)) * self.linthresh


steps = ListedColormap(["#08306b", "#2171b5", "#6baed6", "#fee391", "#fd8d3c", "#a50f15"], name="usr2_steps")
mpl.colormaps.register(steps)
z = np.linspace(-100, 100, 400).reshape(20, 20)
norm = SymLogishNorm(vmin=-100, vmax=100, linthresh=2)
fig, ax = plt.subplots(figsize=(4, 3.5))
im = ax.imshow(z, norm=norm, cmap="usr2_steps")
cb = fig.colorbar(im, ax=ax)
fig.canvas.draw()
print(np.round(norm([-100, -2, 0, 2, 100]), 6).tolist(), np.round(norm.inverse([0, 0.5, 1]), 6).tolist())
print(mpl.colormaps["usr2_steps"].N, "usr2_steps" in mpl.colormaps, im.cmap.name)
print([t.get_text() for t in cb.ax.get_yticklabels()])
fig.savefig("symlogish.png")
mpl.colormaps.unregister("usr2_steps")
print("usr2_steps" in mpl.colormaps)
