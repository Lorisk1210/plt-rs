# Build figures with the object API only (matplotlib.figure.Figure), as in a library or web app.
from matplotlib.figure import Figure

fig = Figure(figsize=(6, 3), dpi=80)
ax1, ax2 = fig.subplots(1, 2)
ax1.plot([1, 2, 3], [1, 4, 9])
ax2.bar(["a", "b"], [2, 3])
fig.suptitle("no pyplot")
fig.savefig("library.png")
print(type(fig.canvas).__name__, fig.get_size_inches().tolist())
sub = fig.add_subfigure(fig.add_gridspec(1, 1)[0])
print(type(sub).__name__)
fig.savefig("figure_without_pyplot_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
