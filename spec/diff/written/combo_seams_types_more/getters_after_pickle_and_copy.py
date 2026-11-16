# Read the same getters before and after a pickle round trip and print both, for lines, bars, texts and an image.
import pickle
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
line, = ax.plot(np.array([0.0, 1, 2]), np.array([1.0, 0, 1]), "o--", lw=2, alpha=0.5, color=(0.2, 0.4, 0.6))
bar = ax.bar([1], [0.5], color="C2", hatch="//")[0]
txt = ax.text(0.5, 0.5, "note", rotation=30, fontsize=11)
im = ax.imshow(np.eye(2), extent=(0, 2, 0, 1), alpha=0.3)
def probe(f):
    a = f.axes[0]
    l, b, t, i = a.lines[0], a.patches[0], a.texts[0], a.images[0]
    return [l.get_color(), l.get_linewidth(), l.get_alpha(), l.get_linestyle(), b.get_facecolor(), b.get_hatch(),
            t.get_rotation(), t.get_fontsize(), i.get_extent(), i.get_alpha()]
before = probe(fig)
after = probe(pickle.loads(pickle.dumps(fig)))
for b_, a_ in zip(before, after):
    print(repr(b_), "|", repr(a_))
fig.savefig("pickled_getters.png", dpi=50)
