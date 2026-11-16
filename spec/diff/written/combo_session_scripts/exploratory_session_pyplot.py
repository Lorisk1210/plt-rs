# An exploratory session in pyplot style: several figures, gcf/gca juggling, reuse by number, final saves of all open figures.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(240)
df = pd.DataFrame({"a": rng.normal(size=200), "b": rng.normal(size=200), "g": rng.choice(list("xyz"), 200)})
plt.figure(1, figsize=(5, 4)); plt.hist(df["a"], bins=20); plt.title("a")
plt.figure(2, figsize=(5, 4)); plt.scatter(df["a"], df["b"], c=df["g"].map({"x": 0, "y": 1, "z": 2}), cmap="tab10", vmin=0, vmax=9)
plt.figure(1); plt.axvline(df["a"].mean(), color="r")
plt.figure(3, figsize=(5, 4)); df.groupby("g")["b"].mean().plot.bar(); plt.title("means")
plt.figure(2); plt.title("scatter"); plt.colorbar()
for num in plt.get_fignums():
    f = plt.figure(num)
    f.savefig(f"session_{num}.png", dpi=40)
    print(num, [ax.get_title() for ax in f.axes], len(f.axes))
