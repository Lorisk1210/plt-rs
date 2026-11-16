# A Figure subclass passed with FigureClass= that adds a watermark text automatically.
import matplotlib.pyplot as plt
from matplotlib.figure import Figure


class WatermarkFigure(Figure):
    def __init__(self, *args, watermark=None, **kwargs):
        super().__init__(*args, **kwargs)
        if watermark is not None:
            self.text(0.5, 0.5, watermark, fontsize=40, color="gray", alpha=0.3, ha="center", va="center", rotation=30)


fig = plt.figure(FigureClass=WatermarkFigure, watermark="DRAFT", figsize=(5, 3.5))
ax = fig.add_subplot()
ax.plot([1, 2, 3], [3, 1, 2])
print(type(fig).__name__, len(fig.texts))
fig.savefig("draft.png")
