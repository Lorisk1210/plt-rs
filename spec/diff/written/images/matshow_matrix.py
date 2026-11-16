# Confusion matrix with matshow, text annotations in each cell and tick labels on top.
import numpy as np
import matplotlib.pyplot as plt

classes = ["cat", "dog", "bird", "fish"]
cm = np.array([[45, 3, 2, 0], [4, 40, 1, 1], [1, 2, 38, 4], [0, 1, 3, 46]])
fig, ax = plt.subplots(figsize=(5, 5))
im = ax.matshow(cm, cmap="Blues")
for i in range(4):
    for j in range(4):
        ax.text(j, i, cm[i, j], ha="center", va="center", color="white" if cm[i, j] > 25 else "black")
ax.set_xticks(range(4), classes)
ax.set_yticks(range(4), classes)
ax.set_xlabel("predicted")
ax.set_ylabel("true")
ax.xaxis.set_label_position("top")
print(f"accuracy {np.trace(cm) / cm.sum():.3f}")
fig.savefig("confusion.png")
