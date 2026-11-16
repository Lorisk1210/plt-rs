# Legend with tuple handles (marker over line, two patches side by side) via HandlerTuple, alongside a colour bar.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.legend_handler import HandlerTuple
from matplotlib.patches import Patch

fig, ax = plt.subplots(figsize=(7, 4.5))
x = np.linspace(0, 10, 50)
l, = ax.plot(x, np.sin(x), "C0")
m, = ax.plot(x[::5], np.sin(x[::5]), "C0o")
p1, p2 = Patch(color="C1"), Patch(color="C2")
sc = ax.scatter(x, np.cos(x), c=x, cmap="viridis", s=10)
fig.colorbar(sc, ax=ax, label="x")
ax.legend([(l, m), (p1, p2)], ["sin with samples", "two groups"], handler_map={tuple: HandlerTuple(ndivide=None, pad=0.3)})
print([type(h).__name__ for h in ax.get_legend().legend_handles])
fig.savefig("tuple_legend.png", dpi=50)
