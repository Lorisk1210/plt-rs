# Print repr/str of the objects plotting functions return (lists, containers, tuples) as users do in notebooks.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
returned = {
    "plot": ax.plot([1, 2]), "bar": ax.bar([1], [2]), "hist": ax.hist([1, 2, 2], bins=2), "pie": None,
    "errorbar": ax.errorbar([1], [1], yerr=0.1), "stem": ax.stem([1], [1]), "fill": ax.fill([0, 1, 1], [0, 0, 1]),
}
fig2, ax2 = plt.subplots()
returned["pie"] = ax2.pie([1, 2])
for k, v in returned.items():
    if isinstance(v, tuple):
        print(k, "tuple", [type(x).__name__ for x in v])
    else:
        print(k, type(v).__name__, str(v)[:60] if " at 0x" not in str(v) else "<object>")
fig.savefig("returns.png", dpi=40)
