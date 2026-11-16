# Pickle a figure with annotations, an arrow, a figure legend and a suptitle; redraw after load.
import pickle
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([0, 1, 2], [0, 2, 1], label="trace")
ax.annotate("top", xy=(1, 2), xytext=(1.5, 2.3), arrowprops=dict(arrowstyle="->"))
fig.legend(loc="lower right")
fig.suptitle("annotated")
clone = pickle.loads(pickle.dumps(fig))
ann = [t for t in clone.axes[0].texts]
print(len(ann), ann[0].get_text(), clone._suptitle.get_text(), len(clone.legends))
clone.savefig("annotated_clone.png")
