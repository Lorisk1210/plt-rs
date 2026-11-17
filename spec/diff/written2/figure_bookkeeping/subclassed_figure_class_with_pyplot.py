# A Figure subclass passed to plt.figure/subplots through FigureClass, with its own attributes and a watermark drawn in draw().
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.figure import Figure


class BrandedFigure(Figure):
    def __init__(self, *args, brand="ACME", **kwargs):
        super().__init__(*args, **kwargs)
        self.brand = brand
        self.draws = 0
        self.mark = self.text(0.99, 0.01, brand, ha="right", va="bottom", alpha=0.3, fontsize=8)

    def draw(self, renderer):
        self.draws += 1
        self.mark.set_text(f"{self.brand} #{self.draws}")
        super().draw(renderer)


fig = plt.figure(FigureClass=BrandedFigure, brand="Lab", figsize=(3, 2))
fig.add_subplot().plot([0, 1])
fig.savefig("branded.png")
fig.canvas.draw()
print(type(fig).__name__, fig.brand, fig.draws, fig.mark.get_text())
fig2, axs = plt.subplots(2, 1, FigureClass=BrandedFigure, brand="Two")
fig2.savefig("branded2.png")
print(type(fig2).__name__, fig2.draws, len(axs), plt.get_fignums())
fig3, ax3 = plt.subplot_mosaic("AB", FigureClass=BrandedFigure, brand="Mosaic")
print(type(fig3).__name__, fig3.brand, sorted(ax3))
print(isinstance(plt.gcf(), BrandedFigure), plt.gcf() is fig3)
sub = fig.subfigures(1, 1)
print(type(sub).__name__, sub.get_figure(root=True) is fig)
import pickle
copy = pickle.loads(pickle.dumps(fig))
print(type(copy).__name__, copy.brand, copy.draws)
plt.close("all")
