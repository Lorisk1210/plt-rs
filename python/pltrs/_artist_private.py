"""
The private attributes of matplotlib's artists that released libraries
read and write, which pltrs keeps elsewhere.

* ``_axes``, the attribute behind matplotlib's `.Artist.axes` (nilearn's
  tests read ``image._axes``), ``_parent_figure`` behind ``figure``,
  ``_stale`` behind ``stale``, ``_animated`` behind ``get_animated``,
  ``_remove_method`` and ``_set_gc_clip``.
* matplotlib's *plain fields* (`FIELDS`, below): private attributes
  that a public getter of their class returns as they are
  (``Text._text``, ``Line2D._drawstyle``, ``Patch._edgecolor``). pltrs
  (Rust) holds their state, so a read of one is answered by pltrs's own
  getter of the class defining it (its subclasses' overrides left out,
  as matplotlib's attribute is the field, not the getter), and an
  assignment libraries make (the table names the setter) by its own
  setter, so that the next draw, getter and pickle see it.
"""

from . import _values
from ._native import artist as _artist, transforms as _transforms

# matplotlib's plain fields, written by spec/diff/private_fields.py from matplotlib 3.11.2:
#: class -> {field: (getter, setter or None)}
FIELDS = {'artist.Artist': {'_agg_filter': ('get_agg_filter', None),
                   '_alpha': ('get_alpha', None),
                   '_animated': ('get_animated', None),
                   '_clipon': ('get_clip_on', None),
                   '_clippath': ('get_clip_path', None),
                   '_gid': ('get_gid', None),
                   '_in_layout': ('get_in_layout', None),
                   '_label': ('get_label', None),
                   '_mouseover': ('get_mouseover', None),
                   '_path_effects': ('get_path_effects', None),
                   '_picker': ('get_picker', None),
                   '_rasterized': ('get_rasterized', 'set_rasterized'),
                   '_sketch': ('get_sketch_params', None),
                   '_url': ('get_url', None),
                   '_visible': ('get_visible', None)},
 'axes._base._AxesBase': {'_adjustable': ('get_adjustable', None),
                          '_anchor': ('get_anchor', None),
                          '_aspect': ('get_aspect', None),
                          '_axes_locator': ('get_axes_locator', None),
                          '_axisbelow': ('get_axisbelow', None),
                          '_box_aspect': ('get_box_aspect', None),
                          '_forward_navigation_events': ('get_forward_navigation_events', None),
                          '_frameon': ('get_frame_on', None),
                          '_navigate': ('get_navigate', None),
                          '_rasterization_zorder': ('get_rasterization_zorder', None),
                          '_subplotspec': ('get_subplotspec', None),
                          '_xmargin': ('get_xmargin', None),
                          '_ymargin': ('get_ymargin', None)},
 'axis.Axis': {'_converter': ('get_converter', None),
               '_pickradius': ('get_pickradius', None),
               '_remove_overlapping_locs': ('get_remove_overlapping_locs', None)},
 'axis.Tick': {'_base_pad': ('get_pad', None),
               '_loc': ('get_loc', None),
               '_tickdir': ('get_tickdir', None)},
 'backend_bases.FigureManagerBase': {'_window_title': ('get_window_title', None)},
 'backend_bases.GraphicsContextBase': {'_alpha': ('get_alpha', None),
                                       '_antialiased': ('get_antialiased', None),
                                       '_cliprect': ('get_clip_rectangle', None),
                                       '_dashes': ('get_dashes', None),
                                       '_forced_alpha': ('get_forced_alpha', None),
                                       '_gid': ('get_gid', None),
                                       '_hatch': ('get_hatch', None),
                                       '_hatch_color': ('get_hatch_color', None),
                                       '_hatch_linewidth': ('get_hatch_linewidth', None),
                                       '_linewidth': ('get_linewidth', None),
                                       '_rgb': ('get_rgb', None),
                                       '_sketch': ('get_sketch_params', None),
                                       '_snap': ('get_snap', None),
                                       '_url': ('get_url', None)},
 'backends._backend_tk.FigureCanvasTk': {'_tkcanvas': ('get_tk_widget', None)},
 'backends.backend_pgf.PdfPages': {'_n_figures': ('get_pagecount', None)},
 'backends.backend_webagg_core.FigureCanvasWebAggCore': {'_capture_scroll': ('get_capture_scroll',
                                                                             None)},
 'backends.backend_webagg_core.FigureManagerWebAgg': {'_window_title': ('get_window_title',
                                                                        None)},
 'backends.qt_editor._formlayout.ColorButton': {'_color': ('get_color', None)},
 'collections.Collection': {'_antialiaseds': ('get_antialiased', None),
                            '_facecolors': ('get_facecolor', None),
                            '_hatch': ('get_hatch', None),
                            '_hatch_linewidth': ('get_hatch_linewidth', None),
                            '_linestyles': ('get_linestyle', None),
                            '_linewidths': ('get_linewidth', None),
                            '_paths': ('get_paths', None),
                            '_pickradius': ('get_pickradius', None),
                            '_transforms': ('get_transforms', None),
                            '_urls': ('get_urls', None)},
 'collections.EventCollection': {'_linelength': ('get_linelength', None),
                                 '_lineoffset': ('get_lineoffset', None)},
 'collections.LineCollection': {'_edgecolors': ('get_color', None),
                                '_gapcolor': ('get_gapcolor', None)},
 'collections.PathCollection': {'_paths': ('get_paths', None)},
 'collections.RegularPolyCollection': {'_numsides': ('get_numsides', None),
                                       '_rotation': ('get_rotation', None)},
 'collections._CollectionWithSizes': {'_sizes': ('get_sizes', 'set_sizes')},
 'collections._MeshData': {'_coordinates': ('get_coordinates', None)},
 'colors.BivarColormap': {'_rgba_bad': ('get_bad', None),
                          '_rgba_outside': ('get_outside', None)},
 'dates.ConciseDateFormatter': {'_offset_string': ('get_offset', None)},
 'figure.Figure': {'_layout_engine': ('get_layout_engine', None)},
 'font_manager.FontProperties': {'_file': ('get_file', None),
                                 '_math_fontfamily': ('get_math_fontfamily', None),
                                 '_size': ('get_size', None),
                                 '_slant': ('get_style', None),
                                 '_stretch': ('get_stretch', None),
                                 '_variant': ('get_variant', None),
                                 '_weight': ('get_weight', None)},
 'gridspec.GridSpecBase': {'_col_width_ratios': ('get_width_ratios', None),
                           '_row_height_ratios': ('get_height_ratios', None)},
 'gridspec.SubplotSpec': {'_gridspec': ('get_gridspec', None)},
 'image._ImageBase': {'_filternorm': ('get_filternorm', None),
                      '_filterrad': ('get_filterrad', None),
                      '_interpolation': ('get_interpolation', None),
                      '_interpolation_stage': ('get_interpolation_stage', None),
                      '_resample': ('get_resample', None)},
 'lines.AxLine': {'_slope': ('get_slope', 'set_slope'),
                  '_xy1': ('get_xy1', None),
                  '_xy2': ('get_xy2', None)},
 'lines.Line2D': {'_antialiased': ('get_antialiased', None),
                  '_color': ('get_color', 'set_color'),
                  '_drawstyle': ('get_drawstyle', None),
                  '_gapcolor': ('get_gapcolor', None),
                  '_linestyle': ('get_linestyle', None),
                  '_linewidth': ('get_linewidth', None),
                  '_markeredgewidth': ('get_markeredgewidth', None),
                  '_markersize': ('get_markersize', None),
                  '_markevery': ('get_markevery', None),
                  '_pickradius': ('get_pickradius', None)},
 'markers.MarkerStyle': {'_alt_path': ('get_alt_path', None),
                         '_fillstyle': ('get_fillstyle', None),
                         '_marker': ('get_marker', None),
                         '_path': ('get_path', None),
                         '_snap_threshold': ('get_snap_threshold', None)},
 'mpl_toolkits.axes_grid1.axes_divider.Divider': {'_anchor': ('get_anchor', None),
                                                  '_aspect': ('get_aspect', None),
                                                  '_horizontal': ('get_horizontal', None),
                                                  '_locator': ('get_locator', None),
                                                  '_pos': ('get_position', None),
                                                  '_vertical': ('get_vertical', None)},
 'mpl_toolkits.axes_grid1.axes_divider.SubplotDivider': {'_subplotspec': ('get_subplotspec',
                                                                          None)},
 'mpl_toolkits.axes_grid1.axes_grid.Grid': {'_divider': ('get_divider', None)},
 'mpl_toolkits.axes_grid1.parasite_axes.ParasiteAxesBase': {'_viewlim_mode': ('get_viewlim_mode',
                                                                              None)},
 'mpl_toolkits.axisartist.axis_artist.AxisArtist': {'_axis_artist_helper': ('get_helper', None),
                                                    '_axisline_style': ('get_axisline_style',
                                                                        None)},
 'mpl_toolkits.axisartist.axis_artist.AxisLabel': {'_pad': ('get_pad', None)},
 'mpl_toolkits.axisartist.axis_artist.Ticks': {'_tick_dir': ('get_tick_direction', None),
                                               '_ticksize': ('get_ticksize', None)},
 'mpl_toolkits.axisartist.axislines.Axes': {'_grid_helper': ('get_grid_helper', None)},
 'mpl_toolkits.axisartist.axislines._FixedAxisArtistHelperBase': {'_path': ('get_line', None)},
 'mpl_toolkits.axisartist.grid_finder.GridFinder': {'_aux_transform': ('get_transform', None)},
 'mpl_toolkits.mplot3d.art3d.Line3D': {'_verts3d': ('get_data_3d', None)},
 'mpl_toolkits.mplot3d.art3d.Patch3DCollection': {'_depthshade': ('get_depthshade', None)},
 'mpl_toolkits.mplot3d.art3d.Path3DCollection': {'_depthshade': ('get_depthshade', None)},
 'mpl_toolkits.mplot3d.axes3d.Axes3D': {'_zmargin': ('get_zmargin', None)},
 'mpl_toolkits.mplot3d.axis3d.Axis': {'_label_position': ('get_label_position', None),
                                      '_tick_position': ('get_ticks_position',
                                                         'set_ticks_position')},
 'offsetbox.AnchoredOffsetbox': {'_child': ('get_child', None)},
 'offsetbox.AuxTransformBox': {'_offset': ('get_offset', None)},
 'offsetbox.DrawingArea': {'_offset': ('get_offset', None)},
 'offsetbox.OffsetBox': {'_children': ('get_children', None)},
 'offsetbox.OffsetImage': {'_data': ('get_data', None),
                           '_offset': ('get_offset', None),
                           '_zoom': ('get_zoom', None)},
 'offsetbox.TextArea': {'_multilinebaseline': ('get_multilinebaseline', None),
                        '_offset': ('get_offset', None)},
 'patches.Annulus': {'_angle': ('get_angle', 'set_angle'),
                     '_center': ('get_center', None),
                     '_width': ('get_width', None)},
 'patches.Arrow': {'_patch_transform': ('get_patch_transform', None),
                   '_path': ('get_path', None)},
 'patches.ConnectionPatch': {'_annotation_clip': ('get_annotation_clip', None)},
 'patches.Ellipse': {'_angle': ('get_angle', 'set_angle'),
                     '_center': ('get_center', None),
                     '_height': ('get_height', None),
                     '_path': ('get_path', None),
                     '_width': ('get_width', None)},
 'patches.FancyArrowPatch': {'_arrow_transmuter': ('get_arrowstyle', None),
                             '_connector': ('get_connectionstyle', 'set_connectionstyle'),
                             '_mutation_scale': ('get_mutation_scale', None)},
 'patches.FancyBboxPatch': {'_bbox_transmuter': ('get_boxstyle', None),
                            '_height': ('get_height', None),
                            '_mutation_scale': ('get_mutation_scale', None),
                            '_width': ('get_width', None)},
 'patches.Patch': {'_antialiased': ('get_antialiased', None),
                   '_edgecolor': ('get_edgecolor', None),
                   '_facecolor': ('get_facecolor', None),
                   '_fill': ('get_fill', None),
                   '_gapcolor': ('get_edgegapcolor', None),
                   '_hatch': ('get_hatch', None),
                   '_hatch_linewidth': ('get_hatch_linewidth', None),
                   '_linestyle': ('get_linestyle', None),
                   '_linewidth': ('get_linewidth', None)},
 'patches.PathPatch': {'_path': ('get_path', None)},
 'patches.Polygon': {'_closed': ('get_closed', None), '_path': ('get_path', None)},
 'patches.Rectangle': {'_height': ('get_height', None),
                       '_width': ('get_width', None),
                       '_x0': ('get_x', None),
                       '_y0': ('get_y', None)},
 'patches.RegularPolygon': {'_path': ('get_path', None)},
 'scale.AsinhScale': {'_transform': ('get_transform', None)},
 'scale.FuncScale': {'_transform': ('get_transform', None)},
 'scale.FuncScaleLog': {'_transform': ('get_transform', None)},
 'scale.LogScale': {'_transform': ('get_transform', None)},
 'scale.LogitScale': {'_transform': ('get_transform', None)},
 'scale.SymmetricalLogScale': {'_transform': ('get_transform', None)},
 'spines.Spine': {'_bounds': ('get_bounds', None), '_path': ('get_path', None)},
 'table.Cell': {'_text': ('get_text', None)},
 'table.Table': {'_cells': ('get_celld', None)},
 'text.Annotation': {'_textcoords': ('get_anncoords', None)},
 'text.OffsetFrom': {'_unit': ('get_unit', None)},
 'text.Text': {'_antialiased': ('get_antialiased', None),
               '_bbox_patch': ('get_bbox_patch', None),
               '_color': ('get_color', 'set_color'),
               '_features': ('get_fontfeatures', None),
               '_fontproperties': ('get_fontproperties', None),
               '_horizontalalignment': ('get_horizontalalignment', None),
               '_language': ('get_language', None),
               '_linespacing': ('get_linespacing', None),
               '_parse_math': ('get_parse_math', None),
               '_rotation_mode': ('get_rotation_mode', None),
               '_text': ('get_text', 'set_text'),
               '_transform_rotates_text': ('get_transform_rotates_text', None),
               '_usetex': ('get_usetex', 'set_usetex'),
               '_verticalalignment': ('get_verticalalignment', None),
               '_wrap': ('get_wrap', None)},
 'text._AnnotationBase': {'_annotation_clip': ('get_annotation_clip', None)},
 'textpath.TextPath': {'_size': ('get_size', None)},
 'ticker.ScalarFormatter': {'_useLocale': ('get_useLocale', None),
                            '_useMathText': ('get_useMathText', None),
                            '_useOffset': ('get_useOffset', None),
                            '_usetex': ('get_usetex', None)},
 'ticker._DummyAxis': {'_data_interval': ('get_data_interval', None),
                       '_minpos': ('get_minpos', None),
                       '_view_interval': ('get_view_interval', None)},
 'transforms.IdentityTransform': {'_mtx': ('get_matrix', None)},
 'widgets.Widget': {'_active': ('get_active', None)},
 'widgets._SelectorWidget': {'_visible': ('get_visible', None)}}
