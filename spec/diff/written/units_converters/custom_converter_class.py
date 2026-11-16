# Register a units converter for a small Money class so amounts plot directly with a currency axis label.
import matplotlib.pyplot as plt
import matplotlib.units as munits
from matplotlib.ticker import FuncFormatter


class Money:
    def __init__(self, cents):
        self.cents = cents


class MoneyConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        if isinstance(value, Money):
            return value.cents / 100
        return [v.cents / 100 for v in value]

    @staticmethod
    def axisinfo(unit, axis):
        return munits.AxisInfo(majfmt=FuncFormatter(lambda x, _: f"€{x:,.2f}"), label="amount")

    @staticmethod
    def default_units(x, axis):
        return "EUR"


munits.registry[Money] = MoneyConverter()
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([1, 2, 3, 4], [Money(1250), Money(990), Money(1720), Money(1505)], "o-")
fig.canvas.draw()
print(ax.yaxis.get_label().get_text(), [t.get_text() for t in ax.get_yticklabels()][:3])
fig.savefig("money.png")
