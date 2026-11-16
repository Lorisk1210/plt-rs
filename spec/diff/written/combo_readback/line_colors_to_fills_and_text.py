# Read each line's colour back and reuse it for its confidence band, end label and a legend patch.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

x = np.linspace(0, 10, 80)
fig, ax = plt.subplots(figsize=(7, 4))
handles = []
for k in range(4):
    line, = ax.plot(x, np.sin(x + k) + k)
    c = line.get_color()
    ax.fill_between(x, np.sin(x + k) + k - 0.3, np.sin(x + k) + k + 0.3, color=c, alpha=0.2)
    ax.text(x[-1] + 0.1, np.sin(x[-1] + k) + k, f"k={k}", color=c, va="center")
    handles.append(Patch(color=c, alpha=0.5, label=f"band {k}"))
ax.legend(handles=handles, loc="upper left")
print([l.get_color() for l in ax.lines])
print([tuple(np.round(c.get_facecolor()[0], 3)) for c in ax.collections])
print([t.get_color() for t in ax.texts])
fig.savefig("colors_reused.png", dpi=50)
