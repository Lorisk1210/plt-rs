# Comic-style "xkcd" chart of a running joke with annotations.
import numpy as np
import matplotlib.pyplot as plt

with plt.xkcd(scale=1, length=100, randomness=2):
    fig, ax = plt.subplots(figsize=(6, 4))
    data = np.ones(100)
    data[70:] -= np.arange(30)
    ax.plot(data)
    ax.annotate("THE DAY I REALIZED\nI COULD COOK BACON\nWHENEVER I WANTED", xy=(70, 1),
                arrowprops=dict(arrowstyle="->"), xytext=(15, -10))
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_ylim(-30, 10)
    ax.set_xlabel("time")
    ax.set_ylabel("my overall health")
    fig.savefig("bacon.png")
print(plt.rcParams["path.sketch"])
