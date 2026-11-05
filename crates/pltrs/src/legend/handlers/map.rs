//! Which handler draws which handle: the keys handlers are registered
//! under, handler maps, and the default map every legend starts from
//! (matplotlib's `handler_map`, `Legend.get_default_handler_map`,
//! `set_default_handler_map`, `update_default_handler_map` and
//! `get_legend_handler`).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

use super::LegendHandler;
use super::builtin::{
    HandlerErrorbar, HandlerLine2D, HandlerLineCollection, HandlerPatch, HandlerPathCollection,
    HandlerPolyCollection, HandlerStem, HandlerStepPatch,
};
use crate::artist::{ArtistId, ArtistStore};
use crate::collections::{CollectionArtist, CollectionKind};
use crate::legend::HandlerTuple;
use crate::legend::handle::Source;
use crate::lines::LineArtist;
use crate::patches::StepPatch;

/// A Rust type a handler is registered for: a patch shape such as
/// `Circle`, or the type of the objects of
/// [`LegendHandle::object`](crate::legend::LegendHandle::object). Keys
/// compare by type.
#[derive(Clone, Copy, Debug)]
pub struct TypeKey {
    id: TypeId,
    name: &'static str,
}

/// The types an archived handler map may be keyed by: pltrs's patch
/// shapes. A key of the program's own type is not archived.
fn archivable_keys() -> [TypeKey; 22] {
    use crate::axes_grid1::{BboxConnector, BboxConnectorPatch, BboxPatch};
    use crate::mplot3d::{ProjectedOutline, ProjectedPath};
    use crate::patches::{
        Annulus, Arc, Arrow, Circle, CirclePolygon, ConnectionPatch, Ellipse, FancyArrow,
        FancyArrowPatch, FancyBboxPatch, PathShape, Polygon, Rectangle, RegularPolygon, Wedge,
    };
    use crate::table::CellShape;
    [
        TypeKey::of::<Rectangle>(),
        TypeKey::of::<Ellipse>(),
        TypeKey::of::<Circle>(),
        TypeKey::of::<Polygon>(),
        TypeKey::of::<Wedge>(),
        TypeKey::of::<RegularPolygon>(),
        TypeKey::of::<CirclePolygon>(),
        TypeKey::of::<PathShape>(),
        TypeKey::of::<Arc>(),
        TypeKey::of::<Annulus>(),
        TypeKey::of::<Arrow>(),
        TypeKey::of::<FancyArrow>(),
        TypeKey::of::<FancyArrowPatch>(),
        TypeKey::of::<ConnectionPatch>(),
        TypeKey::of::<FancyBboxPatch>(),
        TypeKey::of::<StepPatch>(),
        TypeKey::of::<BboxPatch>(),
        TypeKey::of::<BboxConnector>(),
        TypeKey::of::<BboxConnectorPatch>(),
        TypeKey::of::<CellShape>(),
        TypeKey::of::<ProjectedOutline>(),
        TypeKey::of::<ProjectedPath>(),
    ]
}

/// Archived as the name of its type, one of pltrs's patch shapes.
impl Archive for TypeKey {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let known = archivable_keys()
            .into_iter()
            .find(|key| key == self)
            .ok_or_else(|| {
                ArchiveError::unsupported(format!("a legend handler for {}", self.name))
            })?;
        encoder.write_str(known.name);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let name = decoder.read_str()?;
        archivable_keys()
            .into_iter()
            .find(|key| key.name == name)
            .ok_or_else(|| ArchiveError::invalid(format!("no handler key for {name}")))
    }
}

impl TypeKey {
    /// The key of type `T`.
    pub fn of<T: Any>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: std::any::type_name::<T>(),
        }
    }

    /// The key of the concrete type behind `value`, named only
    /// generically (keys compare by type alone).
    fn of_value(value: &dyn Any, name: &'static str) -> Self {
        Self {
            id: value.type_id(),
            name,
        }
    }

    /// The name of the type.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl PartialEq for TypeKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TypeKey {}

impl Hash for TypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// A kind of handle a [`LegendHandler`] is registered for in a
/// [`HandlerMap`] (matplotlib's handler map keys: artist classes,
/// `tuple`, or the class of any other object).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandlerKey {
    /// Lines (matplotlib's `Line2D`).
    Line,
    /// Patches of every shape (`Patch`); a [`HandlerKey::Type`] of one
    /// shape comes first.
    Patch,
    /// Line collections (`LineCollection`).
    LineCollection,
    /// Markers sized in points², as made by `scatter` (`PathCollection`).
    PathCollection,
    /// Polygons, as made by `fill_between` (`PolyCollection`).
    PolyCollection,
    /// The outlines of patches drawn together (`PatchCollection`).
    PatchCollection,
    /// The bars of a `bar` call (`BarContainer`).
    BarContainer,
    /// The artists of an `errorbar` call (`ErrorbarContainer`).
    ErrorbarContainer,
    /// The artists of a `stem` call (`StemContainer`).
    StemContainer,
    /// Several handles in one entry
    /// ([`LegendHandle::tuple`](crate::legend::LegendHandle::tuple);
    /// matplotlib's `tuple`).
    Tuple,
    /// One Rust type: a patch shape, or the type of the objects of
    /// [`LegendHandle::object`](crate::legend::LegendHandle::object).
    Type(TypeKey),
}

