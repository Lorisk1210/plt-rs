# Change size and dpi between draws and read the canvas size each time.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
ax.plot(np.arange(5))
for size, dpi in [((3, 2), 50), ((6, 2), 50), ((6, 2), 100)]:
    fig.set_size_inches(size)
    fig.set_dpi(dpi)
    fig.canvas.draw()
    print(fig.canvas.get_width_height(), np.asarray(fig.canvas.buffer_rgba()).shape)
fig.savefig("resized.png")
ax.set_title("resized")
fig.canvas.draw()
print(fig.bbox.bounds)
