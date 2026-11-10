"""The 2-D figures the interaction programs drive (``interaction.py``):
one of each kind of Axes a user pans, zooms and points at.

Every scene is a list of statements leaving ``fig`` (the root figure),
``ax`` (the Axes the program drives) and ``axs`` (all of the figure's
Axes, in a fixed order) drawn once, so that their positions are those of
the layout.
"""

from __future__ import annotations

#: The statements making each scene.
SCENES: dict[str, list[str]] = {
    "plain": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y, 'o-', label='sin')",
        "axs = [ax]",
    ],
    "shared": [
        "fig, (ax, ax2) = plt.subplots(1, 2, sharex=True, sharey=True, figsize=(5, 3))",
        "ax.plot(x, y)",
        "ax2.plot(x, values / 5)",
        "axs = [ax, ax2]",
    ],
    "shared_columns": [
        "fig, (ax, ax2) = plt.subplots(2, 1, sharex=True, figsize=(4, 4))",
        "ax.plot(x, y)",
        "ax2.plot(x, values)",
        "axs = [ax, ax2]",
    ],
    "twinx": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax2 = ax.twinx()",
        "ax2.plot(x, values, 'r')",
        "axs = [ax, ax2]",
    ],
    "twiny": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax2 = ax.twiny()",
        "ax2.plot(values, y, 'g')",
        "axs = [ax, ax2]",
    ],
    "log": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.loglog(np.logspace(-1, 3, 9), np.logspace(0, 2, 9), 's-')",
        "axs = [ax]",
    ],
    "symlog": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(np.linspace(-50, 50, 11), np.linspace(-20, 80, 11) ** 3)",
        "ax.set_yscale('symlog', linthresh=10)",
        "ax.set_xscale('symlog', linthresh=1, linscale=0.5)",
        "axs = [ax]",
    ],
    "logit": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(np.linspace(0.01, 0.99, 9), np.linspace(0.1, 0.9, 9), 'o')",
        "ax.set_xscale('logit')",
        "ax.set_yscale('logit')",
        "axs = [ax]",
    ],
    "dates": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "days = np.arange(np.datetime64('2024-01-01'), np.datetime64('2024-01-08'))",
        "ax.plot(days, values)",
        "axs = [ax]",
    ],
    "categorical": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.bar(['apple', 'pear', 'plum', 'fig'], [3, 1, 4, 2])",
        "axs = [ax]",
    ],
    "polar": [
        "fig = plt.figure(figsize=(4, 4))",
        "ax = fig.add_subplot(projection='polar')",
        "ax.plot(x, values)",
        "axs = [ax]",
    ],
    "image": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "im = ax.imshow(z, extent=(0, 6, 0, 5))",
        "cb = fig.colorbar(im)",
        "axs = [ax, cb.ax]",
    ],
    "aspect": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax.set_aspect('equal', adjustable='datalim')",
        "axs = [ax]",
    ],
    "aspect_box": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax.set_aspect(2)",
        "axs = [ax]",
    ],
    "inverted": [
        "fig, ax = plt.subplots(figsize=(4, 3))",
        "ax.plot(x, y)",
        "ax.invert_xaxis()",
        "ax.set_ylim(1.5, -1.5)",
        "axs = [ax]",
    ],
    "subfigures": [
        "fig = plt.figure(figsize=(6, 3))",
        "left, right = fig.subfigures(1, 2, wspace=0.1)",
        "ax = left.subplots()",
        "ax2 = right.subplots()",
        "ax.plot(x, y)",
        "ax2.plot(x, values)",
        "axs = [ax, ax2]",
    ],
    "constrained": [
        "fig, grid = plt.subplots(2, 2, layout='constrained', figsize=(5, 4))",
        "ax = grid[0, 0]",
        "grid[1, 1].plot(x, values)",
        "ax.plot(x, y)",
        "ax.set_title('constrained')",
        "axs = list(grid.flat)",
    ],
    "tight": [
        "fig, (ax, ax2) = plt.subplots(1, 2, figsize=(5, 3))",
        "ax.plot(x, y)",
        "ax2.scatter(x, values)",
        "ax.set_ylabel('label')",
        "fig.tight_layout()",
        "axs = [ax, ax2]",
    ],
    "inset": [
        "fig, outer = plt.subplots(figsize=(4, 3))",
        "outer.plot(x, values)",
        "ax = outer.inset_axes([0.55, 0.5, 0.4, 0.4])",
        "ax.plot(x, y)",
        "axs = [outer, ax]",
    ],
    "axes_grid1": [
        "from mpl_toolkits.axes_grid1 import host_subplot",
        "fig = plt.figure(figsize=(4, 3))",
        "ax = host_subplot(111, figure=fig)",
        "ax.plot(x, y)",
        "ax2 = ax.twinx()",
        "ax2.plot(x, values)",
        "axs = [ax, ax2]",
    ],
    "axisartist": [
        "from mpl_toolkits import axisartist",
        "fig = plt.figure(figsize=(4, 3))",
        "ax = fig.add_subplot(axes_class=axisartist.Axes)",
        "ax.plot(x, y)",
        "axs = [ax]",
    ],
}

#: The scenes the key bindings and the events programs run on (each
#: toolbar program runs on every scene).
KEY_SCENES = ("plain", "log", "symlog", "logit", "dates", "categorical", "polar", "image", "twinx", "subfigures",
              "axisartist")
EVENT_SCENES = ("plain", "shared", "twinx", "polar", "image", "subfigures", "inset", "constrained", "dates",
                "axes_grid1")


def scene(name: str) -> list[str]:
    """The statements making scene ``name``, drawn once."""
    return [*SCENES[name], "fig.canvas.draw()"]
