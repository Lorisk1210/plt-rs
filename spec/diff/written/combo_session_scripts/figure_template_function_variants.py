# A make_figure(template) function with presets (paper, slide, poster) controlling size, fonts and layout; used three times.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt

TEMPLATES = {"paper": ((3.5, 2.6), 8, "tight"), "slide": ((10, 5.6), 16, "constrained"), "poster": ((12, 9), 24, "constrained")}
def make_figure(template, nrows=1, ncols=1):
    size, font, layout = TEMPLATES[template]
    with mpl.rc_context({"font.size": font}):
        fig, axs = plt.subplots(nrows, ncols, figsize=size, layout=layout, squeeze=False)
        for ax in axs.flat:
            ax.plot(np.arange(10), np.arange(10) ** 0.5)
            ax.set_title(template)
        fig.canvas.draw()
        return fig, axs
for name in TEMPLATES:
    f, a = make_figure(name, ncols=2 if name != "paper" else 1)
    print(name, f.get_size_inches().tolist(), a[0, 0].title.get_fontsize(), type(f.get_layout_engine()).__name__)
    f.savefig(f"tmpl_{name}.png", dpi=30)
