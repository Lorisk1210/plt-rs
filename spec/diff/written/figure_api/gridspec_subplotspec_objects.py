# Work with SubplotSpec objects directly: subgridspec, get_position, is_first_row, set_subplotspec.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(6, 4))
gs = fig.add_gridspec(2, 2)
ax = fig.add_subplot(gs[0, 0])
spec = ax.get_subplotspec()
print(spec.is_first_row(), spec.is_last_col(), spec.get_geometry())
inner = gs[1, :].subgridspec(1, 3)
for i in range(3):
    fig.add_subplot(inner[0, i]).plot([0, i])
ax.set_subplotspec(gs[0, 1])
fig.canvas.draw()
print([round(v, 3) for v in ax.get_position().bounds])
fig.savefig("specs.png")