pltrs_core::archive_enum!(HandlerKey {
    Line,
    Patch,
    LineCollection,
    PathCollection,
    PolyCollection,
    PatchCollection,
    BarContainer,
    ErrorbarContainer,
    StemContainer,
    Tuple,
    Type(v0),
});

impl HandlerKey {
    /// The key of the Rust type `T`, such as a patch shape
    /// (`HandlerKey::of::<Circle>()`) or the type of custom handle
    /// objects.
    pub fn of<T: Any>() -> Self {
        Self::Type(TypeKey::of::<T>())
    }

    /// The keys that may hold the handler of artist `id`, the most
    /// specific first; none for artists legends cannot show.
    pub(crate) fn of_artist(store: &ArtistStore, id: ArtistId) -> Vec<Self> {
        let Some(artist) = store.get(id) else {
            return Vec::new();
        };
        if store.get_as::<LineArtist>(id).is_some() {
            return vec![Self::Line];
        }
        if let Some(view) = artist.as_patch() {
            return vec![
                Self::Type(TypeKey::of_value(view.shape, "patch shape")),
                Self::Patch,
            ];
        }
        store
            .get_as::<CollectionArtist>(id)
            .map(Self::of_collection)
            .unwrap_or_default()
    }

    /// The keys that may hold the handler of `collection`; none for the
    /// kinds legends cannot show.
    pub(crate) fn of_collection(collection: &CollectionArtist) -> Vec<Self> {
        match collection.kind {
            CollectionKind::Line => vec![Self::LineCollection],
            CollectionKind::Path => vec![Self::PathCollection],
            CollectionKind::Poly => vec![Self::PolyCollection],
            CollectionKind::Patch => vec![Self::PatchCollection],
            CollectionKind::Generic | CollectionKind::QuadMesh | CollectionKind::TriMesh => {
                Vec::new()
            }
        }
    }
}

/// Handlers by the kind of handle they draw (matplotlib's `handler_map`
/// dictionary). Handlers are shared between clones of a map.
///
/// ```
/// use pltrs::legend::{HandlerKey, HandlerMap, HandlerPatch, Legend};
/// let map = HandlerMap::new().with(HandlerKey::Line, HandlerPatch);
/// assert!(map.get(HandlerKey::Line).is_some());
/// assert!(Legend::default_handler_map().get(HandlerKey::Tuple).is_some());
/// ```
#[derive(Clone, Debug, Default)]
pub struct HandlerMap {
    handlers: HashMap<HandlerKey, Rc<dyn LegendHandler>>,
}

pltrs_core::archive_struct!(HandlerMap { handlers });

impl HandlerMap {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// matplotlib's default handler map: lines, patches, step patches,
    /// line, path, polygon and patch collections, bar, error bar and stem
    /// containers, and tuples.
    pub fn builtin() -> Self {
        Self::new()
            .with(HandlerKey::StemContainer, HandlerStem)
            .with(HandlerKey::ErrorbarContainer, HandlerErrorbar)
            .with(HandlerKey::Line, HandlerLine2D)
            .with(HandlerKey::Patch, HandlerPatch)
            .with(HandlerKey::of::<StepPatch>(), HandlerStepPatch)
            .with(HandlerKey::LineCollection, HandlerLineCollection)
            .with(HandlerKey::BarContainer, HandlerPatch)
            .with(HandlerKey::Tuple, HandlerTuple::default())
            .with(HandlerKey::PathCollection, HandlerPathCollection)
            .with(HandlerKey::PolyCollection, HandlerPolyCollection)
            .with(HandlerKey::PatchCollection, HandlerPolyCollection)
    }

    /// The map with `handler` drawing the handles of `key`.
    #[must_use]
    pub fn with(mut self, key: HandlerKey, handler: impl LegendHandler + 'static) -> Self {
        self.insert(key, Rc::new(handler));
        self
    }

    /// Makes `handler` draw the handles of `key`, returning the handler it
    /// replaces.
    pub fn insert(
        &mut self,
        key: HandlerKey,
        handler: Rc<dyn LegendHandler>,
    ) -> Option<Rc<dyn LegendHandler>> {
        self.handlers.insert(key, handler)
    }

