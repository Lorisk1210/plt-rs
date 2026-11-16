# Drop shadow for a line using ScaledTranslation offset transform in points.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.transforms as transforms

x = np.linspace(0, 2 * np.pi, 100)
fig, ax = plt.subplots(figsize=(6, 3.5))
line, = ax.plot(x, np.sin(x), lw=3, color="C0")
dx, dy = 3 / 72, -3 / 72
offset = transforms.ScaledTranslation(dx, dy, fig.dpi_scale_trans)
shadow_transform = ax.transData + offset
ax.plot(x, np.sin(x), lw=3, color="gray", alpha=0.5, transform=shadow_transform, zorder=line.get_zorder() - 0.5)
ax.set_title("line with shadow")
print(np.round(offset.get_matrix(), 4).tolist())
fig.savefig("shadow.png")
