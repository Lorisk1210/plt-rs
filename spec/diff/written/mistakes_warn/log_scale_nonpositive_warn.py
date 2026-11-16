# Setting log limits that include zero or negatives: warnings, ignored limits, and the result.
import warnings
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([1, 10, 100], [1, 10, 100])
ax.set_yscale("log")
with warnings.catch_warnings(record=True) as w:
    ax.set_ylim(0, 1000)
    ax.set_xlim(-10, 200)
for item in w:
    print(item.category.__name__, item.message)
print([round(v, 4) for v in ax.get_ylim()], [round(v, 4) for v in ax.get_xlim()])
fig.savefig("log_limits.png")
