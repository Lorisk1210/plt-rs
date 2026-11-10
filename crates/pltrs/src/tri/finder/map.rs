//! The trapezoid map of matplotlib's `TrapezoidMapTriFinder` (`_tri.cpp`)
//! and its search tree, step for step: the trapezoids an edge crosses are
//! replaced by new ones above and below it, and their leaves in the tree
//! by nodes that test the edge's end points and the edge itself.
//!
//! Points, edges, trapezoids and nodes live in arenas and refer to each
//! other by index; the identity comparisons of the C++ pointers are
//! comparisons of indices. Nodes and trapezoids a step replaces stay in
//! their arenas unused.

use pltrs_core::Point;

/// An edge of the map, directed from `left` to `right` (lexicographically
/// larger), with the triangles and opposite points on either side.
#[derive(Clone, Copy, Debug)]
pub(super) struct Edge {
    /// The index of the left end point.
    pub(super) left: usize,
    /// The index of the right end point.
    pub(super) right: usize,
    /// The triangle below the edge, if any.
    pub(super) triangle_below: Option<usize>,
    /// The triangle above the edge, if any.
    pub(super) triangle_above: Option<usize>,
    /// The third point of the triangle below.
    pub(super) point_below: Option<usize>,
    /// The third point of the triangle above.
    pub(super) point_above: Option<usize>,
}

/// A trapezoid between two edges, bounded by vertical lines through two
/// points, with its (up to four) neighbours.
#[derive(Clone, Copy, Debug)]
struct Trapezoid {
    left: usize,
    right: usize,
    below: usize,
    above: usize,
    lower_left: Option<usize>,
    lower_right: Option<usize>,
    upper_left: Option<usize>,
    upper_right: Option<usize>,
    /// The leaf of the search tree holding this trapezoid.
    node: usize,
}

/// A node of the search tree.
#[derive(Clone, Copy, Debug)]
enum Node {
    /// Left or right of a point.
    X {
        point: usize,
        left: usize,
        right: usize,
    },
    /// Below or above an edge.
    Y {
        edge: usize,
        below: usize,
        above: usize,
    },
    /// A trapezoid.
    Leaf(usize),
}

/// The trapezoid map of a triangulation's edges.
#[derive(Clone, Debug)]
pub(super) struct TrapezoidMap {
    points: Vec<Point>,
    /// The triangle a search ending on each point returns.
    point_triangles: Vec<Option<usize>>,
    edges: Vec<Edge>,
    trapezoids: Vec<Trapezoid>,
    nodes: Vec<Node>,
    parents: Vec<Vec<usize>>,
    root: usize,
}

/// The trapezoids replacing one an edge crosses.
#[derive(Clone, Copy, Debug)]
struct Replacements {
    /// Left of the edge's left end, in the first trapezoid crossed.
    left: Option<usize>,
    below: usize,
    above: usize,
    /// Right of the edge's right end, in the last trapezoid crossed.
    right: Option<usize>,
}

/// Inserting an edge failed: the triangulation is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Invalid;

impl TrapezoidMap {
    /// The map of `edges` over `points`, inserted in their order after the
    /// first two, the bottom and top of the enclosing rectangle whose
    /// lower corners are the points `corners` and `corners + 1`.
    pub(super) fn new(
        points: Vec<Point>,
        point_triangles: Vec<Option<usize>>,
        edges: Vec<Edge>,
        corners: usize,
    ) -> Result<Self, Invalid> {
        let mut map = Self {
            points,
            point_triangles,
            edges,
            trapezoids: Vec::new(),
            nodes: Vec::new(),
            parents: Vec::new(),
            root: 0,
        };
        let first = map.new_trapezoid(corners, corners + 1, 0, 1);
        map.root = map.new_leaf(first);
        for edge in 2..map.edges.len() {
            map.add_edge(edge)?;
        }
        Ok(map)
    }

    /// The triangle a search for `xy` ends on (matplotlib's `find_one`).
    pub(super) fn find(&self, xy: Point) -> Option<usize> {
        match self.nodes[self.search_point(xy)] {
            Node::X { point, .. } => self.point_triangles[point],
            Node::Y { edge, .. } => {
                let edge = &self.edges[edge];
                edge.triangle_above.or(edge.triangle_below)
            }
            Node::Leaf(trapezoid) => self.edges[self.trapezoids[trapezoid].below].triangle_above,
        }
    }

