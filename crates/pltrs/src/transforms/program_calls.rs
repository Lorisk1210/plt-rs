//! The program's maps called where matplotlib calls them, and pltrs'
//! own needs served from what they answered.
//!
//! A map of the program's (a function scale, a scale or transform class
//! of a host language) sees every call matplotlib makes to it: drawing
//! and measuring an axes call it once for every tick label measured or
//! drawn, twice for every tick line and gridline drawn (its path, then
//! its points), once for every spine measured or drawn. pltrs makes those
//! calls where matplotlib makes them (`axes::transform_sites`), with the
//! values matplotlib hands over, and lays out and draws the same parts its
//! own way meanwhile, needing the same values in other batches and at
//! other moments. That work runs [`quietly`]: a map of the program's
//! answers it from what the program answered for the same values since
//! the last layout began ([`forget`]), and is called only for a value it
//! never answered.
//!
//! The answers are kept per map (by its address, the map kept alive with
//! them) and per value (its bits), for the thread laying out and drawing.
//! A map of the plane of the program's (a projection's transform) is
//! made anew whenever its transform is resolved: its answers are kept by
//! the key naming the code it runs ([`PointMap::program_key`]), per point
//! and per path.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use pltrs_core::{Path, Point};

use super::{AxisMap, PointMap};

/// How many values of one map are kept at most: a map answering more
/// (a program animating a large dataset) starts afresh.
const KEPT_PER_MAP: usize = 1 << 16;

/// What one map of the program's answered, by the bits of the value it
/// was given.
struct MapAnswers {
    /// The map, kept alive so that its address names it.
    map: Rc<dyn AxisMap>,
    values: HashMap<u64, f64>,
}

/// What one map of the plane of the program's answered: points by the
/// bits of their coordinates, and paths.
struct PlaneAnswers {
    /// A map of the key, held so that the key keeps naming its code.
    _alive: Rc<dyn PointMap>,
    points: HashMap<[u64; 2], Point>,
    /// The paths given and what the map made of them.
    paths: Vec<(Path, Path)>,
}

thread_local! {
    /// How many [`quietly`] scopes the thread is in.
    static QUIET: Cell<usize> = const { Cell::new(0) };
    /// The answers of the program's maps, by map address.
    static ANSWERS: RefCell<HashMap<usize, MapAnswers>> = RefCell::new(HashMap::new());
    /// The answers of the program's maps of the plane, by key.
    static PLANE_ANSWERS: RefCell<HashMap<usize, PlaneAnswers>> = RefCell::new(HashMap::new());
}

/// The address naming `map` among the answers.
fn address(map: &Rc<dyn AxisMap>) -> usize {
    Rc::as_ptr(map).cast::<()>().addr()
}

/// Restores the quiet depth it was made with when dropped.
struct QuietDepth(usize);

impl Drop for QuietDepth {
    fn drop(&mut self) {
        QUIET.set(self.0);
    }
}

/// Runs `work`, pltrs' own computation of values matplotlib computes
/// elsewhere, with the program's maps answering from what they answered
/// before (see the module documentation).
pub(crate) fn quietly<R>(work: impl FnOnce() -> R) -> R {
    let depth = QUIET.get();
    let _restore = QuietDepth(depth);
    QUIET.set(depth + 1);
    work()
}

/// Runs `work`, calls matplotlib makes, with the program's maps called
/// even inside a [`quietly`] scope.
pub(crate) fn observed<R>(work: impl FnOnce() -> R) -> R {
    let _restore = QuietDepth(QUIET.get());
    QUIET.set(0);
    work()
}

/// Forgets every answer, as a layout begins: what the program's maps
/// answer from then on is what that layout and the draw after it use.
pub(crate) fn forget() {
    // Dropped after the borrow ends: a map's drop may run the program's
    // code.
    let answers = ANSWERS.with_borrow_mut(std::mem::take);
    drop(answers);
    let answers = PLANE_ANSWERS.with_borrow_mut(std::mem::take);
    drop(answers);
}

