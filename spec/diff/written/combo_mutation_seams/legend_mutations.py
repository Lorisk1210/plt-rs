# Mutate a legend after creation: title, loc, ncols, frame, texts' colours and handles' widths; read back.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
for k in range(4):
    ax.plot(np.arange(5) * (k + 1), label=f"s{k}")
leg = ax.legend()
leg.set_title("series")
leg.set_loc("lower right")
leg.set_ncols(2)
leg.get_frame().set_facecolor("lightyellow")
leg.get_frame().set_edgecolor("k")
for text, handle in zip(leg.get_texts(), leg.get_lines()):
    text.set_color(handle.get_color())
    handle.set_linewidth(4)
fig.canvas.draw()
print(leg.get_title().get_text(), leg._loc, leg._ncols, leg.get_frame().get_facecolor())
print([t.get_color() for t in leg.get_texts()], [h.get_linewidth() for h in leg.get_lines()])
fig.savefig("legend_mutations.png", dpi=50)
