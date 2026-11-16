# Training curves: train/val loss with best-epoch marker, accuracy on a twin axis, and learning rate on a log subplot.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(41)
epochs = np.arange(1, 61)
train = 2.0 * np.exp(-epochs / 12) + 0.15 + rng.normal(0, 0.02, 60)
val = 2.0 * np.exp(-epochs / 12) + 0.25 + 0.004 * np.clip(epochs - 30, 0, None) ** 1.5 / 5 + rng.normal(0, 0.03, 60)
acc = 1 - val / 3
lr = 1e-3 * 0.5 ** (epochs // 15)
best = int(np.argmin(val))
fig, (a, b) = plt.subplots(2, 1, figsize=(8, 6), sharex=True, gridspec_kw={"height_ratios": [3, 1]})
a.plot(epochs, train, label="train")
a.plot(epochs, val, label="val")
a.axvline(epochs[best], color="k", ls="--", lw=0.8)
a.annotate(f"best {val[best]:.3f} @ {epochs[best]}", (epochs[best], val[best]), xytext=(10, 30), textcoords="offset points",
           arrowprops=dict(arrowstyle="->"))
a2 = a.twinx()
a2.plot(epochs, acc, color="C2", alpha=0.6, label="val acc")
a.legend(loc="upper right")
b.semilogy(epochs, lr, drawstyle="steps-post")
b.set_ylabel("lr")
b.set_xlabel("epoch")
print(best + 1, round(float(val[best]), 4), sorted(set(lr.tolist())))
fig.savefig("curves.png", dpi=50)