/// The answers of `map` for every value of `values`, when it answered
/// each before.
fn known(map: &Rc<dyn AxisMap>, values: &[f64]) -> Option<Vec<f64>> {
    ANSWERS.with_borrow(|answers| {
        let kept = answers.get(&address(map))?;
        (values.iter())
            .map(|value| kept.values.get(&value.to_bits()).copied())
            .collect()
    })
}

/// Keeps what `map` answered: `mapped[i]` for `given[i]`.
fn keep(map: &Rc<dyn AxisMap>, given: &[f64], mapped: &[f64]) {
    ANSWERS.with_borrow_mut(|answers| {
        let kept = answers.entry(address(map)).or_insert_with(|| MapAnswers {
            map: Rc::clone(map),
            values: HashMap::new(),
        });
        if kept.values.len() + given.len() > KEPT_PER_MAP {
            kept.values.clear();
        }
        for (value, answer) in given.iter().zip(mapped) {
            kept.values.insert(value.to_bits(), *answer);
        }
        debug_assert!(
            std::ptr::addr_eq(Rc::as_ptr(&kept.map), Rc::as_ptr(map)),
            "the address names the map"
        );
    });
}

/// Maps every value of `values` through `map` in place, as
/// [`AxisMap::forward_all`] does: a map of the program's called once for
/// all of them, or answering from what it answered before while pltrs'
/// own work runs [`quietly`].
pub(crate) fn forward_all(map: &Rc<dyn AxisMap>, values: &mut [f64]) {
    if !map.calls_program() {
        map.forward_all(values);
        return;
    }
    if QUIET.get() > 0
        && let Some(answers) = known(map, values)
    {
        values.copy_from_slice(&answers);
        return;
    }
    let given = values.to_vec();
    map.forward_all(values);
    keep(map, &given, values);
}

/// Maps every value of `values` through `map` in place, as matplotlib
/// maps the masked coordinates of a mesh (see
/// [`AxisMap::forward_masked_row`]), the program's answers kept as
/// [`forward_all`] keeps them.
// Kept out of the hot paths mapping every path drawn (maps of the
// program's are rare).
#[inline(never)]
pub(crate) fn forward_masked_row(map: &Rc<dyn AxisMap>, values: &mut [f64]) {
    if !map.calls_program() {
        map.forward_all(values);
        return;
    }
    let given = values.to_vec();
    map.forward_masked_row(values);
    keep(map, &given, values);
}

/// One value through `map`, as [`AxisMap::forward`] maps it (see
/// [`forward_all`]).
pub(crate) fn forward(map: &Rc<dyn AxisMap>, value: f64) -> f64 {
    if !map.calls_program() {
        return map.forward(value);
    }
    if QUIET.get() > 0
        && let Some(answers) = known(map, &[value])
    {
        return answers[0];
    }
    let answer = map.forward(value);
    keep(map, &[value], &[answer]);
    answer
}

/// The key of a map of the plane of the program's whose answers are kept.
fn plane_key(map: &Rc<dyn PointMap>) -> Option<usize> {
    map.program_key().filter(|_| map.calls_program())
}

/// The bits of a point, which name it among the answers.
fn point_bits(point: Point) -> [u64; 2] {
    [point.x.to_bits(), point.y.to_bits()]
}

/// Runs `with` on the answers kept for `key` (made for `map` if none).
fn plane_answers<R>(
    key: usize,
    map: &Rc<dyn PointMap>,
    with: impl FnOnce(&mut PlaneAnswers) -> R,
) -> R {
    PLANE_ANSWERS.with_borrow_mut(|answers| {
        let kept = answers.entry(key).or_insert_with(|| PlaneAnswers {
            _alive: Rc::clone(map),
            points: HashMap::new(),
            paths: Vec::new(),
        });
        with(kept)
    })
}

/// The answers kept for `key`, if any, read by `read`.
fn known_plane<R>(key: usize, read: impl FnOnce(&PlaneAnswers) -> Option<R>) -> Option<R> {
    if QUIET.get() == 0 {
        return None;
    }
    PLANE_ANSWERS.with_borrow(|answers| answers.get(&key).and_then(read))
}

