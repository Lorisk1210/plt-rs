# functools.lru_cache around a figure factory, invalidation, and PNG bytes compared between cached and fresh renders.
import functools
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure

calls = []


@functools.lru_cache(maxsize=4)
def chart(freq: float, phase: float = 0.0) -> bytes:
    calls.append((freq, phase))
    fig = Figure(figsize=(3, 2), dpi=50)
    ax = fig.add_subplot()
    t = np.linspace(0, 1, 200)
    ax.plot(t, np.sin(2 * np.pi * freq * t + phase))
    ax.set_title(f"f={freq}")
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    return buf.getvalue()


a = chart(2.0)
b = chart(2.0)
c = chart(3.0)
d = chart(2.0, phase=0.0)
print(a is b, a == c, len(calls), chart.cache_info().hits, chart.cache_info().misses)
for f in (4.0, 5.0, 6.0, 7.0):
    chart(f)
print(chart.cache_info().currsize, len(calls))
chart(2.0)
print(len(calls), calls[-1])
chart.cache_clear()
fresh = chart(2.0)
print(fresh == a, len(calls))
print(len(a) > 1000, a[:8])
