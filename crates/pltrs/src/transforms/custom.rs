//! Maps of the plane defined by the program (matplotlib's own `Transform`
//! subclasses).

use std::fmt;
use std::rc::Rc;

use super::PointMap;

/// A shared, program-defined non-affine map of the plane, as
/// [`Transform::custom`](super::Transform::custom) wraps it. Two maps are
/// equal only when they are the same map.
#[derive(Clone)]
pub struct CustomMap(Rc<dyn PointMap>);

impl CustomMap {
    /// Shares `map`.
    pub fn new(map: impl PointMap + 'static) -> Self {
        Self(Rc::new(map))
    }

    /// The map.
    pub fn map(&self) -> Rc<dyn PointMap> {
        Rc::clone(&self.0)
    }
}

impl fmt::Debug for CustomMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CustomMap").field(&self.0).finish()
    }
}

impl PartialEq for CustomMap {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_core::Point;

    #[derive(Debug)]
    struct Swap;

    impl PointMap for Swap {
        fn forward(&self, p: Point) -> Point {
            Point::new(p.y, p.x)
        }
        fn inverse(&self, p: Point) -> Point {
            Point::new(p.y, p.x)
        }
    }

    #[test]
    fn maps_equal_only_themselves() {
        let map = CustomMap::new(Swap);
        assert_eq!(map, map.clone());
        assert_ne!(map, CustomMap::new(Swap));
        assert_eq!(
            map.map().forward(Point::new(1.0, 2.0)),
            Point::new(2.0, 1.0)
        );
    }
}
