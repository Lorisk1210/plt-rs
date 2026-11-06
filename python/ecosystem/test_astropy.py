"""astropy's visualization (``WCSAxes`` and the normalizations of
``astropy.visualization``) on pltrs as matplotlib."""

import numpy as np
import pytest

pytest.importorskip("astropy")


@pytest.fixture
def wcs():
    """A celestial tangent-plane WCS of a 50 × 40 image."""
    from astropy.wcs import WCS
    w = WCS(naxis=2)
    w.wcs.ctype = ["RA---TAN", "DEC--TAN"]
    w.wcs.crval = [150.0, 2.0]
    w.wcs.crpix = [25.0, 20.0]
    w.wcs.cdelt = [-0.01, 0.01]
    return w


@pytest.fixture
def image():
    """A smooth image of 40 × 50 pixels."""
    y, x = np.mgrid[0:40, 0:50]
    return np.exp(-((x - 25) ** 2 + (y - 20) ** 2) / 200.0)


def test_wcsaxes_image(check, wcs, image):
    import matplotlib.pyplot as plt
    fig = plt.figure()
    ax = fig.add_subplot(projection=wcs)
    ax.imshow(image, origin="lower", cmap="gray")
    ax.coords.grid(color="white", ls=":")
    ax.coords[0].set_axislabel("RA")
    ax.coords[1].set_axislabel("Dec")
    check(fig)


def test_wcsaxes_overlay_and_contours(check, wcs, image):
    import matplotlib.pyplot as plt
    fig = plt.figure()
    ax = fig.add_subplot(projection=wcs)
    ax.contour(image, levels=[0.2, 0.5, 0.8])
    ax.get_coords_overlay("galactic").grid(color="tab:blue")
    check(fig)


def test_simple_norm(check, image):
    import matplotlib.pyplot as plt
    from astropy.visualization import simple_norm
    fig, ax = plt.subplots()
    im = ax.imshow(image, norm=simple_norm(image, "sqrt", percent=99))
    fig.colorbar(im)
    check(fig)