/// Maps every point of `points` through `map` in place, as
/// [`PointMap::forward_all`] does: a map of the program's called once for
/// all of them, or answering from what it answered before while pltrs'
/// own work runs [`quietly`].
// Kept out of the hot paths mapping every path drawn (maps of the
// program's are rare).
#[inline(never)]
pub(crate) fn forward_points(map: &Rc<dyn PointMap>, points: &mut [Point]) {
    let Some(key) = plane_key(map) else {
        map.forward_all(points);
        return;
    };
    let known = known_plane(key, |kept| {
        (points.iter())
            .map(|&point| kept.points.get(&point_bits(point)).copied())
            .collect::<Option<Vec<Point>>>()
    });
    if let Some(answers) = known {
        points.copy_from_slice(&answers);
        return;
    }
    let given = points.to_vec();
    map.forward_all(points);
    keep_points(key, map, &given, points);
}

/// One point through `map`, as [`PointMap::forward`] maps it (see
/// [`forward_points`]).
// Kept out of the hot paths mapping every path drawn (maps of the
// program's are rare).
#[inline(never)]
pub(crate) fn forward_point(map: &Rc<dyn PointMap>, point: Point) -> Point {
    let Some(key) = plane_key(map) else {
        return map.forward(point);
    };
    if let Some(answer) = known_plane(key, |kept| kept.points.get(&point_bits(point)).copied()) {
        return answer;
    }
    let answer = map.forward(point);
    keep_points(key, map, &[point], &[answer]);
    answer
}

/// Keeps what the map of `key` answered: `mapped[i]` for `given[i]`.
fn keep_points(key: usize, map: &Rc<dyn PointMap>, given: &[Point], mapped: &[Point]) {
    plane_answers(key, map, |kept| {
        if kept.points.len() + given.len() > KEPT_PER_MAP {
            kept.points.clear();
        }
        for (&point, &answer) in given.iter().zip(mapped) {
            kept.points.insert(point_bits(point), answer);
        }
    });
}

/// `path` through `map`, as [`PointMap::transform_path`] maps it (see
/// [`forward_points`]).
// Kept out of the hot paths mapping every path drawn (maps of the
// program's are rare).
#[inline(never)]
pub(crate) fn transform_path(map: &Rc<dyn PointMap>, path: &Path) -> Path {
    let Some(key) = plane_key(map) else {
        return map.transform_path(path);
    };
    let known = known_plane(key, |kept| {
        (kept.paths.iter())
            .find(|(given, _)| same_path(given, path))
            .map(|(_, mapped)| mapped.clone())
    });
    if let Some(mapped) = known {
        return mapped;
    }
    let mapped = map.transform_path(path);
    plane_answers(key, map, |kept| {
        if kept.paths.len() >= KEPT_PATHS_PER_MAP {
            kept.paths.clear();
        }
        kept.paths.push((path.clone(), mapped.clone()));
    });
    mapped
}

/// How many paths of one map of the plane are kept at most.
const KEPT_PATHS_PER_MAP: usize = 1 << 10;

