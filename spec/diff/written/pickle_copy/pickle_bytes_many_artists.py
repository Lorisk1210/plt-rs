# Pickle to bytes a figure with an image, colour bar, text, patches and a legend; compare after load.
import pickle
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.5))
im = a.imshow(np.arange(100).reshape(10, 10), cmap="magma")
fig.colorbar(im, ax=a)
b.bar(["x", "y", "z"], [3, 1, 2], label="bars")
b.add_patch(Rectangle((0.5, 2), 1, 0.5, fc="none", ec="r"))
b.text(0, 2.5, "note")
b.legend()
try:
    data = pickle.dumps(fig)
except TypeError as err:
    print("string categories do not pickle:", err)
    b.set_xticks(range(3), ["x", "y", "z"])
    b.cla()
    b.bar([0, 1, 2], [3, 1, 2], label="bars", tick_label=["x", "y", "z"])
    b.legend()
    data = pickle.dumps(fig)
fig2 = pickle.loads(data)
print(len(fig2.axes), [type(c).__name__ for c in fig2.axes[1].get_children()][:6])
print(fig2.axes[0].images[0].get_array().shape, fig2.get_size_inches())
fig2.savefig("unpickled.png")
