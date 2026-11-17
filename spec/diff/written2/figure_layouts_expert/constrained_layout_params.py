# Constrained layout parameters (w_pad, h_pad, wspace, hspace, rect) and set_constrained_layout_pads; resulting positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for kw in [{}, dict(w_pad=0.2, h_pad=0.2), dict(wspace=0.3, hspace=0.3), dict(rect=(0.1, 0.1, 0.8, 0.8))]:
    fig, axs = plt.subplots(2, 2, figsize=(5, 4), dpi=72, layout="constrained")
    fig.get_layout_engine().set(**kw)
    for k, ax in enumerate(axs.flat):
        ax.set_title(f"t{k}")
        ax.set_xlabel("x")
    fig.canvas.draw()
    print(sorted(kw), [[round(v, 3) for v in ax.get_position().bounds] for ax in axs.flat][:2])
    print(sorted(fig.get_layout_engine().get().items()))
    plt.close(fig)
fig, axs = plt.subplots(1, 2, figsize=(5, 3), layout="constrained")
fig.colorbar(axs[0].imshow([[0, 1]]), ax=axs[0], shrink=0.6)
fig.colorbar(axs[1].imshow([[0, 1]]), ax=axs, location="bottom")
fig.suptitle("suptitle")
fig.canvas.draw()
print([[round(v, 3) for v in a.get_position().bounds] for a in fig.axes])
axs[0].set_in_layout(False)
fig.canvas.draw()
print([round(v, 3) for v in axs[1].get_position().bounds])
fig.get_layout_engine().set(h_pad=0.3)
fig.canvas.draw()
print([round(v, 3) for v in axs[1].get_position().bounds], fig.get_layout_engine().get()["h_pad"])
fig.savefig("constrained.png")
print(len(fig.axes))
plt.close("all")
