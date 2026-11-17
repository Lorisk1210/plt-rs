# sticky_edges from imshow, bar, hist, fill_between and stackplot, and how margins and use_sticky_edges change the limits.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(8, 5))
arts = [axs[0, 0].imshow(np.eye(3)), axs[0, 1].bar([0, 1], [1, 2]), axs[0, 2].hist([1, 2, 2, 3])[2],
        axs[1, 0].fill_between([0, 1, 2], [0, 1, 0]), axs[1, 1].stackplot([0, 1, 2], [1, 2, 1])[0],
        axs[1, 2].plot([0, 1], [0, 1])[0]]
for k, art in enumerate(arts):
    first = art[0] if hasattr(art, "__getitem__") and not isinstance(art, np.ndarray) else art
    print(k, type(first).__name__, first.sticky_edges.x[:2], first.sticky_edges.y[:2])
fig.canvas.draw()
print([tuple(round(v, 3) for v in a.get_ylim()) for a in axs.flat])
for a in axs.flat:
    a.margins(0.2)
fig.canvas.draw()
print([tuple(round(v, 3) for v in a.get_ylim()) for a in axs.flat])
for a in axs.flat:
    a.use_sticky_edges = False
    a.autoscale_view()
print([tuple(round(v, 3) for v in a.get_ylim()) for a in axs.flat])
ln = arts[-1]
ln.sticky_edges.y.append(1.5)
axs[1, 2].use_sticky_edges = True
axs[1, 2].margins(y=0.5)
axs[1, 2].autoscale_view()
print([round(v, 3) for v in axs[1, 2].get_ylim()], ln.sticky_edges)
fig.savefig("sticky.png")
