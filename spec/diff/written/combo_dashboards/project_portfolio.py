# Project portfolio: Gantt bars coloured by status, budget burn lines, risk matrix scatter, and a status pie.
import numpy as np
import matplotlib.pyplot as plt

projects = [("alpha", 0, 6, "on track", 120, 2, 3), ("beta", 2, 5, "at risk", 80, 4, 4), ("gamma", 1, 9, "late", 200, 5, 2),
            ("delta", 5, 4, "on track", 60, 1, 2), ("eps", 6, 5, "at risk", 90, 3, 5)]
status_color = {"on track": "seagreen", "at risk": "orange", "late": "firebrick"}
fig, axs = plt.subplots(2, 2, figsize=(11, 8))
for i, (name, start, length, status, *_ ) in enumerate(projects):
    axs[0, 0].broken_barh([(start, length)], (i - 0.4, 0.8), color=status_color[status])
axs[0, 0].set_yticks(range(len(projects)), [p[0] for p in projects])
axs[0, 0].set_xlabel("month")
months = np.arange(12)
for name, start, length, status, budget, *_ in projects:
    spent = np.clip((months - start) / length, 0, 1.3) * budget
    axs[0, 1].plot(months, spent, label=name, color=status_color[status])
axs[0, 1].legend(fontsize=8)
for name, *_, impact, likelihood in projects:
    axs[1, 0].scatter(likelihood, impact, s=300, color=status_color[[p for p in projects if p[0] == name][0][3]])
    axs[1, 0].annotate(name, (likelihood, impact), ha="center", va="center", fontsize=8)
axs[1, 0].set(xlim=(0, 6), ylim=(0, 6), xlabel="likelihood", ylabel="impact")
counts = {s: sum(p[3] == s for p in projects) for s in status_color}
axs[1, 1].pie(counts.values(), labels=counts.keys(), colors=status_color.values(), autopct="%d%%")
fig.tight_layout()
print(counts, [c.get_facecolor()[0].round(3).tolist() for c in axs[0, 0].collections][:2])
fig.savefig("portfolio.png", dpi=60)
