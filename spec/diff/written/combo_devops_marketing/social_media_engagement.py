# Social posts: engagement over time by post type (markers), weekday x hour heat map, and follower growth with milestones.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(204)
posts = pd.DataFrame({"ts": pd.Timestamp("2024-01-01") + pd.to_timedelta(rng.uniform(0, 90 * 24, 300), unit="h"),
                      "kind": rng.choice(["photo", "video", "text"], 300), "eng": rng.lognormal(4, 0.8, 300)})
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 1, 1)
for kind, marker in [("photo", "o"), ("video", "^"), ("text", "s")]:
    d = posts[posts.kind == kind]
    a.scatter(d["ts"], d["eng"], marker=marker, s=12, label=kind, alpha=0.7)
a.set_yscale("log")
a.legend()
b = fig.add_subplot(2, 2, 3)
grid = posts.groupby([posts.ts.dt.weekday, posts.ts.dt.hour // 3])["eng"].mean().unstack().reindex(index=range(7), columns=range(8))
b.imshow(grid, cmap="Purples", aspect="auto")
b.set_yticks(range(7), ["M", "T", "W", "T", "F", "S", "S"])
b.set_xticks(range(8), [f"{h * 3}h" for h in range(8)])
c = fig.add_subplot(2, 2, 4)
followers = 1000 + np.cumsum(rng.poisson(30, 90))
c.plot(followers)
for m in (2000, 3000, 4000):
    k = np.searchsorted(followers, m)
    if k < 90:
        c.annotate(f"{m}", (k, followers[k]), xytext=(-20, 15), textcoords="offset points", arrowprops=dict(arrowstyle="->"))
fig.autofmt_xdate()
fig.tight_layout()
print(posts.groupby("kind")["eng"].median().round(2).to_dict(), int(followers[-1]))
fig.savefig("social.png", dpi=50)
