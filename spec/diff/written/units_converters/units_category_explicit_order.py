# Force category order by plotting an invisible line first, then the real data in another order.
import matplotlib.pyplot as plt

order = ["low", "medium", "high", "critical"]
counts = {"high": 12, "low": 40, "critical": 3, "medium": 25}
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(order, [0] * len(order), alpha=0)
ax.bar(list(counts), list(counts.values()), color=["orange", "green", "red", "gold"])
print([t.get_text() for t in ax.get_xticklabels()])
ax.set_ylabel("tickets")
ax.set_title("severity in fixed order")
fig.savefig("severity.png")
fig.savefig("severity_small.png", dpi=40)
print(ax.get_xlim())
