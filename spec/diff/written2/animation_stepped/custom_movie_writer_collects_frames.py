# An AbstractMovieWriter subclass that keeps frames in memory as RGBA arrays; FuncAnimation.save drives it with blitting.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation


class MemoryWriter(animation.AbstractMovieWriter):
    def __init__(self, fps=5):
        super().__init__(fps=fps)
        self.frames = []
        self.calls = []

    def setup(self, fig, outfile, dpi=None):
        super().setup(fig, outfile, dpi=dpi)
        self.calls.append(("setup", outfile, self.dpi))

    def grab_frame(self, **savefig_kwargs):
        self.fig.canvas.draw()
        self.frames.append(np.asarray(self.fig.canvas.buffer_rgba()).copy())
        self.calls.append(("grab", sorted(savefig_kwargs)))

    def finish(self):
        self.calls.append(("finish", len(self.frames)))


fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
x = np.linspace(0, 2 * np.pi, 60)
line, = ax.plot(x, np.sin(x))
ax.set_ylim(-1.2, 1.2)


def init():
    line.set_ydata(np.zeros_like(x))
    return (line,)


def update(i):
    line.set_ydata(np.sin(x + i / 3))
    return (line,)


anim = animation.FuncAnimation(fig, update, frames=8, init_func=init, blit=True, repeat=False)
writer = MemoryWriter(fps=4)
anim.save("ignored.mem", writer=writer, dpi=50)
print(writer.calls[0], writer.calls[-1], len(writer.frames), writer.frames[0].shape)
diffs = [int((writer.frames[k] != writer.frames[k + 1]).any(axis=2).sum()) for k in range(len(writer.frames) - 1)]
print(diffs)
print(writer.fps, writer.frame_size, writer.outfile)
print(type(anim.event_source).__name__, anim._save_count if hasattr(anim, "_save_count") else None)
