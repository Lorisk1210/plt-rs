# A FileMovieWriter subclass that only keeps its temporary PNG frames (no external program), with frame_prefix and clear_temp.
import os
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation


class FramesOnly(animation.FileMovieWriter):
    supported_formats = ["png"]

    def _run(self):
        self.kept = sorted(os.listdir(os.path.dirname(self.temp_prefix)))

    def finish(self):
        self._run()

    @classmethod
    def isAvailable(cls):
        return True


fig, ax = plt.subplots(figsize=(2, 2), dpi=30)
pts = ax.scatter([], [])
ax.set_xlim(-1, 1)
ax.set_ylim(-1, 1)


def update(i):
    t = np.linspace(0, 2 * np.pi, i + 3)
    pts.set_offsets(np.c_[np.cos(t), np.sin(t)])
    return (pts,)


os.makedirs("frames", exist_ok=True)
w = FramesOnly(fps=2)
anim = animation.FuncAnimation(fig, update, frames=4)
w.setup(fig, "out.png", dpi=30, frame_prefix="frames/f_")
for i in range(4):
    update(i)
    w.grab_frame()
w.finish()
print(sorted(os.listdir("frames")))
print(w.frame_format, w.fps, w.dpi, w.outfile, w.frame_size)
print(w.kept, w._base_temp_name())
