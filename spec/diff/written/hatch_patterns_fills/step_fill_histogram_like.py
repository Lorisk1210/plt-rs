# fill_between with step='mid' and 'post' to shade under step plots correctly.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(8)
y = np.array([1, 3, 2, 4, 3, 5, 2, 1])
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3))
a.step(x, y, where="mid", color="k")
a.fill_between(x, y, step="mid", alpha=0.4)
a.set_title("mid")
b.step(x, y, where="post", color="k")
b.fill_between(x, y, step="post", alpha=0.4, hatch="//")
b.set_title("post")
print(len(a.collections), len(b.collections))
fig.savefig("step_fill.png")
