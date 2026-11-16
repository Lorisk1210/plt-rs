# The plt.axis() family: 'equal', 'scaled', 'tight', 'square', explicit list, and reading it back.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 2 * np.pi, 100)
fig, axs = plt.subplots(2, 3, figsize=(10, 6))
opts = ["equal", "scaled", "tight", "square", "image", [-2, 2, -1, 1]]
for ax, opt in zip(axs.flat, opts):
    ax.plot(2 * np.cos(t), np.sin(t))
    result = ax.axis(opt)
    ax.set_title(str(opt))
    print(opt, [round(v, 3) for v in result])
fig.tight_layout()
fig.savefig("axis_options.png")
