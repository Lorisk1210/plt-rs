"""matplotlib, as pltrs: ``import matplotlib`` loads pltrs under
matplotlib's name (see ``_pltrs_matplotlib_alias``)."""

import _pltrs_matplotlib_alias

_pltrs_matplotlib_alias.become(__name__)