    /// The node a search for `xy` stops at: a point it equals, an edge it
    /// lies on, or a trapezoid.
    #[expect(clippy::float_cmp, reason = "matplotlib's exact point test")]
    fn search_point(&self, xy: Point) -> usize {
        let mut node = self.root;
        loop {
            node = match self.nodes[node] {
                Node::X { point, left, right } => {
                    let p = self.points[point];
                    if xy.x == p.x && xy.y == p.y {
                        return node;
                    }
                    if is_right_of(xy, p) { right } else { left }
                }
                Node::Y { edge, below, above } => match self.orientation(edge, xy) {
                    0 => return node,
                    side if side < 0 => above,
                    _ => below,
                },
                Node::Leaf(_) => return node,
            };
        }
    }

    /// The trapezoid containing the left end of `edge`, on the side the
    /// edge leaves it (matplotlib's `Node::search(const Edge&)`).
    #[expect(clippy::float_cmp, reason = "matplotlib's exact slope tests")]
    fn search_edge(&self, edge: usize) -> Option<usize> {
        let e = self.edges[edge];
        let mut node = self.root;
        loop {
            node = match self.nodes[node] {
                Node::X { point, left, right } => {
                    if e.left == point || is_right_of(self.points[e.left], self.points[point]) {
                        right
                    } else {
                        left
                    }
                }
                Node::Y {
                    edge: other,
                    below,
                    above,
                } => {
                    let o = self.edges[other];
                    let (slope, other_slope) = (self.slope(edge), self.slope(other));
                    if e.left == o.left || e.right == o.right {
                        if slope == other_slope {
                            if o.triangle_above == e.triangle_below {
                                above
                            } else if o.triangle_below == e.triangle_above {
                                below
                            } else {
                                return None;
                            }
                        } else if (slope > other_slope) == (e.left == o.left) {
                            above
                        } else {
                            below
                        }
                    } else {
                        let mut side = self.orientation(other, self.points[e.left]);
                        if side == 0 {
                            if o.point_above.is_some_and(|p| has_point(&e, p)) {
                                side = -1;
                            } else if o.point_below.is_some_and(|p| has_point(&e, p)) {
                                side = 1;
                            } else {
                                return None;
                            }
                        }
                        if side < 0 { above } else { below }
                    }
                }
                Node::Leaf(trapezoid) => return Some(trapezoid),
            };
        }
    }

    /// The trapezoids `edge` crosses, from left to right.
    fn trapezoids_crossed(&self, edge: usize) -> Option<Vec<usize>> {
        let e = self.edges[edge];
        let mut trapezoid = self.search_edge(edge)?;
        let mut crossed = vec![trapezoid];
        loop {
            let right = self.trapezoids[trapezoid].right;
            if !is_right_of(self.points[e.right], self.points[right]) {
                return Some(crossed);
            }
            let mut side = self.orientation(edge, self.points[right]);
            if side == 0 {
                if e.point_below == Some(right) {
                    side = 1;
                } else if e.point_above == Some(right) {
                    side = -1;
                } else {
                    return None;
                }
            }
            let t = &self.trapezoids[trapezoid];
            trapezoid = if side < 0 {
                t.lower_right
            } else {
                t.upper_right
            }?;
            crossed.push(trapezoid);
            // Each trapezoid lies right of the last; more steps than
            // trapezoids would be a cycle of a broken map.
            if crossed.len() > self.trapezoids.len() {
                return None;
            }
        }
    }

