# Animation.to_jshtml with embed limits and default modes; structure of the HTML (frames, controls) with the random ids masked.
import re
import numpy as np
import itertools
import uuid
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation

# The page's element ids come from uuid4: made reproducible, as one does to diff the HTML.
_ids = itertools.count(1)
uuid.uuid4 = lambda: uuid.UUID(int=next(_ids))

fig, ax = plt.subplots(figsize=(2, 2), dpi=30)
bar = ax.bar([0, 1, 2], [1, 2, 3])
ax.set_ylim(0, 5)


def update(i):
    for k, b in enumerate(bar):
        b.set_height(1 + (i + k) % 4)
    return list(bar)


anim = animation.FuncAnimation(fig, update, frames=5, interval=200)
for mode in ("loop", "once", "reflect"):
    html = anim.to_jshtml(fps=5, default_mode=mode)
    masked = re.sub(r"[0-9a-f]{32}", "ID", html)
    print(mode, masked.count("data:image/png;base64,"), len(set(re.findall(r"[0-9a-f]{32}", html))),
          'checked' in masked, f'value="{mode}"' in masked)
small = anim.to_jshtml(fps=5, embed_frames=True)
print(small.startswith("<link rel=") or small.lstrip().startswith("<"), "frames[0]" in small)
with plt.rc_context({"animation.embed_limit": 0.0001}):
    import warnings
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        limited = anim.to_jshtml()
    print([str(w.message)[:60] for w in caught], limited.count("data:image/png;base64,"))
print(plt.rcParams["animation.html"], plt.rcParams["animation.frame_format"])
print(anim._repr_html_() is None)
with plt.rc_context({"animation.html": "jshtml"}):
    print(anim._repr_html_().count("data:image/png;base64,"))
