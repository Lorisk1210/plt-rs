# Read transData, transAxes, transFigure, dpi_scale_trans and their inverses at fixed points after limits, sizes and dpi change.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(-1, 1)
pts = np.array([[0, -1], [5, 0], [10, 1]])
def show(tag):
    fig.canvas.draw()
    print(tag, np.round(ax.transData.transform(pts), 2).tolist())
    print(tag, np.round(ax.transAxes.transform([[0, 0], [1, 1]]), 2).tolist(), np.round(fig.transFigure.transform([1, 1]), 2).tolist())
    print(tag, np.round(fig.dpi_scale_trans.transform([1, 1]), 2).tolist(),
          np.round(ax.transData.inverted().transform([[200, 150]]), 4).tolist())
    print(tag, np.round((ax.transData + fig.transFigure.inverted()).transform(pts[1]), 4).tolist())


show("start")
ax.set_xlim(-10, 10)
show("xlim")
fig.set_size_inches(6, 3)
show("size")
fig.set_dpi(50)
show("dpi")
ax.set_yscale("symlog", linthresh=0.1)
show("symlog")
ax.set_position([0.2, 0.2, 0.5, 0.5])
show("position")
print(ax.transData.is_affine, ax.transAxes.is_affine, ax.transLimits.is_affine)
print(np.round(ax.transLimits.transform([[0, 0]]), 4).tolist(), np.round(ax.transScale.transform([[1, 1]]), 4).tolist())