    /// Inserts `edge`, replacing each trapezoid it crosses with up to four
    /// (matplotlib's `add_edge_to_tree`).
    fn add_edge(&mut self, edge: usize) -> Result<(), Invalid> {
        let crossed = self.trapezoids_crossed(edge).ok_or(Invalid)?;
        let Edge {
            left: p, right: q, ..
        } = self.edges[edge];
        let mut left_old = None;
        let mut left_below: Option<usize> = None;
        let mut left_above: Option<usize> = None;
        let count = crossed.len();
        for (i, &old) in crossed.iter().enumerate() {
            let o = self.trapezoids[old];
            let (start, end) = (i == 0, i == count - 1);
            let have_left = start && p != o.left;
            let have_right = end && q != o.right;
            let left = have_left.then(|| self.new_trapezoid(o.left, p, o.below, o.above));
            let right = have_right.then(|| self.new_trapezoid(q, o.right, o.below, o.above));
            let right_end = if end { q } else { o.right };
            let (below, above) = if start {
                let below = self.new_trapezoid(p, right_end, o.below, edge);
                let above = self.new_trapezoid(p, right_end, edge, o.above);
                if let Some(left) = left {
                    self.set_lower_left(left, o.lower_left);
                    self.set_upper_left(left, o.upper_left);
                    self.set_lower_right(left, Some(below));
                    self.set_upper_right(left, Some(above));
                } else {
                    self.set_lower_left(below, o.lower_left);
                    self.set_upper_left(above, o.upper_left);
                }
                (below, above)
            } else {
                let previous_below = left_below.ok_or(Invalid)?;
                let previous_above = left_above.ok_or(Invalid)?;
                let below = if self.trapezoids[previous_below].below == o.below {
                    self.trapezoids[previous_below].right = right_end;
                    previous_below
                } else {
                    self.new_trapezoid(o.left, right_end, o.below, edge)
                };
                let above = if self.trapezoids[previous_above].above == o.above {
                    self.trapezoids[previous_above].right = right_end;
                    previous_above
                } else {
                    self.new_trapezoid(o.left, right_end, edge, o.above)
                };
                if below != previous_below {
                    self.set_upper_left(below, Some(previous_below));
                    let lower_left = if o.lower_left == left_old {
                        Some(previous_below)
                    } else {
                        o.lower_left
                    };
                    self.set_lower_left(below, lower_left);
                }
                if above != previous_above {
                    self.set_lower_left(above, Some(previous_above));
                    let upper_left = if o.upper_left == left_old {
                        Some(previous_above)
                    } else {
                        o.upper_left
                    };
                    self.set_upper_left(above, upper_left);
                }
                (below, above)
            };
            if let Some(right) = right {
                self.set_lower_right(right, o.lower_right);
                self.set_upper_right(right, o.upper_right);
                self.set_lower_right(below, Some(right));
                self.set_upper_right(above, Some(right));
            } else {
                self.set_lower_right(below, o.lower_right);
                self.set_upper_right(above, o.upper_right);
            }
            let replacements = Replacements {
                left,
                below,
                above,
                right,
            };
            self.replace_crossed(old, edge, replacements, [left_below, left_above]);
            if !end {
                left_old = Some(old);
                left_below = Some(below);
                left_above = Some(above);
            }
        }
        Ok(())
    }

    /// Replaces the leaf of the crossed trapezoid `old` with the nodes of
    /// its replacements: below and above `edge` between its end points,
    /// split off by the end points from those left and right of them.
    /// Replacements carried over from the previous trapezoid (`previous`,
    /// below and above) keep their leaves.
    fn replace_crossed(
        &mut self,
        old: usize,
        edge: usize,
        Replacements {
            left,
            below,
            above,
            right,
        }: Replacements,
        previous: [Option<usize>; 2],
    ) {
        let leaf = |map: &mut Self, trapezoid: usize, previous: Option<usize>| {
            if previous == Some(trapezoid) {
                map.trapezoids[trapezoid].node
            } else {
                map.new_leaf(trapezoid)
            }
        };
        let below_node = leaf(self, below, previous[0]);
        let above_node = leaf(self, above, previous[1]);
        let mut top = self.new_node(Node::Y {
            edge,
            below: below_node,
            above: above_node,
        });
        let Edge {
            left: p, right: q, ..
        } = self.edges[edge];
        if let Some(right) = right {
            let right = self.new_leaf(right);
            top = self.new_node(Node::X {
                point: q,
                left: top,
                right,
            });
        }
        if let Some(left) = left {
            let left = self.new_leaf(left);
            top = self.new_node(Node::X {
                point: p,
                left,
                right: top,
            });
        }
        let old_node = self.trapezoids[old].node;
        if old_node == self.root {
            self.root = top;
        } else {
            self.replace_with(old_node, top);
        }
    }

