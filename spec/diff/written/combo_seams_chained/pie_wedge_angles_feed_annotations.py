# Use pie wedges' theta1/theta2/center/r to place external labels, connector lines and a matching bar legend.
import numpy as np
import matplotlib.pyplot as plt

vals = [35, 25, 20, 12, 8]
names = ["rent", "food", "travel", "fun", "misc"]
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4.5))
wedges, _ = a.pie(vals, startangle=90, wedgeprops=dict(width=0.45))
info = []
for w, n in zip(wedges, names):
    mid = np.radians((w.theta1 + w.theta2) / 2)
    cx, cy = w.center
    x, y = cx + w.r * np.cos(mid), cy + w.r * np.sin(mid)
    a.annotate(n, (x, y), xytext=(1.35 * np.cos(mid), 1.35 * np.sin(mid)), ha="center", arrowprops=dict(arrowstyle="-"))
    info.append((n, round(w.theta1, 2), round(w.theta2, 2), w.r))
b.barh(names, vals, color=[w.get_facecolor() for w in wedges])
b.invert_yaxis()
print(info)
fig.savefig("pie_labels.png", dpi=50)
