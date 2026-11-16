# Debugging a plot with prints at each stage: limits, autoscale, margins, sticky edges, and the final layout.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
print(1, ax.get_xlim(), ax.get_autoscale_on(), ax.margins())
ax.bar(range(5), [3, 1, 4, 1, 5])
print(2, [round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
ax.plot(range(5), [2, 2, 2, 2, 2], "r-")
ax.margins(x=0.2)
print(3, [round(v, 3) for v in ax.get_xlim()], ax.margins())
ax.set_ylim(top=6)
ax.autoscale(axis="x", tight=True)
print(4, [round(v, 3) for v in ax.get_xlim()], ax.get_ylim(), ax.get_autoscalex_on(), ax.get_autoscaley_on())
fig.tight_layout()
print(5, [round(v, 3) for v in ax.get_position().bounds])
fig.savefig("debug.png", dpi=50)
