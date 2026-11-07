"""What the ``hostile`` generator leaves out, and the values it adds to
the ``api`` generator's."""

from __future__ import annotations

#: Callables left out by name: they wait, loop, run external programs or
#: change the global configuration every later statement would see.
LEFT_OUT = {
    "pause", "ginput", "waitforbuttonpress", "start_event_loop", "stop_event_loop", "mainloop",
    "start_main_loop", "flush_events", "switch_backend", "use", "rc_file", "rc_file_defaults", "rcdefaults",
    "xkcd", "install_repl_displayhook", "uninstall_repl_displayhook", "ion", "ioff", "interactive", "save",
    "to_html5_video", "to_jshtml", "setup", "grab_frame", "finish", "saving", "new_frame_seq",
    "new_saved_frame_seq", "make_dvi", "make_png", "make_tex", "get_text_width_height_descent", "get_grey",
    "get_rgba",
}
#: Modules whose callables are left out: they run TeX or test helpers.
MODULES_LEFT_OUT = ("matplotlib.testing", "matplotlib.texmanager", "matplotlib.dviread",
                    "matplotlib.backends.backend_pgf")
#: Special methods left out: Python calls them on its own terms (making,
#: copying, pickling and deleting objects, descriptors, buffers).
DUNDERS_LEFT_OUT = {
    "__del__", "__exit__", "__enter__", "__init__", "__new__", "__init_subclass__", "__class_getitem__",
    "__set_name__", "__get__", "__set__", "__delete__", "__reduce__", "__reduce_ex__", "__setstate__",
    "__getstate__", "__copy__", "__deepcopy__", "__buffer__", "__release_buffer__", "__iter__", "__next__",
}
#: (callable, parameter) -> the values matplotlib never returns from (it
#: loops, waits for clicks or fills memory), ``*`` for every value.
NEVER_RETURNS = {
    ("streamplot", "integration_max_error_scale"): {"float('nan')"},
    ("clabel", "manual"): {"1", "np.True_"},
    ("find_bezier_t_intersecting_with_closedpath", "t0"): {"float('nan')", "float('inf')", "'0.0'"},
    ("find_bezier_t_intersecting_with_closedpath", "t1"): {"float('nan')", "float('inf')", "'1.0'"},
    ("find_bezier_t_intersecting_with_closedpath", "tolerance"): {"*"},
    ("split_bezier_intersecting_with_closedpath", "bezier"): {"*"},
    ("split_bezier_intersecting_with_closedpath", "tolerance"): {"*"},
    ("split_de_casteljau", "beta"): {"*"},
}
#: callable -> keywords its calls are given, so that what they draw stays
#: small (a hexbin of matplotlib's default grid has 10,000 hexagons).
SMALL = {"hexbin": ("gridsize", "4")}
#: Values of required parameters the ``api`` generator's tables lack.
REQUIRED = {
    "parents": ["ax"],
    "parent": ["ax"],
    "grid_helper": ["__import__('mpl_toolkits.axisartist.floating_axes', fromlist=['_'])"
                    ".GridHelperCurveLinear(transforms.Affine2D(), extremes=(0, 4, 0, 3))"],
    "contents": ["b'%PDF-1.4'"],
    "file": ["io.BytesIO()"],
}
