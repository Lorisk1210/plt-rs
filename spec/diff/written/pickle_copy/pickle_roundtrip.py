# Pickle a finished figure to disk, load it later and add a line before saving again.
import pickle
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
x = np.linspace(0, 5, 50)
ax.plot(x, np.exp(-x), label="decay")
ax.set_title("stored figure")
ax.legend()
with open("figure.pickle", "wb") as f:
    pickle.dump(fig, f)
plt.close(fig)
with open("figure.pickle", "rb") as f:
    fig2 = pickle.load(f)
ax2 = fig2.axes[0]
ax2.plot(x, np.exp(-2 * x), label="faster")
ax2.legend()
print(ax2.get_title(), [l.get_label() for l in ax2.lines])
fig2.savefig("restored.png")
