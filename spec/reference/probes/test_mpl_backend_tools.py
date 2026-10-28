"""The shortcuts of the help window, ported from matplotlib's own
``tests/test_backend_tools.py``.

Area ``mpl_backend_tools``.

Ops
---
``backend_tools.ToolHelpBase.format_shortcut``  {key_sequence} -> the key
    name of a ``keymap.*`` rcParam as the help window shows it.

Cases
-----
Named after the upstream test, with its parameters.
"""

from matplotlib.backend_tools import ToolHelpBase

from spec_support import ProbeSet

probes = ProbeSet("mpl_backend_tools")


def test_format_shortcut():
    for key in ["home", "backspace", "f1", "ctrl+a", "ctrl+A", "a", "A", "ctrl+shift+f1", "1",
                "cmd+p", "cmd+1"]:
        probes.probe("backend_tools.ToolHelpBase.format_shortcut", ToolHelpBase.format_shortcut,
                     key_sequence=key)
