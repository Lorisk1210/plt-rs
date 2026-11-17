# A user-defined Normalize subclass (piecewise-linear with midpoints) used by imshow, contourf and colorbar; inverse for ticks.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors


class PiecewiseNorm(colors.Normalize):
    def __init__(self, xs, ys, clip=False):
        super().__init__(min(xs), max(xs), clip)
        self.xs, self.ys = list(xs), list(ys)

    def __call__(self, value, clip=None):
        result, is_scalar = self.process_value(value)
        out = np.ma.masked_array(np.interp(result, self.xs, self.ys, left=-np.inf, right=np.inf), mask=np.ma.getmask(result))
        return out[0] if is_scalar else out

    def inverse(self, value):
        return np.interp(value, self.ys, self.xs)


norm = PiecewiseNorm([0, 1, 10, 100], [0, 0.5, 0.75, 1])
print(np.round(norm(np.array([0, 0.5, 1, 5, 10, 50, 100])), 4).tolist(), round(float(norm(1)), 4))
print(np.round(norm.inverse(np.array([0.25, 0.6, 0.9])), 4).tolist())
data = np.logspace(-1, 2, 40).reshape(5, 8)
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3))
im = a1.imshow(data, norm=norm, cmap="viridis")
cb = fig.colorbar(im, ax=a1)
cf = a2.contourf(data, levels=[0, 0.5, 1, 5, 10, 50, 100], norm=norm, cmap="viridis")
cb2 = fig.colorbar(cf, ax=a2)
fig.canvas.draw()
print([round(t, 3) for t in cb.get_ticks()], [round(v, 3) for v in cb.ax.get_ylim()], cb.ax.get_yscale())
print([round(t, 3) for t in cb2.get_ticks()], np.round(cf.get_facecolor()[:2], 3).tolist())
print(type(im.norm).__name__, im.norm.vmin, im.norm.vmax)
fig.savefig("piecewise.png")
