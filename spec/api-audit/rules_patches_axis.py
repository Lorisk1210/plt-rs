"""Rules for patches and axis."""
from rules_base import cov, diff, na, miss, ALIAS

P = r"matplotlib\.patches:"
A = r"matplotlib\.axis:"
SHAPE = diff("Patch::update_shape / Patch::set_shape", "a patch's geometry is a shape value with public fields, changed through update_shape")

RULES = [
    (P + r"Annulus\.(get_radii|radii)", cov("Annulus::semi_axes")),
    (P + r"(Arrow|FancyArrow)\.set_data", SHAPE),
    (P + r"FancyArrowPatch\.set_positions", SHAPE),
    (P + r"FancyArrowPatch\.set_patch[AB]", cov("ArrowSpec::patch_a / ArrowSpec::patch_b")),
    (P + r"(FancyBboxPatch|Rectangle)\.(get_x|get_y)", cov("Rectangle::xy / FancyBboxPatch::xy")),
    (P + r"(FancyBboxPatch|Rectangle)\.(set_x|set_y|set_bounds)", SHAPE),
    (P + r"FancyBboxPatch\.get_bbox", cov("FancyBboxPatch::xy / FancyBboxPatch::width / FancyBboxPatch::height")),
    (P + r"Patch\.get_verts", cov("PatchShape::placed_path")),
    (P + r"Patch\.(contains_point|contains_points)", cov("Patch::contains_display_point")),
    (P + r"Patch\.update_from", na("handles share state: there is nothing to copy (conventions)")),
    (P + r"Patch\.get_extents", cov("ArtistHandle::window_extent")),
    (P + r"Patch\.get_data_transform", cov("ArtistHandle::transform")),
    (P + r"Patch\.get_patch_transform", cov("PatchShape::patch_transform")),
    (P + r"Patch\.(get_aa|set_aa|get_ec|set_ec|get_fc|set_fc|get_ls|set_ls|get_lw|set_lw)", ALIAS),
    (P + r"Polygon\.set_closed", SHAPE),
    (P + r"Polygon\.(get_xy|xy)", cov("Polygon::vertices")),
    (P + r"StepPatch\.get_data", cov("StepPatch::values / StepPatch::edges / StepPatch::baseline")),
    (P + r"Wedge\.set_radius", SHAPE),
    (P + r"_Style\.(get_styles|pprint_styles)", diff("FromStr of BoxStyle / ArrowStyle / ConnectionStyle", "the enums list the styles")),
    (A + r"Axis\.get_scale", cov("Axes::xscale / Axes::yscale")),
    (A + r"Axis\.limit_range_for_scale", cov("Scale::limit_range_for_scale")),
    (A + r"(Axis|Tick)\.get_children", diff("Artist::children, ArtistHandle::findobj", "the artist tree")),
    (A + r"Axis\.reset_ticks", na("tick caches: ticks are rebuilt on every draw")),
    (A + r"(Axis|XAxis|YAxis)\.(get|set)_view_interval", cov("Axes::xlim / Axes::ylim / Axes::set_limits")),
    (A + r"(Axis|XAxis|YAxis)\.(get|set)_data_interval", diff("Axes::data_lim, Axes::update_datalim", "data limits are kept by the axes")),
    (A + r"(XAxis|YAxis)\.get_minpos", cov("TrackedBbox::minpos (Axes::data_lim)")),
    (A + r"Axis\.get_offset_text", diff("Axis::offset_text_extent / Axis::offset_text_visible, TicklabelFormat", "the offset text is drawn by the axis")),
    (A + r"Axis\.(get_pickradius|set_pickradius|pickradius)", miss("picking an axis within a radius")),
    (A + r"Axis\.get_(major|minor)ticklabels", cov("Axis::ticklabels(minor)")),
    (A + r"Axis\.get_minor_ticks", cov("Axis::ticklocs(true) / Axis::ticklines(true)")),
    (A + r"(Axis|XAxis|YAxis)\.get_tick_space", cov("AxisInfo::tick_space")),
    (A + r"YAxis\.set_offset_position", miss("which side the offset text sits on (`set_offset_position`)")),
    (A + r"Tick\.get_tickdir", cov("Axis::tick_style (TickDirection)")),
    (A + r"Tick\.get_tick_padding", cov("Axis::tick_padding")),
    (A + r"Tick\.(set_pad|get_pad)", cov("TickParams::pad (Axes::tick_params), Axis::tick_padding")),
    (A + r"Tick\.(set_clip_path|set_url|contains)", na("ticks are drawn by their axis, not separate artists")),
    (A + r"(Tick|XTick|YTick)\.(get_view_interval|update_position)", na("internal tick placement")),
]
