# A Normalize built with colors.make_norm_from_scale from a registered custom scale, used by imshow and its colorbar.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors, scale

LogitNorm = colors.make_norm_from_scale(scale.LogitScale)(colors.Normalize)
FuncSqrt = colors.make_norm_from_scale(scale.FuncScale, init=lambda functions, vmin=None, vmax=None, clip=False: None)(colors.Normalize)
data = np.linspace(0.01, 0.99, 25).reshape(5, 5)
fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3))
n1 = LogitNorm(0.01, 0.99)
im1 = a1.imshow(data, norm=n1)
cb1 = fig.colorbar(im1, ax=a1)
n2 = FuncSqrt((np.sqrt, np.square), vmin=0, vmax=1)
im2 = a2.imshow(data, norm=n2)
cb2 = fig.colorbar(im2, ax=a2)
fig.canvas.draw()
print(type(n1).__name__, np.round(n1([0.01, 0.5, 0.99]), 4).tolist(), np.round(n1.inverse([0.25, 0.75]), 4).tolist())
print(type(n2).__name__, np.round(n2([0, 0.25, 1]), 4).tolist())
print(cb1.ax.get_yscale(), [t.get_text() for t in cb1.ax.get_yticklabels()][:3])
print(cb2.ax.get_yscale(), [round(v, 3) for v in cb2.ax.get_ylim()])
print(type(n1._scale).__name__, n1._trf.__class__.__name__)
im3 = a1.imshow(data, norm="logit")
print(type(im3.norm).__name__, im3.norm.vmin)
for name in ("linear", "log", "symlog", "asinh", "logit"):
    im = a1.imshow(data + (name == "log") * 0.0, norm=name)
    fig.canvas.draw()
    print(name, type(im.norm).__name__, np.round(im.norm(np.array([0.1, 0.5])), 4).tolist())
fig.savefig("scale_norms.png")
