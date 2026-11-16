# set_xticklabels without set_xticks (FixedFormatter with auto locator) and more labels than ticks.
import warnings
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, ax = plt.subplots(figsize=(5, 3))
ax.plot(range(10))
with warnings.catch_warnings(record=True) as w:
    ax.set_xticklabels(["a", "b", "c", "d", "e", "f", "g"])
print([type(x.message).__name__ for x in w], [str(x.message)[:60] for x in w])
try:
    ax.set_xticks([0, 5], ["zero", "five", "extra"])
except ValueError as err:
    print("ValueError:", err)
fig.savefig("ticklabels.png")
