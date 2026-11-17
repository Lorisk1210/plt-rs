# set_xticks with labels and minor, set_ticklabels with lists/formatter strings/functions, and the locators/formatters they install.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3), dpi=60)
ax.set_xlim(0, 10)
ax.set_xticks([0, 2.5, 5, 7.5, 10], labels=["zero", "q1", "half", "q3", "end"], fontsize=7, color="b")
ax.set_xticks([1, 3, 6], minor=True)
ax.set_yticks([0, 0.5, 1])
ax.yaxis.set_ticklabels(["lo", "mid", "hi"])
fig.canvas.draw()
print(type(ax.xaxis.get_major_locator()).__name__, type(ax.xaxis.get_major_formatter()).__name__)
print([t.get_text() for t in ax.get_xticklabels()], ax.get_xticklabels()[0].get_color())
print(ax.get_xticks(minor=True).tolist(), [t.get_text() for t in ax.get_xticklabels(minor=True)])
print([t.get_text() for t in ax.get_yticklabels()], type(ax.yaxis.get_major_formatter()).__name__)
ax.xaxis.set_major_formatter("{x:.1f} s")
fig.canvas.draw()
print(type(ax.xaxis.get_major_formatter()).__name__, [t.get_text() for t in ax.get_xticklabels()])
ax.xaxis.set_major_formatter(lambda x, pos: f"#{pos}:{x:g}")
fig.canvas.draw()
print(type(ax.xaxis.get_major_formatter()).__name__, [t.get_text() for t in ax.get_xticklabels()])
ax.set_xticks([])
fig.canvas.draw()
print(ax.get_xticks().tolist(), len(ax.get_xticklabels()))
try:
    ax.set_xticks([1, 2], labels=["only one"])
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    ax.set_yticklabels(["a"], fontsize="giant")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(ax.get_xticklabels(which="both") == [], len(ax.get_yticklabels(which="both")))
fig.savefig("set_ticks.png")
