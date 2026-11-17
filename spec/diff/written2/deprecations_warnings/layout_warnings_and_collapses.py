# Layout engines that cannot do their job: constrained layout with tiny figures, tight_layout with huge labels, warnings recorded.
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def run(label, build):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig = build()
        fig.canvas.draw()
        fig.savefig("layout.png")
    print(label, [str(w.message).splitlines()[0][:60] for w in caught])
    print(label, [[round(v, 3) for v in a.get_position().bounds] for a in fig.axes][:2])
    plt.close(fig)


def tiny_constrained():
    fig, axs = plt.subplots(3, 3, figsize=(0.8, 0.8), layout="constrained")
    for a in axs.flat:
        a.set_title("a long title")
    return fig


def huge_labels_tight():
    fig, ax = plt.subplots(figsize=(2, 2))
    ax.set_ylabel("y\n" * 12)
    ax.set_xlabel("an extremely long x label " * 3)
    fig.tight_layout()
    return fig


def colorbar_constrained():
    fig, ax = plt.subplots(figsize=(1, 1), layout="constrained")
    fig.colorbar(ax.imshow([[0, 1]]), ax=ax)
    return fig


def compressed_layout():
    fig, axs = plt.subplots(2, 2, figsize=(4, 2), layout="compressed")
    for a in axs.flat:
        a.imshow([[0, 1], [1, 0]])
    return fig


for name, fn in [("tiny", tiny_constrained), ("huge", huge_labels_tight), ("cbar", colorbar_constrained),
                 ("compressed", compressed_layout)]:
    run(name, fn)