    fn new_trapezoid(&mut self, left: usize, right: usize, below: usize, above: usize) -> usize {
        self.trapezoids.push(Trapezoid {
            left,
            right,
            below,
            above,
            lower_left: None,
            lower_right: None,
            upper_left: None,
            upper_right: None,
            node: usize::MAX,
        });
        self.trapezoids.len() - 1
    }

    fn new_leaf(&mut self, trapezoid: usize) -> usize {
        let node = self.new_node(Node::Leaf(trapezoid));
        self.trapezoids[trapezoid].node = node;
        node
    }

    /// Adds `node`, the parent of its children.
    fn new_node(&mut self, node: Node) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        self.parents.push(Vec::new());
        match node {
            Node::X { left, right, .. } => {
                self.parents[left].push(index);
                self.parents[right].push(index);
            }
            Node::Y { below, above, .. } => {
                self.parents[below].push(index);
                self.parents[above].push(index);
            }
            Node::Leaf(_) => {}
        }
        index
    }

    /// Puts `new` in the place of `old` in each of its parents.
    fn replace_with(&mut self, old: usize, new: usize) {
        while let Some(&parent) = self.parents[old].first() {
            match &mut self.nodes[parent] {
                Node::X { left, right, .. } => {
                    if *left == old {
                        *left = new;
                    } else {
                        *right = new;
                    }
                }
                Node::Y { below, above, .. } => {
                    if *below == old {
                        *below = new;
                    } else {
                        *above = new;
                    }
                }
                Node::Leaf(_) => {}
            }
            self.parents[old].remove(0);
            self.parents[new].push(parent);
        }
    }

    fn set_lower_left(&mut self, trapezoid: usize, other: Option<usize>) {
        self.trapezoids[trapezoid].lower_left = other;
        if let Some(other) = other {
            self.trapezoids[other].lower_right = Some(trapezoid);
        }
    }

    fn set_lower_right(&mut self, trapezoid: usize, other: Option<usize>) {
        self.trapezoids[trapezoid].lower_right = other;
        if let Some(other) = other {
            self.trapezoids[other].lower_left = Some(trapezoid);
        }
    }

    fn set_upper_left(&mut self, trapezoid: usize, other: Option<usize>) {
        self.trapezoids[trapezoid].upper_left = other;
        if let Some(other) = other {
            self.trapezoids[other].upper_right = Some(trapezoid);
        }
    }

    fn set_upper_right(&mut self, trapezoid: usize, other: Option<usize>) {
        self.trapezoids[trapezoid].upper_right = other;
        if let Some(other) = other {
            self.trapezoids[other].upper_left = Some(trapezoid);
        }
    }

    /// Which side of `edge` `xy` lies on: `1` below, `-1` above, `0` on
    /// its line (matplotlib's `Edge::get_point_orientation`).
    fn orientation(&self, edge: usize, xy: Point) -> i8 {
        let (left, right) = self.ends(edge);
        let cross = (xy.x - left.x) * (right.y - left.y) - (xy.y - left.y) * (right.x - left.x);
        if cross > 0.0 {
            1
        } else if cross < 0.0 {
            -1
        } else {
            0
        }
    }

    fn slope(&self, edge: usize) -> f64 {
        let (left, right) = self.ends(edge);
        (right.y - left.y) / (right.x - left.x)
    }

    fn ends(&self, edge: usize) -> (Point, Point) {
        let e = &self.edges[edge];
        (self.points[e.left], self.points[e.right])
    }
}

/// matplotlib's `XY::is_right_of`: by x, then by y.
#[expect(clippy::float_cmp, reason = "matplotlib's exact order")]
pub(super) fn is_right_of(p: Point, other: Point) -> bool {
    if p.x == other.x {
        p.y > other.y
    } else {
        p.x > other.x
    }
}

fn has_point(edge: &Edge, point: usize) -> bool {
    edge.left == point || edge.right == point
}