    /// The handler registered for `key` itself, if any.
    pub fn get(&self, key: HandlerKey) -> Option<Rc<dyn LegendHandler>> {
        self.handlers.get(&key).cloned()
    }

    /// Adds every handler of `other`, replacing those of the same keys
    /// (Python's `dict.update`).
    pub fn update(&mut self, other: &Self) {
        self.handlers
            .extend((other.handlers.iter()).map(|(&key, handler)| (key, Rc::clone(handler))));
    }

    /// The keys with a handler, in no particular order.
    pub fn keys(&self) -> impl Iterator<Item = HandlerKey> + '_ {
        self.handlers.keys().copied()
    }

    /// The number of keys with a handler.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether no key has a handler.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// The handler drawing `source`: its own, else the one of its most
    /// specific key that has one (matplotlib's `get_legend_handler`).
    pub(crate) fn handler_for(
        &self,
        source: &Source,
        store: &ArtistStore,
    ) -> Option<Rc<dyn LegendHandler>> {
        if let Source::Handled(_, handler) = source {
            return Some(Rc::clone(&handler.0));
        }
        (source.handler_keys(store).into_iter()).find_map(|key| self.get(key))
    }
}

pltrs_core::thread_state! {
    /// The handler map legends start from (matplotlib's class attribute
    /// `Legend._default_handler_map`), per thread like the rc parameters.
    static DEFAULT_HANDLERS: RefCell<HandlerMap> = RefCell::new(HandlerMap::builtin());
}

/// A copy of the default handler map.
pub(crate) fn default_handlers() -> HandlerMap {
    DEFAULT_HANDLERS.with(|map| map.borrow().clone())
}

/// Replaces the default handler map. The handlers replaced are dropped
/// once the map is no longer borrowed, since dropping a handler of the
/// program's may run its code.
pub(crate) fn set_default_handlers(map: HandlerMap) {
    DEFAULT_HANDLERS.set(map);
}

/// Adds the handlers of `map` to the default handler map (the handlers
/// replaced are dropped once it is no longer borrowed).
pub(crate) fn update_default_handlers(map: &HandlerMap) {
    let mut updated = default_handlers();
    updated.update(map);
    set_default_handlers(updated);
}

