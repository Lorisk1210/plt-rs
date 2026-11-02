"""Corrections of automatic matches found by spot checks."""
from rules_base import cov, diff, na, miss, PY_CONST

RULES = [
    (r"matplotlib\.axis:Axis\.get_major_ticks", diff("Axis::ticklocs / Axis::ticklines / Axis::ticklabels, Axis::set_major_tick_line_props", "ticks are not separate objects; their parts are read and styled through the axis")),
    (r"matplotlib\.patches:Patch\.get_path", cov("PatchShape::path (Patch::shape)")),
    (r"matplotlib\.patches:Polygon\.set_xy", diff("Patch::update_shape", "a patch's geometry is a shape value with public fields")),
    (r"matplotlib\.path:Path\.should_simplify", diff("SimplifySettings, CleanupOptions", "simplification is decided when a path is conditioned for drawing")),
    (r"matplotlib\.ticker:.*\.set_locs", diff("Formatter::format_ticks", "a formatter sees all locations at once when it formats them")),
    (r"matplotlib\.gridspec:GridSpecBase\.(nrows|ncols)", cov("GridSpec::shape")),
    (r"matplotlib\.gridspec:GridSpecBase\.subplots", cov("Figure::subplots_on")),
    (r"matplotlib\.projections:PolarAxes\.name", PY_CONST),
    (r"matplotlib\.spines:Spine\.get_path", diff("Spine::line_extent, Projection::outline", "the spine's path follows its position and the projection")),
    (r"mpl_toolkits\.mplot3d\.axis3d:Axis\.get_major_ticks", cov("Axis3D::ticks")),
    (r"mpl_toolkits\.axes_grid1\.mpl_axes:Axes\.axis", diff("SimpleAxisArtist", "the side artists of an axes_grid1 axes")),
    (r"mpl_toolkits\.axisartist:AxisArtist\.get_helper", na("axis helpers are internal to AxisArtist")),
    (r"matplotlib\.axes:_AxesBase\.add_image", diff("Axes::attach", "images are made by imshow and friends; attach re-adds a removed one")),
    (r"matplotlib\.colorbar:Colorbar\.mappable", diff("Figure::colorbar(mappable, ..), Colorbar::update_normal", "the mappable is given when the colorbar is made")),
    (r"matplotlib\.axes:Axes\.legend_", cov("Axes::current_legend")),
    (r"matplotlib\.axis:Axis\.set_default_intervals", diff("Axis::axis_date", "default intervals are part of making an axis a date axis")),
]
