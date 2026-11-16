# Read stackplot polygons' vertices to place labels at the vertical centre of each layer's widest point.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
ys = [np.full(10, 2.0), 1 + np.sin(x / 2) + 1, np.linspace(1, 3, 10)]
fig, ax = plt.subplots(figsize=(7, 4))
polys = ax.stackplot(x, ys, labels=["base", "wave", "ramp"])
bottom = np.zeros(10)
for poly, layer, name in zip(polys, ys, ["base", "wave", "ramp"]):
    k = int(np.argmax(layer))
    ax.text(x[k], bottom[k] + layer[k] / 2, name, ha="center", va="center", color="w", fontweight="bold")
    bottom += layer
verts = polys[1].get_paths()[0].vertices
print(verts.shape, [round(float(v), 3) for v in verts[:2, 1]], [t.get_text() for t in ax.texts])
fig.savefig("stack_labels.png", dpi=50)
