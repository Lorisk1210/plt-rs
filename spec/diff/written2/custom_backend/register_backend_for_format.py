# backend_bases.register_backend for a new savefig format handled by a canvas class defined here; Agg stays the backend.
import io
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import register_backend, get_registered_canvas_class
from matplotlib.backends.backend_agg import FigureCanvasAgg
import numpy as np
import sys
import types


class AsciiCanvas(FigureCanvasAgg):
    def print_txt(self, filename_or_obj, *, columns=40, **kwargs):
        self.draw()
        rgba = np.asarray(self.buffer_rgba())
        h, w = rgba.shape[:2]
        step = max(1, w // columns)
        gray = rgba[::step * 2, ::step, :3].mean(axis=2)
        lines = ["".join("#" if v < 128 else "+" if v < 220 else " " for v in row) for row in gray]
        text = "\n".join(lines)
        if hasattr(filename_or_obj, "write"):
            filename_or_obj.write(text.encode() if isinstance(filename_or_obj, io.BytesIO) else text)
        else:
            with open(filename_or_obj, "w") as f:
                f.write(text)


mod = types.ModuleType("ascii_backend")
mod.FigureCanvas = AsciiCanvas
sys.modules["ascii_backend"] = mod
register_backend("txt", "ascii_backend", "ASCII art")
print(get_registered_canvas_class("txt").__name__, "txt" in plt.figure().canvas.get_supported_filetypes())
fig, ax = plt.subplots(figsize=(3, 2), dpi=40)
ax.fill([0, 1, 1], [0, 0, 1], "k")
ax.set_axis_off()
fig.savefig("art.txt")
out = io.BytesIO()
fig.savefig(out, format="txt", columns=20)
text = open("art.txt").read()
print(len(text.splitlines()), sum(c == "#" for c in text), out.getvalue().count(b"\n"))
print(text.splitlines()[len(text.splitlines()) // 2])
print(matplotlib.get_backend(), type(fig.canvas).__name__)
print(sorted(plt.figure().canvas.get_supported_filetypes())[-3:])
try:
    fig.savefig("art.unknownformat")
except ValueError as e:
    print("ValueError", str(e)[:60])
