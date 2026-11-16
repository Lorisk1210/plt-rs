# Two series with partially overlapping categories: the axis keeps first-seen order.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(["mon", "tue", "wed", "thu"], [3, 4, 2, 5], "o-", label="week 1")
ax.plot(["wed", "thu", "fri", "sat"], [6, 3, 4, 1], "s--", label="week 2")
ax.bar(["sun"], [2], color="gray", label="sunday")
ax.legend()
labels = [t.get_text() for t in ax.get_xticklabels()]
print(labels, list(ax.get_xticks()))
mapping = ax.xaxis.units._mapping
print(dict(mapping))
fig.savefig("week.png")
ax.set_ylabel("count")
ax.set_title("Days of two weeks")
