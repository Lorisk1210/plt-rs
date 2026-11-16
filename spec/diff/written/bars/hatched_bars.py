# Black-and-white friendly bars distinguished by hatch patterns for a printed report.
import numpy as np
import matplotlib.pyplot as plt

labels = ["2021", "2022", "2023"]
a = [5, 7, 6]
b = [3, 4, 6]
c = [2, 3, 2]
x = np.arange(3)
fig, ax = plt.subplots(figsize=(5.5, 4))
ax.bar(x - 0.25, a, 0.25, label="product A", color="white", edgecolor="black", hatch="//")
ax.bar(x, b, 0.25, label="product B", color="white", edgecolor="black", hatch="xx")
ax.bar(x + 0.25, c, 0.25, label="product C", color="lightgray", edgecolor="black", hatch="..")
ax.set_xticks(x, labels)
ax.legend()
plt.rcParams["hatch.linewidth"] = 0.5
fig.savefig("hatched.png")
print([p.get_hatch() for p in ax.patches[:3]])
