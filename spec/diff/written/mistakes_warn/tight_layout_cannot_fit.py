# Huge tick labels in a tiny figure: tight_layout warns it cannot make axes large enough.
import warnings
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, axs = plt.subplots(3, 3, figsize=(2, 2))
for ax in axs.flat:
    ax.plot([1, 2])
    ax.set_title("a very long title indeed", fontsize=12)
    ax.set_ylabel("long label", fontsize=12)
with warnings.catch_warnings(record=True) as w:
    fig.tight_layout()
print([str(x.message)[:70] for x in w])
fig.savefig("cramped.png")
