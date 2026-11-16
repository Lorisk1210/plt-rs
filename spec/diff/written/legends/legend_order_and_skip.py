# Reorder legend entries and hide one with a label starting with underscore, as SO answers suggest.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([1, 2, 3], [1, 2, 3], label="c: third")
ax.plot([1, 2, 3], [2, 3, 4], label="a: first")
ax.plot([1, 2, 3], [3, 4, 5], label="_hidden helper")
ax.plot([1, 2, 3], [4, 5, 6], label="b: second")
handles, labels = ax.get_legend_handles_labels()
print(labels)
order = sorted(range(len(labels)), key=lambda i: labels[i])
ax.legend([handles[i] for i in order], [labels[i] for i in order], loc="upper left")
fig.savefig("ordered.png")
ax.legend(reverse=True, loc="lower right")
fig.savefig("reversed.png")
print([t.get_text() for t in ax.get_legend().get_texts()])
