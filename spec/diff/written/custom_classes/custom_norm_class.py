# A Normalize subclass mapping values through a piecewise-linear midpoint (classic SO recipe).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize


class MidpointNormalize(Normalize):
    def __init__(self, vmin=None, vmax=None, midpoint=None, clip=False):
        self.midpoint = midpoint
        super().__init__(vmin, vmax, clip)

    def __call__(self, value, clip=None):
        x, y = [self.vmin, self.midpoint, self.vmax], [0, 0.5, 1]
        return np.ma.masked_array(np.interp(value, x, y))


data = np.linspace(-1, 4, 100).reshape(10, 10)
fig, ax = plt.subplots(figsize=(4.5, 4))
im = ax.imshow(data, cmap="RdBu_r", norm=MidpointNormalize(vmin=-1, vmax=4, midpoint=0))
fig.colorbar(im)
print(round(float(im.norm(0.0)), 3), round(float(im.norm(4.0)), 3))
fig.savefig("midpoint.png")
