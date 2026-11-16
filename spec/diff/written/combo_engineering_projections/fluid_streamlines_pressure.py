# Flow around a cylinder: pressure coefficient contourf, streamlines coloured by speed, cylinder patch and Cp on the surface.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

Y, X = np.mgrid[-2:2:120j, -3:3:180j]
R2 = X ** 2 + Y ** 2
U = 1 - (X ** 2 - Y ** 2) / np.maximum(R2, 1) ** 2
V = -2 * X * Y / np.maximum(R2, 1) ** 2
inside = R2 < 1
U[inside] = np.nan; V[inside] = np.nan
Cp = 1 - (U ** 2 + V ** 2)
fig, (a, b) = plt.subplots(1, 2, figsize=(13, 4.5), gridspec_kw={"width_ratios": [2, 1]})
cf = a.contourf(X, Y, np.ma.masked_invalid(Cp), levels=20, cmap="RdBu_r")
a.streamplot(X, Y, np.nan_to_num(U), np.nan_to_num(V), color=np.nan_to_num(np.hypot(U, V)), cmap="gray", density=1.2, linewidth=0.7)
a.add_patch(Circle((0, 0), 1, color="k"))
a.set_aspect("equal")
fig.colorbar(cf, ax=a, label="Cp")
th = np.linspace(0, 2 * np.pi, 200)
b.plot(np.degrees(th), 1 - 4 * np.sin(th) ** 2)
b.set(xlabel="angle [°]", ylabel="Cp on surface")
print(round(float(np.nanmin(Cp)), 3), round(float(np.nanmax(Cp)), 3))
fig.savefig("cylinder.png", dpi=50)
