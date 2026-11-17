# Blitted FuncAnimation stepped by draw events: cached backgrounds per axes, what a resize does to them, and artists set animated.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation
from matplotlib.backend_bases import ResizeEvent

fig, (a1, a2) = plt.subplots(1, 2, figsize=(4, 2), dpi=50)
l1, = a1.plot(np.linspace(0, 1, 20), np.zeros(20))
l2, = a2.plot(np.linspace(0, 1, 20), np.zeros(20))
for a in (a1, a2):
    a.set_ylim(-1, 1)


def update(i):
    l1.set_ydata(np.sin(np.linspace(0, 6, 20) + i))
    l2.set_ydata(np.cos(np.linspace(0, 6, 20) + i))
    return l1, l2


anim = animation.FuncAnimation(fig, update, frames=10, blit=True, init_func=lambda: (l1, l2))
fig.canvas.draw()
print(l1.get_animated(), l2.get_animated(), len(anim._blit_cache))
for i in range(3):
    anim._draw_next_frame(i, blit=True)
print(len(anim._blit_cache), sorted(type(k).__name__ for k in anim._blit_cache))
print(sorted(set(type(v[1]).__name__ for v in anim._blit_cache.values())) if anim._blit_cache else [])
fig.set_size_inches(5, 3)
ResizeEvent("resize_event", fig.canvas)._process()
print(len(anim._blit_cache), anim._resize_id is not None)
fig.canvas.draw()
anim._draw_next_frame(4, blit=True)
print(round(float(l1.get_ydata()[0]), 4), round(float(l2.get_ydata()[0]), 4), len(anim._blit_cache))
anim.save("blit.gif", writer="pillow", fps=5)
print(l1.get_animated(), l2.get_animated())
