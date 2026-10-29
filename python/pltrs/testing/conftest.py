"""
The pytest configuration and fixtures matplotlib's test modules import
from ``matplotlib.testing.conftest`` (ported from matplotlib 3.11.2,
Matplotlib License).
"""

import importlib
import os
import sys

import pytest

from .. import _api, get_backend, use

#: The package (``pltrs``, or ``matplotlib`` when the suite loads pltrs
#: under that name), whose ``_called_from_pytest`` flag the hooks set.
_package = importlib.import_module("..", __package__)


def pytest_configure(config):
    """Register matplotlib's markers and warning filters, and select Agg."""
    for key, value in [
        ("markers", "flaky: (Provided by pytest-rerunfailures.)"),
        ("markers", "timeout: (Provided by pytest-timeout.)"),
        ("markers", "backend: Set alternate Matplotlib backend temporarily."),
        ("markers", "baseline_images: Compare output against references."),
        ("markers", "pytz: Tests that require pytz to be installed."),
        ("filterwarnings", "error"),
        ("filterwarnings",
         "ignore:.*The py23 module has been deprecated:DeprecationWarning"),
        ("filterwarnings",
         r"ignore:DynamicImporter.find_spec\(\) not found; "
         r"falling back to find_module\(\):ImportWarning"),
    ]:
        config.addinivalue_line(key, value)

    use('agg', force=True)
    _package._called_from_pytest = True
    _package._init_tests()


def pytest_unconfigure(config):
    """Note that pytest is no longer running."""
    _package._called_from_pytest = False


@pytest.fixture(autouse=True)
def mpl_test_settings(request):
    """Run each test in matplotlib's test configuration, then clean up."""
    from . import setup
    from .decorators import _cleanup_cm
    from .. import style

    with _cleanup_cm():

        backend = None
        backend_marker = request.node.get_closest_marker('backend')
        prev_backend = get_backend()
        if backend_marker is not None:
            assert len(backend_marker.args) == 1, \
                "Marker 'backend' must specify 1 backend."
            backend, = backend_marker.args
            skip_on_importerror = backend_marker.kwargs.get(
                'skip_on_importerror', False)

            # special case Qt backend importing to avoid conflicts
            if backend.lower().startswith('qt5'):
                if any(sys.modules.get(k) for k in ('PyQt4', 'PySide')):
                    pytest.skip('Qt4 binding already imported')

        setup()
        with _api.suppress_matplotlib_deprecation_warning():
            if backend is not None:
                # This import must come after setup() so it doesn't load the
                # default backend prematurely.
                from .. import pyplot as plt
                try:
                    plt.switch_backend(backend)
                except ImportError as exc:
                    if 'cairo' in backend.lower() or skip_on_importerror:
                        pytest.skip("Failed to switch to backend "
                                    f"{backend} ({exc}).")
                    else:
                        raise
            # Default of cleanup and image_comparison too.
            style.use(["classic", "_classic_test_patch"])
        try:
            yield
        finally:
            if backend is not None:
                plt.close("all")
                use(prev_backend)


@pytest.fixture
def high_memory(pytestconfig):
    """Skip tests that need much memory, unless asked for or on CI."""
    from . import is_ci_environment
    if not (os.environ.get('MPL_TEST_EXPENSIVE') or is_ci_environment()):
        pytest.skip('Test uses too much memory')


@pytest.fixture
def pd():
    """Fixture to import and configure pandas, skipping without it."""
    pd = pytest.importorskip('pandas')
    try:
        from pandas.plotting import (
            deregister_matplotlib_converters as deregister)
        deregister()
    except ImportError:
        pass
    return pd


@pytest.fixture
def xr():
    """Fixture to import xarray, skipping without it."""
    xr = pytest.importorskip('xarray')
    return xr


@pytest.fixture
def text_placeholders(monkeypatch):
    """
    Replace texts with placeholder rectangles.

    The rectangle size only depends on the font size and the number of
    characters. It is thus insensitive to font properties and rendering
    details.
    """
    from .. import text
    from ..patches import Rectangle

    def patched_get_text_metrics_with_cache(renderer, text, fontprop, ismath,
                                            dpi):
        height = fontprop.get_size()
        width = len(text) * height / 1.618  # Golden ratio for character size.
        descent = 0
        return width, height, descent

    def patched_text_draw(self, renderer):
        if renderer is not None:
            self._renderer = renderer
        if not self.get_visible():
            return
        if self.get_text() == '':
            return
        bbox = self.get_window_extent()
        rect = Rectangle(bbox.p0, bbox.width, bbox.height,
                         facecolor=self.get_color(), edgecolor='none')
        rect.draw(renderer)

    monkeypatch.setattr(text, '_get_text_metrics_with_cache',
                        patched_get_text_metrics_with_cache)
    monkeypatch.setattr(text.Text, 'draw', patched_text_draw)