/// Whether two paths have the same vertices (bit for bit), codes and
/// interpolation steps.
pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    a.codes() == b.codes()
        && a.interpolation_steps() == b.interpolation_steps()
        && a.vertices().len() == b.vertices().len()
        && (a.vertices().iter().zip(b.vertices())).all(|(&p, &q)| point_bits(p) == point_bits(q))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Doubles values, recording every call.
    #[derive(Debug, Default)]
    struct Counted {
        calls: RefCell<Vec<Vec<f64>>>,
    }

    impl AxisMap for Counted {
        fn forward(&self, value: f64) -> f64 {
            self.calls.borrow_mut().push(vec![value]);
            2.0 * value
        }

        fn inverse(&self, value: f64) -> f64 {
            value / 2.0
        }

        fn forward_all(&self, values: &mut [f64]) {
            self.calls.borrow_mut().push(values.to_vec());
            for v in values.iter_mut() {
                *v *= 2.0;
            }
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    fn counted() -> (Rc<Counted>, Rc<dyn AxisMap>) {
        let map = Rc::new(Counted::default());
        let shared: Rc<dyn AxisMap> = Rc::clone(&map) as Rc<dyn AxisMap>;
        (map, shared)
    }

    #[test]
    fn quiet_work_is_answered_from_earlier_calls() {
        forget();
        let (map, shared) = counted();
        let mut values = [1.0, 2.0];
        forward_all(&shared, &mut values);
        assert_eq!(values, [2.0, 4.0]);
        let quiet = quietly(|| {
            let mut again = [2.0, 1.0];
            forward_all(&shared, &mut again);
            (again, forward(&shared, 1.0))
        });
        assert_eq!(quiet, ([4.0, 2.0], 2.0));
        assert_eq!(
            *map.calls.borrow(),
            [vec![1.0, 2.0]],
            "answered, not called"
        );
        forget();
    }

    /// Shifts points, counting its calls, as a map of the plane of the
    /// program's named by `key`.
    #[derive(Debug)]
    struct Plane {
        key: Option<usize>,
        calls: std::cell::Cell<usize>,
    }

    impl PointMap for Plane {
        fn forward(&self, point: Point) -> Point {
            self.calls.set(self.calls.get() + 1);
            Point::new(point.x + 1.0, point.y)
        }

        fn inverse(&self, point: Point) -> Point {
            Point::new(point.x - 1.0, point.y)
        }

        fn forward_all(&self, points: &mut [Point]) {
            self.calls.set(self.calls.get() + 1);
            for p in points {
                p.x += 1.0;
            }
        }

        fn calls_program(&self) -> bool {
            true
        }

        fn program_key(&self) -> Option<usize> {
            self.key
        }
    }

    fn plane(key: Option<usize>) -> (Rc<Plane>, Rc<dyn PointMap>) {
        let map = Rc::new(Plane {
            key,
            calls: std::cell::Cell::new(0),
        });
        let shared: Rc<dyn PointMap> = Rc::clone(&map) as Rc<dyn PointMap>;
        (map, shared)
    }

    #[test]
    fn maps_of_the_plane_answer_by_key_across_their_copies() {
        forget();
        let (first, shared) = plane(Some(7));
        let mut points = [Point::new(0.0, 1.0), Point::new(2.0, 3.0)];
        forward_points(&shared, &mut points);
        assert_eq!(points[1], Point::new(3.0, 3.0));
        let path = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(2.0, 3.0)]);
        let mapped = transform_path(&shared, &path);
        assert_eq!(first.calls.get(), 2);
        // A copy made by another resolution names the same code.
        let (copy, again) = plane(Some(7));
        quietly(|| {
            assert_eq!(
                forward_point(&again, Point::new(2.0, 3.0)),
                Point::new(3.0, 3.0)
            );
            assert!(same_path(&transform_path(&again, &path), &mapped));
        });
        assert_eq!(copy.calls.get(), 0, "answered, not called");
        observed(|| forward_point(&again, Point::new(2.0, 3.0)));
        assert_eq!(copy.calls.get(), 1, "matplotlib's call is made");
        // A map without a key is called every time.
        let (unkept, unnamed) = plane(None);
        quietly(|| {
            forward_point(&unnamed, Point::new(2.0, 3.0));
            forward_point(&unnamed, Point::new(2.0, 3.0));
        });
        assert_eq!(unkept.calls.get(), 2);
        forget();
    }

    #[test]
    fn values_never_answered_and_observed_calls_call_the_program() {
        forget();
        let (map, shared) = counted();
        quietly(|| {
            assert_eq!(forward(&shared, 3.0), 6.0, "never answered");
            assert_eq!(forward(&shared, 3.0), 6.0, "answered since");
            observed(|| forward(&shared, 3.0));
        });
        assert_eq!(*map.calls.borrow(), [vec![3.0], vec![3.0]]);
        forget();
        quietly(|| forward(&shared, 3.0));
        assert_eq!(map.calls.borrow().len(), 3, "forgotten");
        forget();
    }
}
