# Take the data of a step plot (get_data, get_drawstyle) and rebuild it as bars aligned to the steps' edges.
import numpy as np
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(10, 3.5))
line, = a.step([0, 1, 2, 3, 4], [1, 3, 2, 4, 3], where="post")
x, y = line.get_data()
style = line.get_drawstyle()
b.bar(np.asarray(x)[:-1], np.asarray(y)[:-1], width=np.diff(x), align="edge", alpha=0.6)
b.step(x, y, where="post", color="k")
print(style, list(x), list(y), len(b.patches))
fig.savefig("step_bars.png", dpi=50)