/// The default handler map with the handlers of `custom` added
/// (matplotlib's `get_legend_handler_map`).
pub(crate) fn with_defaults(custom: &HandlerMap) -> HandlerMap {
    let mut map = default_handlers();
    map.update(custom);
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::legend::handle::LegendHandle;
    use crate::legend::{
        HandlerArtist, HandlerEntry, Legend, LegendArtist, LegendError, LegendOptions,
    };
    use crate::patches::{Circle, Ellipse, PatchArtist, PatchProps, Rectangle};

    /// matplotlib's legend guide `AnyObject`, which is no artist.
    #[derive(Debug)]
    struct AnyObject {
        hatch: &'static str,
    }

    /// The guide's `AnyObjectHandler`: a red rectangle filling the box,
    /// hatched as the object says.
    #[derive(Debug)]
    struct AnyObjectHandler;

    impl LegendHandler for AnyObjectHandler {
        fn create_artists(
            &self,
            entry: &HandlerEntry<'_>,
        ) -> Result<Vec<HandlerArtist>, LegendError> {
            let area = entry.area();
            let hatch = entry
                .object::<AnyObject>()
                .map_or("", |object| object.hatch);
            let rectangle = Rectangle::new((area.xdescent, area.ydescent), area.width, area.height);
            let props = PatchProps::default().facecolor("red").hatch(hatch);
            Ok(vec![HandlerArtist::patch(rectangle, props)])
        }
    }

    /// The guide's `HandlerEllipse`: an ellipse styled like the original.
    #[derive(Debug)]
    struct HandlerEllipse;

    impl LegendHandler for HandlerEllipse {
        fn create_artists(
            &self,
            entry: &HandlerEntry<'_>,
        ) -> Result<Vec<HandlerArtist>, LegendError> {
            let area = entry.area();
            let center = (0.5 * area.width, 0.5 * area.height);
            Ok(vec![entry.patch_like_original(Ellipse::new(
                center,
                area.width,
                area.height,
            ))])
        }
    }

    /// The handle artists of the first entry of `legend` of type `T`.
    fn parts_of<T: crate::artist::Artist, R>(legend: &Legend, f: impl Fn(&T) -> R) -> Vec<R> {
        let state = legend.raw().figure_state().borrow();
        let parts = state
            .store
            .get_as::<LegendArtist>(legend.id())
            .map(|legend| legend.parts.clone())
            .unwrap_or_default();
        parts
            .into_iter()
            .filter_map(|id| state.store.get_as::<T>(id).map(&f))
            .collect()
    }

    #[test]
    fn objects_are_drawn_by_the_handler_of_their_type() {
        let ax = Figure::new().subplot();
        let object = || LegendHandle::object(AnyObject { hatch: "xx" });
        let plain = LegendOptions::default()
            .handles([object()])
            .labels(["My first handler"]);
        assert!(
            ax.legend(plain).expect("a legend").labels().is_empty(),
            "no handler"
        );
        let handled = LegendOptions::default()
            .handles([object()])
            .labels(["My first handler"])
            .handler(HandlerKey::of::<AnyObject>(), AnyObjectHandler);
        let legend = ax.legend(handled).expect("a legend");
        assert_eq!(legend.labels(), ["My first handler"]);
        let faces = parts_of::<PatchArtist<Rectangle>, _>(&legend, |patch| {
            (patch.style.facecolor.clone(), patch.style.hatch.clone())
        });
        assert!(faces.contains(&(Some(Color::from("red")), Some("xx".to_owned()))));
    }

    #[test]
    fn a_shape_handler_comes_before_the_patch_handler() {
        let ax = Figure::new().subplot();
        let circle = ax.add_patch_with(
            Circle::new((0.5, 0.5), 0.25),
            PatchProps::default().facecolor("green").label("circle"),
        );
        let options = LegendOptions::default()
            .handles([&circle])
            .handler(HandlerKey::of::<Circle>(), HandlerEllipse);
        let legend = ax.legend(options).expect("a legend");
        let faces = parts_of::<PatchArtist<Ellipse>, _>(&legend, |p| p.style.facecolor.clone());
        assert_eq!(faces, [Some(Color::from("green"))], "the circle's look");
        let handler = legend.legend_handler(&LegendHandle::from(&circle));
        assert_eq!(format!("{handler:?}"), "Some(HandlerEllipse)");
        let plain = ax
            .legend(LegendOptions::default().handles([&circle]))
            .expect("a legend");
        assert_eq!(parts_of::<PatchArtist<Ellipse>, _>(&plain, |_| ()), []);
        assert_eq!(
            format!("{:?}", plain.legend_handler(&LegendHandle::from(&circle))),
            "Some(HandlerPatch)"
        );
    }

    #[test]
    fn a_handle_of_its_own_handler_ignores_the_map() {
        let ax = Figure::new().subplot();
        let circle = ax.add_patch_with(Circle::new((0.5, 0.5), 0.25), PatchProps::default());
        let own = LegendHandle::from(&circle).with_handler(HandlerEllipse);
        let options = LegendOptions::default()
            .handles([own.clone()])
            .labels(["own"])
            .handler(HandlerKey::of::<Circle>(), HandlerPatch);
        let legend = ax.legend(options).expect("a legend");
        assert_eq!(
            parts_of::<PatchArtist<Ellipse>, _>(&legend, |_| ()).len(),
            1
        );
        assert_eq!(
            format!("{:?}", legend.legend_handler(&own)),
            "Some(HandlerEllipse)"
        );
    }

    #[test]
    fn the_default_map_can_be_replaced_and_extended() {
        let builtin = Legend::default_handler_map();
        assert_eq!(builtin.len(), 11);
        assert!(builtin.get(HandlerKey::of::<AnyObject>()).is_none());
        Legend::update_default_handler_map(
            &HandlerMap::new().with(HandlerKey::of::<AnyObject>(), AnyObjectHandler),
        );
        let ax = Figure::new().subplot();
        let legend = ax
            .legend(
                LegendOptions::default()
                    .handles([LegendHandle::object(AnyObject { hatch: "" })])
                    .labels(["registered once"]),
            )
            .expect("a legend");
        assert_eq!(legend.labels(), ["registered once"]);
        assert_eq!(legend.handler_map().len(), 12);
        Legend::set_default_handler_map(HandlerMap::new());
        assert!(Legend::default_handler_map().is_empty());
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        assert!(legend.legend_handler(&LegendHandle::from(&line)).is_none());
        Legend::set_default_handler_map(builtin);
        assert!(legend.legend_handler(&LegendHandle::from(&line)).is_some());
    }

    #[test]
    fn maps_update_like_dictionaries() {
        let mut map = HandlerMap::new().with(HandlerKey::Line, HandlerPatch);
        map.update(
            &HandlerMap::new()
                .with(HandlerKey::Line, HandlerLine2D)
                .with(HandlerKey::Tuple, HandlerPatch),
        );
        assert_eq!(map.len(), 2);
        assert_eq!(
            format!("{:?}", map.get(HandlerKey::Line)),
            "Some(HandlerLine2D)"
        );
        assert_eq!(HandlerKey::of::<Circle>(), HandlerKey::of::<Circle>());
        assert_ne!(HandlerKey::of::<Circle>(), HandlerKey::of::<Ellipse>());
        assert!(TypeKey::of::<Circle>().name().ends_with("Circle"));
    }
}
