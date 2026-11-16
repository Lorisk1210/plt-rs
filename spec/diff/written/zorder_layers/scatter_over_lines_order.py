# Scatter points drawn behind lines by default (collections 1 vs lines 2) and fixing it with zorder.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 1, 10)
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.5))
for ax, z in [(a, None), (b, 5)]:
    ax.plot(x, x, color="k", lw=6)
    kw = {} if z is None else {"zorder": z}
    ax.scatter(x, x, s=120, color="orange", **kw)
    ax.set_title(f"scatter zorder={ax.collections[0].get_zorder()}")
print(a.lines[0].get_zorder(), a.collections[0].get_zorder(), b.collections[0].get_zorder())
fig.savefig("order.png")
