# A custom AbstractPathEffect drawing a blurred-looking halo (stacked strokes) plus withTickedStroke and SimpleLineShadow.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.patheffects as pe


class Halo(pe.AbstractPathEffect):
    def __init__(self, color="gold", layers=4, offset=(0, 0)):
        super().__init__(offset)
        self.color = color
        self.layers = layers
        self.calls = 0

    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        self.calls += 1
        for k in range(self.layers, 0, -1):
            gc0 = renderer.new_gc()
            gc0.copy_properties(gc)
            gc0.set_foreground(self.color)
            gc0.set_alpha(0.15)
            gc0.set_linewidth(gc.get_linewidth() + 2 * k)
            renderer.draw_path(gc0, tpath, affine + self._offset_transform(renderer), None)
            gc0.restore()


halo = Halo()
fig, ax = plt.subplots(figsize=(5, 3))
x = np.linspace(0, 6, 100)
l1, = ax.plot(x, np.sin(x), lw=2, path_effects=[halo, pe.Normal()])
l2, = ax.plot(x, np.cos(x), lw=2, path_effects=[pe.withTickedStroke(angle=120, spacing=8, length=0.6)])
l3, = ax.plot(x, 0.5 * np.sin(2 * x), lw=2, path_effects=[pe.SimpleLineShadow(offset=(2, -2)), pe.Normal()])
t = ax.text(1, 0.6, "halo text", fontsize=14, path_effects=[pe.withStroke(linewidth=4, foreground="w")])
fig.savefig("effects.png")
print(halo.calls, [type(e).__name__ for e in l1.get_path_effects()])
fig.savefig("effects.pdf", metadata={"CreationDate": None})
print(halo.calls, [type(e).__name__ for e in t.get_path_effects()])
print(pe.Stroke(linewidth=3)._gc, [type(e).__name__ for e in l3.get_path_effects()])
