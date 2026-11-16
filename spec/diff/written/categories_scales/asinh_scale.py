# Signal with tiny and huge values both signs on the 'asinh' scale (newer alternative to symlog).
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(-10, 10, 500)
y = np.sinh(t) * 0.01
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(t, y)
ax.set_yscale("asinh", linear_width=0.5)
ax.grid(True, alpha=0.3)
ax.set_title("asinh scale")
fig.canvas.draw()
print(ax.get_yscale())
print([t.get_text() for t in ax.get_yticklabels()])
fig.savefig("asinh.png")