# (end of the plain fields)

#: Every plain field's name, to leave other misses at once.
_FIELD_NAMES = frozenset(name for fields in FIELDS.values() for name in fields)
#: class -> its plain fields (or None), by the class.
_fields_of = {}


def _table_key(cls):
    """A class as `FIELDS` names it: by module without its top-level
    package (but for the toolkits) and qualname."""
    module = cls.__module__ or ""
    if not module.startswith("mpl_toolkits."):
        module = module.partition(".")[2]
    return f"{module}.{cls.__qualname__}"


def fields_of(cls):
    """``{field: (getter, setter)}`` of matplotlib's class ``cls`` stands
    for, or None."""
    try:
        return _fields_of[cls]
    except KeyError:
        found = FIELDS.get(_table_key(cls))
        if found is not None:
            # (Only the package's classes are kept: a class of the
            # program's would live as long as the table.)
            _fields_of[cls] = found
        return found


#: The package's name (``pltrs``, or ``matplotlib`` where pltrs is loaded
#: as matplotlib), which its classes' modules start with.
_PACKAGE = __name__.partition(".")[0]


def own_method(obj, name):
    """The package's own method ``name`` of ``obj``'s class, as a function:
    what matplotlib's field holds, whatever a subclass of the program's
    overrides."""
    for klass in type(obj).__mro__:
        module = klass.__module__ or ""
        if (module == _PACKAGE or module.startswith((_PACKAGE + ".", "mpl_toolkits."))) and name in vars(klass):
            return vars(klass)[name]
    return getattr(type(obj), name)


