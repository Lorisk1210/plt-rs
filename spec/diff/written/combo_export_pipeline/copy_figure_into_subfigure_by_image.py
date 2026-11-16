# Compose a poster: render three independent figures to arrays and place them into subfigures of one big figure.
import numpy as np
import matplotlib.pyplot as plt

def render(builder):
    f, ax = plt.subplots(figsize=(3, 2.4), dpi=60)
    builder(ax)
    f.canvas.draw()
    img = np.asarray(f.canvas.buffer_rgba()).copy()
    plt.close(f)
    return img

panels = [render(lambda ax: ax.pie([1, 2, 3])), render(lambda ax: ax.bar(range(4), [4, 2, 3, 1])),
          render(lambda ax: ax.plot(np.random.default_rng(141).normal(size=40).cumsum()))]
poster = plt.figure(figsize=(10, 3.5), layout="constrained")
for sf, img, title in zip(poster.subfigures(1, 3), panels, ["share", "counts", "trend"]):
    ax = sf.subplots()
    ax.imshow(img)
    ax.axis("off")
    sf.suptitle(title)
print([p.shape for p in panels])
poster.savefig("poster.png", dpi=50)
