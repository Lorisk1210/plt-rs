# Bookkeeping across many subplots: subplot2grid, add_subplot with 3-digit codes, get_geometry, label_outer and sharing groups.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(6, 6))
a = plt.subplot2grid((3, 3), (0, 0), colspan=2)
b = plt.subplot2grid((3, 3), (0, 2), rowspan=2)
c = plt.subplot2grid((3, 3), (1, 0), colspan=2, sharex=a)
d = fig.add_subplot(337)
e = fig.add_subplot(3, 3, (8, 9))
for ax in (a, b, c, d, e):
    ss = ax.get_subplotspec()
    print(ss.get_geometry(), ss.rowspan, ss.colspan)
print(c.get_shared_x_axes().joined(a, c), len(a.get_shared_x_axes().get_siblings(a)))
fig2, axs = plt.subplots(3, 3, sharex="col", sharey="row")
for ax in axs.flat:
    ax.label_outer()
fig2.canvas.draw()
print([[any(t.get_visible() for t in ax.get_xticklabels()) for ax in row] for row in axs])
print([[any(t.get_visible() for t in ax.get_yticklabels()) for ax in row] for row in axs])
print(axs[0, 0].get_shared_y_axes().joined(axs[0, 0], axs[0, 2]), axs[0, 0].get_shared_x_axes().joined(axs[0, 0], axs[2, 0]))
fig3, axs3 = plt.subplots(2, 2, squeeze=False, sharex=True)
axs3[0, 0].set_xlim(0, 100)
print([ax.get_xlim() for ax in axs3.flat])
print(len(plt.get_fignums()), [len(f.axes) for f in map(plt.figure, plt.get_fignums())])
fig4 = plt.figure()
gs = fig4.add_gridspec(2, 2)
top = fig4.add_subplot(gs[0, :])
bl = fig4.add_subplot(gs[1, 0])
print(top.get_subplotspec().get_topmost_subplotspec() is top.get_subplotspec(), gs.nrows, gs.ncols, len(fig4.axes))
plt.close("all")
