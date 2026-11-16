# tight_layout on axes it cannot handle (add_axes + subplots) warns; users still save the result.
import warnings
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 4))
ax1 = fig.add_subplot(211)
ax1.plot([1, 2, 3])
ax2 = fig.add_axes([0.6, 0.15, 0.3, 0.25])
ax2.plot([3, 2, 1])
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    fig.tight_layout()
for w in caught:
    print(w.category.__name__, str(w.message)[:80])
fig.savefig("mixed.png")
fig.tight_layout()
