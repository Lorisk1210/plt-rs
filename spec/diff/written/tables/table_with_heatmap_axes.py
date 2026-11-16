# Heat map with its exact values listed in a table on the right (two axes in a gridspec).
import numpy as np
import matplotlib.pyplot as plt

vals = np.round(np.random.default_rng(21).random((4, 4)), 2)
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3), gridspec_kw={"width_ratios": [1, 1.2]})
a.imshow(vals, cmap="magma", vmin=0, vmax=1)
a.set_xticks(range(4), list("ABCD"))
a.set_yticks(range(4), list("wxyz"))
b.axis("off")
b.table(cellText=vals.astype(str), rowLabels=list("wxyz"), colLabels=list("ABCD"), loc="center")
fig.savefig("heat_table.png")
print(vals.sum().round(2))
