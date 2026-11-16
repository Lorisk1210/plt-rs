# Survey report: diverging Likert bars centred on neutral, a response-count table and a net-score dot plot.
import numpy as np
import matplotlib.pyplot as plt

questions = ["easy to learn", "fast", "good docs", "would recommend", "fair price"]
counts = np.array([[5, 10, 15, 40, 30], [12, 18, 20, 30, 20], [20, 25, 25, 20, 10], [3, 7, 10, 35, 45], [25, 30, 20, 15, 10]])
pct = counts / counts.sum(axis=1, keepdims=True) * 100
colors = ["#ca0020", "#f4a582", "#dddddd", "#92c5de", "#0571b0"]
fig = plt.figure(figsize=(12, 6))
gs = fig.add_gridspec(2, 2, width_ratios=[2, 1])
ax = fig.add_subplot(gs[:, 0])
left = -(pct[:, 0] + pct[:, 1] + pct[:, 2] / 2)
for k in range(5):
    ax.barh(questions, pct[:, k], left=left, color=colors[k], label=["SD", "D", "N", "A", "SA"][k])
    left = left + pct[:, k]
ax.axvline(0, color="k", lw=0.8)
ax.legend(ncols=5, loc="lower center", bbox_to_anchor=(0.5, 1.0))
ax.invert_yaxis()
tab = fig.add_subplot(gs[0, 1])
tab.axis("off")
tab.table(cellText=counts, rowLabels=[q[:10] for q in questions], colLabels=["SD", "D", "N", "A", "SA"], loc="center").set_fontsize(8)
net = pct[:, 3] + pct[:, 4] - pct[:, 0] - pct[:, 1]
dot = fig.add_subplot(gs[1, 1])
dot.scatter(net, range(5), color=np.where(net > 0, "C0", "C3"))
dot.set_yticks(range(5), questions)
dot.invert_yaxis()
dot.axvline(0, color="k", lw=0.5)
fig.tight_layout()
print(np.round(net, 2).tolist())
fig.savefig("likert.png", dpi=50)
