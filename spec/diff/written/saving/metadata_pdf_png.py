# Report figure saved as PDF and PNG with author/title metadata so the files are traceable.
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image

x = np.arange(1, 11)
fig, ax = plt.subplots()
ax.plot(x, x ** 2, "s-", label="quadratic")
ax.plot(x, 10 * x, "^--", label="linear")
ax.legend()
ax.set_xlabel("n")

fig.savefig("growth.pdf", metadata={"Title": "Growth comparison", "Author": "Data Team",
                                    "Subject": "quarterly report", "Keywords": "growth, n^2",
                                    "CreationDate": None, "ModDate": None})
fig.savefig("growth.png", metadata={"Title": "Growth comparison", "Author": "Data Team",
                                    "Software": None})
fig.savefig("growth.svg", metadata={"Title": "Growth comparison", "Date": None})

with Image.open("growth.png") as im:
    info = {k: v for k, v in im.info.items() if k in ("Title", "Author", "Software")}
    print(sorted(info.items()))
