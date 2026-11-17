# axisartist Axes inside subplots with constrained layout and subplot_mosaic; positions with and without the layout engine.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist

for layout in (None, "constrained", "tight"):
    fig, axs = plt.subplots(2, 2, figsize=(5, 4), dpi=80, layout=layout, subplot_kw={"axes_class": axisartist.Axes})
    for k, ax in enumerate(axs.flat):
        ax.plot(np.arange(5), np.arange(5) ** (k + 1))
        ax.axis["left"].label.set_text(f"y{k}")
        ax.axis["bottom"].label.set_text(f"x{k}")
    fig.canvas.draw()
    print(layout, [[round(v, 3) for v in ax.get_position().bounds] for ax in axs.flat][:2])
    fig.savefig(f"aa_{layout}.png")
    plt.close(fig)
fig, d = plt.subplot_mosaic("AB;CC", figsize=(5, 4), dpi=80, layout="constrained",
                            per_subplot_kw={"A": {"axes_class": axisartist.Axes}})
d["A"].plot([0, 1], [0, 1])
d["B"].plot([0, 1], [1, 0])
fig.canvas.draw()
print({k: type(v).__name__ for k, v in d.items()})
print({k: [round(x, 3) for x in v.get_position().bounds] for k, v in d.items()})
fig.savefig("mosaic.png")
fig.set_layout_engine("none")
d["C"].set_title("after layout off")
fig.canvas.draw()
print([round(x, 3) for x in d["C"].get_position().bounds], type(fig.get_layout_engine()).__name__)
fig.savefig("mosaic_off.png")
