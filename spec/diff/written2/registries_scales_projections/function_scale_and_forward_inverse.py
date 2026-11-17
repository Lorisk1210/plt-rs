# FuncScale and FuncScaleLog with forward/inverse functions, secondary axes built on them, and tick values read back.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import scale as mscale


def forward(x):
    return np.arcsinh(np.asarray(x, dtype=float) / 3)


def inverse(y):
    return 3 * np.sinh(np.asarray(y, dtype=float))


fig, axs = plt.subplots(1, 3, figsize=(10, 3))
x = np.linspace(-50, 50, 201)
axs[0].plot(x, x)
axs[0].set_yscale("function", functions=(forward, inverse))
axs[1].plot(np.logspace(0, 4, 50), np.logspace(0, 4, 50))
axs[1].set_xscale("functionlog", functions=(np.sqrt, np.square))
sec = axs[2].secondary_xaxis("top", functions=(forward, inverse))
axs[2].plot(x, np.sin(x / 5))
fig.canvas.draw()
for ax in axs[:2]:
    print(ax.get_xscale(), ax.get_yscale(), type(ax.yaxis._scale).__name__, type(ax.xaxis._scale).__name__)
print(np.round(axs[0].get_yticks(), 4).tolist())
print([t.get_text() for t in axs[1].get_xticklabels()])
print(np.round(sec.get_xticks(), 4).tolist(), type(sec).__name__)
s = mscale.FuncScale(axs[0].yaxis, (forward, inverse))
print(np.round(s.get_transform().transform([0, 3, 30]), 6).tolist(), np.round(s.get_transform().inverted().transform([1.0]), 6).tolist())
print(type(mscale.scale_factory("log", axs[0].xaxis)).__name__, mscale.LogScale(axs[0].xaxis, base=2).base)
fig.savefig("funcscale.png")
