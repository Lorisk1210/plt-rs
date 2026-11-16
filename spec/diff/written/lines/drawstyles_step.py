# Step-wise data: drawstyle steps-pre/mid/post compared with ax.step and where=.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(8)
y = np.array([1, 3, 2, 5, 4, 4, 6, 3])
fig, axs = plt.subplots(2, 2, figsize=(8, 5), sharex=True, sharey=True)
for ax, ds in zip(axs.flat, ["default", "steps-pre", "steps-mid", "steps-post"]):
    ax.plot(x, y, drawstyle=ds, marker="o")
    ax.set_title(ds)
fig.savefig("drawstyles.png")
fig2, ax2 = plt.subplots(figsize=(5, 3))
ax2.step(x, y, where="mid", label="step mid")
ax2.step(x, y + 1, where="post", label="step post")
ax2.legend()
print(ax2.lines[0].get_drawstyle(), ax2.lines[1].get_drawstyle())
fig2.savefig("step.png")
