# axes_grid1.Grid (AxesGrid) with share_x/share_y combinations and label modes; tick label visibility on each axes.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import Grid

for share_all, label_mode in [(False, "L"), (True, "1"), (True, "all"), (False, "keep")]:
    fig = plt.figure(figsize=(5, 4), dpi=60)
    grid = Grid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.25, share_all=share_all, label_mode=label_mode)
    for k, ax in enumerate(grid):
        ax.plot([0, k + 1], [0, (k + 1) * 10])
    fig.canvas.draw()
    vis = []
    for ax in grid:
        xl = [t.get_visible() for t in ax.get_xticklabels()]
        yl = [t.get_visible() for t in ax.get_yticklabels()]
        vis.append((any(xl), any(yl)))
    print(share_all, label_mode, vis)
    print([tuple(round(v, 2) for v in ax.get_xlim()) for ax in grid])
    print([round(v, 3) for v in grid[3].get_position().bounds])
    fig.savefig(f"grid_{label_mode}.png")
    plt.close(fig)
fig = plt.figure(figsize=(5, 4))
grid = Grid(fig, 111, nrows_ncols=(3, 2), axes_pad=0.1, direction="column", n_axes=5)
print(len(grid), grid.get_geometry(), [grid.axes_all.index(a) for a in grid.axes_column[0]])
print([[grid.axes_all.index(a) for a in row] for row in grid.axes_row])
print(type(grid.get_divider()).__name__, grid.get_axes_pad(), grid.get_aspect())
grid[0].set_xlim(0, 100)
print([tuple(round(v, 1) for v in ax.get_xlim()) for ax in grid])
grid.set_label_mode("L")
fig.canvas.draw()
print([any(t.get_visible() for t in ax.get_yticklabels()) for ax in grid])
