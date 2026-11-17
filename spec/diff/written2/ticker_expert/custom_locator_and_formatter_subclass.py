# Locator and Formatter subclasses: a prime-number locator and a roman-numeral formatter with format_ticks, on major and minor axes.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker


class PrimeLocator(ticker.Locator):
    def __call__(self):
        vmin, vmax = self.axis.get_view_interval()
        return self.tick_values(vmin, vmax)

    def tick_values(self, vmin, vmax):
        lo, hi = int(max(2, min(vmin, vmax))), int(max(vmin, vmax))
        primes = [n for n in range(lo, hi + 1) if all(n % d for d in range(2, int(n ** 0.5) + 1))]
        return self.raise_if_exceeds(primes)


class Roman(ticker.Formatter):
    table = [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]

    def __call__(self, x, pos=None):
        n, out = int(round(x)), ""
        for value, sym in self.table:
            while n >= value:
                out += sym
                n -= value
        return out or "N"

    def format_ticks(self, values):
        return [f"[{self(v)}]" if i == 0 else self(v) for i, v in enumerate(values)]


fig, ax = plt.subplots(figsize=(6, 2), dpi=60)
ax.set_xlim(0, 30)
ax.xaxis.set_major_locator(PrimeLocator())
ax.xaxis.set_major_formatter(Roman())
ax.xaxis.set_minor_locator(ticker.MultipleLocator(1))
ax.xaxis.set_minor_formatter(ticker.NullFormatter())
fig.canvas.draw()
print(type(ax.get_xticks()).__name__, [int(v) for v in ax.get_xticks()])
print([t.get_text() for t in ax.get_xticklabels()])
print(len(ax.xaxis.get_minorticklocs()), Roman().format_ticks([1, 4, 9]), Roman()(0))
ax.set_xlim(0, 3000)
try:
    fig.canvas.draw()
    print(len(ax.get_xticks()))
except RuntimeError as e:
    print("RuntimeError", str(e)[:60])
print(PrimeLocator.MAXTICKS, isinstance(ax.xaxis.get_major_locator(), ticker.Locator))
fig.savefig("primes.png")
