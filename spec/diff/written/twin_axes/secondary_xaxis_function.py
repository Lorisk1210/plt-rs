# Wavelength vs frequency with secondary_xaxis using forward/inverse functions.
import numpy as np
import matplotlib.pyplot as plt

wavelength_nm = np.linspace(380, 750, 200)
intensity = np.exp(-((wavelength_nm - 550) / 60) ** 2)

def nm_to_thz(nm):
    nm = np.asarray(nm, dtype=float)
    with np.errstate(divide="ignore"):
        return 299792.458 / nm

fig, ax = plt.subplots(figsize=(7, 3.5), layout="constrained")
ax.plot(wavelength_nm, intensity, color="darkgreen")
ax.set_xlabel("wavelength [nm]")
ax.set_ylabel("relative intensity")
sec = ax.secondary_xaxis("top", functions=(nm_to_thz, nm_to_thz))
sec.set_xlabel("frequency [THz]")
print(type(sec).__name__)
fig.savefig("spectrum.png")
