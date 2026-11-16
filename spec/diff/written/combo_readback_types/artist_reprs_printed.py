# Print reprs and str of common artists (users print them to see what plot returned).
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
objs = [fig, ax, ax.plot([0, 1], label="trend")[0], ax.bar([0], [1])[0], ax.text(0.5, 0.5, "note"), ax.xaxis,
        ax.imshow(np.zeros((2, 2))), ax.legend(), ax.scatter([0], [0]), ax.annotate("a", (0, 0)), ax.spines["left"],
        ax.get_xticklabels()[0], ax.fill_between([0, 1], [0, 1]), ax.axhline(0.5), ax.patch]
for o in objs:
    s = str(o)
    print(type(o).__name__, "|", "<default object repr>" if " at 0x" in s else s[:70])
fig.savefig("reprs.png", dpi=40)
print(len(objs), len(fig.axes))
