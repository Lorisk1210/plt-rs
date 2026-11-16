# Email A/B: funnel per variant as grouped horizontal bars with rates, open-time histogram, and click heat by hour.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(207)
stages = ["sent", "opened", "clicked", "bought"]
a = np.array([10000, 2400, 480, 96]); b = np.array([10000, 2900, 640, 120])
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
y = np.arange(4)
axs[0].barh(y - 0.2, a, 0.4, label="A"); axs[0].barh(y + 0.2, b, 0.4, label="B")
for k in range(1, 4):
    axs[0].text(a[k] + 200, k - 0.2, f"{a[k] / a[k - 1]:.0%}", va="center", fontsize=7)
    axs[0].text(b[k] + 200, k + 0.2, f"{b[k] / b[k - 1]:.0%}", va="center", fontsize=7)
axs[0].set_yticks(y, stages); axs[0].invert_yaxis(); axs[0].set_xscale("log"); axs[0].legend()
axs[1].hist([rng.exponential(3, 2400), rng.exponential(2.5, 2900)], bins=30, label=["A", "B"], histtype="step")
axs[1].set_xlabel("hours to open"); axs[1].legend()
heat = rng.poisson(5, (2, 24)) + np.array([[0], [2]])
im = axs[2].imshow(heat, aspect="auto", cmap="Greens")
axs[2].set_yticks([0, 1], ["A", "B"])
fig.colorbar(im, ax=axs[2])
fig.tight_layout()
print((a[1:] / a[:-1]).round(3).tolist(), (b[1:] / b[:-1]).round(3).tolist())
fig.savefig("email_ab.png", dpi=50)
