"""Helpers shared by the rule files."""


def cov(item, note=""):
    return ("covered", item, note)


def diff(item, note):
    return ("differently", item, note)


def na(note):
    return ("n/a", "", note)


def miss(note):
    return ("missing", "", note)


CHANGE_TRACKING = na("change tracking: pltrs recomputes on every draw (conventions: what was left out)")
TRANSFORM_META = na("metadata of matplotlib's lazy transform tree; pltrs transforms are resolved 2-D maps")
PY_CONST = na("class constant used by matplotlib's implementation")
ALIAS = na("alias: pltrs keeps one name per property (conventions: aliases)")
LABELLED_DATA = na("labelled data (`data=` with string keys) indexes Python mappings; pass the columns themselves")
PY_MACHINERY = na("Python machinery (class factories, introspection, file objects)")
BACKEND_PLUMBING = na("backend registration plumbing: pltrs has its backends built in (pltrs::pyplot::backend, pltrs_render)")
BUILTIN_TOOLS = diff("pltrs::gui (built-in window tools), Navigation", "a built-in tool of the figure window, bound to its keymap.* rcParam; programs add their own with Figure::add_tool(ToolSpec)")
EXTERNAL = na("drives an external program pltrs does not need: it writes the format itself")
