# Lollipop chart (stems plus markers) of sorted values, horizontal, with value text.
import numpy as np
import matplotlib.pyplot as plt

names = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf"]
vals = np.array([23, 45, 12, 67, 34, 51, 29])
order = np.argsort(vals)
fig, ax = plt.subplots(figsize=(6, 4))
ax.hlines(np.arange(7), 0, vals[order], color="skyblue", lw=2)
ax.plot(vals[order], np.arange(7), "o", color="steelblue", ms=8)
for i, v in enumerate(vals[order]):
    ax.text(v + 1.5, i, str(v), va="center", fontsize=8)
ax.set_yticks(range(7), [names[i] for i in order])
ax.set_xlim(0, 75)
print(vals[order].tolist())
fig.savefig("lollipop.png")