#: The fields being read, by object id and name: a getter of pltrs's
#: reading the very attribute (unset) finds none, rather than itself.
_reading = set()


def plain_field(obj, name):
    """The plain field ``name`` of ``obj``, as matplotlib's getter of the
    class defining it gives it; AttributeError where none does, or where a
    class of the package overrides that getter with a read of the field."""
    key = (id(obj), name)
    if key in _reading:
        raise AttributeError(name)
    for klass in type(obj).__mro__:
        fields = fields_of(klass)
        if fields is not None and name in fields:
            getter = getattr(klass, fields[name][0], None)
            # A class of the package between whose own getter reads the very
            # field keeps none (NonUniformImage's ``_filterrad``, which
            # matplotlib's never sets): its getter's miss stands.
            nearest = own_method(obj, fields[name][0])
            if nearest is not getter and name in getattr(getattr(nearest, "__code__", None), "co_names", ()):
                getter = None
            if getter is not None:
                _reading.add(key)
                try:
                    return getter(obj)
                finally:
                    _reading.discard(key)
    raise AttributeError(name)


class PrivateAttributesMixin:
    """matplotlib's private attributes of an artist, where nothing set
    them."""

    @property
    def _remove_method(self):
        # matplotlib's function removing the artist from what holds it,
        # which its container sets (None for what nothing can remove):
        # the one an owner of the package's set, else, for an artist an
        # Axes or a figure holds, its removal from there.
        given = self.__dict__.get("_remove_method")
        if given is not None or not self._natively_removable():
            return given
        from .axes._base import _AxesBase
        if isinstance(self, _AxesBase):
            # matplotlib's figure removes its Axes with `delaxes`.
            return self.get_figure(root=False).delaxes
        return type(self)._remove_natively

    @_remove_method.setter
    def _remove_method(self, function):
        self.__dict__["_remove_method"] = function

    def _natively_removable(self):
        """Whether an Axes or a figure of pltrs (Rust) holds the artist as
        one it removes (`.Artist.remove`)."""
        figure = self.__dict__.get("_figure")
        return (figure is not None and figure is not self and not self.__dict__.get("_decoupled")
                and not _artist.is_part(self._native) and not self.__dict__.get("_only_given_figure"))

    @property
    def _stale(self):
        # matplotlib's flag behind `stale`, which a program sets without
        # running the stale callbacks.
        return self.stale

    @_stale.setter
    def _stale(self, value):
        _artist.set_stale(self._native, bool(value))

    @property
    def _transformSet(self):
        # matplotlib's flag behind `is_transform_set`, which a program
        # clears so that the Axes adding the artist gives it ``transData``
        # (pyregion).
        return self.is_transform_set()

    @_transformSet.setter
    def _transformSet(self, value):
        if not value:
            self.__dict__.pop("_given_transform", None)
            _transforms.unset_transform(self._native)
        elif not self.is_transform_set():
            self.set_transform(self.get_transform())

    @property
    def _animated(self):
        # matplotlib's flag behind `get_animated`, which a program sets
        # without `set_animated`'s ``pchanged`` (ultraplot leaves its Axes
        # out of a figure's draw so): the figure's draws see it.
        return self.get_animated()

    @_animated.setter
    def _animated(self, value):
        _artist.set_animated(self._native, bool(value))
        _values.remember(self, "animated", value)

    @property
    def _parent_figure(self):
        # matplotlib's (sub)figure behind `figure`, which programs set as
        # `set_figure` sets it (marsilea).
        return self.get_figure(root=False)

    @_parent_figure.setter
    def _parent_figure(self, figure):
        self.set_figure(figure)

    def _set_gc_clip(self, gc):
        """Set the clip properly for the gc."""
        # matplotlib 3.11.2's, under the Matplotlib License (over the
        # public getters of what it reads).
        if self.get_clip_on():
            if self.get_clip_box() is not None:
                gc.set_clip_rectangle(self.get_clip_box())
            gc.set_clip_path(self.get_clip_path())
        else:
            gc.set_clip_rectangle(None)
            gc.set_clip_path(None)

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)
        for name, (getter, setter) in (fields_of(cls) or {}).items():
            if setter is not None and not hasattr(cls, name):
                setattr(cls, name, _values.assignable(getattr(cls, getter), getattr(cls, setter)))

    def __dir__(self):
        # The plain fields the class answers (`__getattr__`), which
        # matplotlib's instance holds as attributes.
        names = set(super().__dir__())
        for klass in type(self).__mro__:
            names.update(fields_of(klass) or ())
        return sorted(names)

    def __getattr__(self, name):
        if name == "stale_callback":
            from ._artist_staleness import default_stale_callback
            return default_stale_callback(self)
        if name == "_axes":
            from .figure import FigureBase
            # A figure has none: matplotlib's deletes it, its ``axes`` being
            # the list of its Axes.
            if not isinstance(self, FigureBase):
                # The Axes the artist is in, as its ``axes`` gives it (an
                # Axes itself, as matplotlib's ``self.axes = self`` sets it).
                return type(self).axes.fget(self)
        prop = getattr(type(self), name, None)
        if isinstance(prop, property) and prop.fget is not None:
            # A property whose getter raised: its error is the one, as
            # without this hook (a figure pyplot never numbered).
            return prop.fget(self)
        if name in _FIELD_NAMES:
            try:
                return plain_field(self, name)
            except AttributeError:
                pass
        raise AttributeError(
            f"'{type(self).__name__}' object has no attribute '{name}'",
            name=name, obj=self)
