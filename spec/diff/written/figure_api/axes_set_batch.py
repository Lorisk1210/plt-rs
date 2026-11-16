# Configure axes with one Axes.set(...) call and inspect with get(); mixes of property names.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(np.arange(10), np.arange(10) ** 0.5)
ax.set(title="set() everything", xlabel="n", ylabel="sqrt n", xlim=(0, 9), ylim=(0, 3.2),
       xticks=[0, 3, 6, 9], yscale="linear", facecolor="#fafafa", aspect="auto")
print(ax.get_title(), ax.get_xlim(), ax.get_xticks().tolist())
plt.setp(ax.lines, color="k", linewidth=2)
print(plt.getp(ax.lines[0], "color"))
fig.savefig("set_batch.png")
fig.savefig("axes_set_batch_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
