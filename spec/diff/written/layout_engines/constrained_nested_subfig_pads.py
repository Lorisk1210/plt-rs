# Constrained layout with subfigures and set_layout_engine padding for a poster panel.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 4), layout="constrained")
fig.get_layout_engine().set(w_pad=4 / 72, h_pad=4 / 72, wspace=0, hspace=0)
sf_left, sf_right = fig.subfigures(1, 2)
ax_l = sf_left.subplots()
ax_l.scatter(np.arange(20), np.arange(20) ** 0.5, c=np.arange(20), cmap="plasma")
ax_l.set_title("left")
axs_r = sf_right.subplots(2, 1)
axs_r[0].plot(np.random.default_rng(4).normal(size=50).cumsum())
axs_r[1].bar(range(4), [4, 3, 6, 1])
sf_right.suptitle("right side")
print(fig.get_layout_engine().get()["w_pad"])
fig.savefig("poster_panel.png")
