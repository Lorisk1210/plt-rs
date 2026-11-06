//! A Compose table: its sequences of keysyms as a trie, with the rules
//! libxkbcommon adds them by.

/// The most nodes a table holds; sequences beyond are ignored (one per
/// keysym of a sequence: the system's tables have about 10,000).
const MAX_NODES: usize = 1 << 20;

/// What a sequence types: its text, its keysym, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// The text (`"é"`), if the production gives one.
    pub(crate) text: Option<String>,
    /// The keysym (`eacute`), if the production gives one.
    pub(crate) keysym: Option<u32>,
}

impl Outcome {
    /// The text typed: the production's, else its keysym's character.
    pub(crate) fn typed(&self) -> Option<String> {
        self.text.clone().or_else(|| {
            self.keysym
                .and_then(crate::keysym::to_char)
                .map(String::from)
        })
    }
}

/// A node of the trie.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    /// Sequences go on: the next keysyms and their nodes, sorted by keysym.
    Branch(Vec<(u32, usize)>),
    /// A sequence ends here.
    Leaf(Outcome),
}

/// Where feeding a keysym leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step<'a> {
    /// No sequence goes on with the keysym.
    Nowhere,
    /// Sequences go on: the node reached.
    Branch(usize),
    /// A sequence is complete.
    Leaf(&'a Outcome),
}

/// What adding a production did, which libxkbcommon warns about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Added {
    /// A new sequence.
    New,
    /// It replaced a sequence: the same one, or one it starts with.
    Overriding,
    /// It was left out: it starts another sequence, or the table is
    /// full.
    Skipped,
}

/// The sequences of a Compose file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComposeTable {
    /// The trie; the root, a branch, comes first.
    nodes: Vec<Node>,
}

impl Default for ComposeTable {
    fn default() -> Self {
        Self {
            nodes: vec![Node::Branch(Vec::new())],
        }
    }
}

impl ComposeTable {
    /// The node every sequence starts from.
    pub(crate) const ROOT: usize = 0;

    /// Adds the production `sequence : outcome` as libxkbcommon does: a
    /// sequence extending one already there replaces it, one that starts
    /// a longer sequence already there is left out, and a later
    /// production of a sequence replaces the earlier.
    pub(crate) fn add(&mut self, sequence: &[u32], outcome: Outcome) -> Added {
        let Some((&last, prefix)) = sequence.split_last() else {
            return Added::Skipped;
        };
        if self.nodes.len() + sequence.len() >= MAX_NODES {
            return Added::Skipped;
        }
        let mut added = Added::New;
        let mut node = Self::ROOT;
        for &keysym in prefix {
            node = match self.child(node, keysym) {
                Some(child) => {
                    if matches!(self.nodes[child], Node::Leaf(_)) {
                        self.nodes[child] = Node::Branch(Vec::new());
                        added = Added::Overriding;
                    }
                    child
                }
                None => self.push_child(node, keysym, Node::Branch(Vec::new())),
            };
        }
        match self.child(node, last) {
            None => {
                self.push_child(node, last, Node::Leaf(outcome));
                added
            }
            Some(child) => match &self.nodes[child] {
                Node::Branch(next) if !next.is_empty() => Added::Skipped,
                Node::Leaf(old) if *old == outcome => Added::Skipped,
                Node::Leaf(_) => {
                    self.nodes[child] = Node::Leaf(outcome);
                    Added::Overriding
                }
                Node::Branch(_) => {
                    self.nodes[child] = Node::Leaf(outcome);
                    added
                }
            },
        }
    }

    /// Where `keysym` leads from the branch `node`.
    pub(crate) fn step(&self, node: usize, keysym: u32) -> Step<'_> {
        match self
            .child(node, keysym)
            .map(|child| (child, &self.nodes[child]))
        {
            None => Step::Nowhere,
            Some((child, Node::Branch(_))) => Step::Branch(child),
            Some((_, Node::Leaf(outcome))) => Step::Leaf(outcome),
        }
    }

    /// Whether the table has no sequence.
    pub(crate) fn is_empty(&self) -> bool {
        matches!(&self.nodes[Self::ROOT], Node::Branch(children) if children.is_empty())
    }

    /// The child of branch `node` for `keysym`.
    fn child(&self, node: usize, keysym: u32) -> Option<usize> {
        let Some(Node::Branch(children)) = self.nodes.get(node) else {
            return None;
        };
        children
            .binary_search_by_key(&keysym, |&(key, _)| key)
            .ok()
            .map(|index| children[index].1)
    }

    /// Adds `child` to branch `node` under `keysym`; returns its index.
    fn push_child(&mut self, node: usize, keysym: u32, child: Node) -> usize {
        let index = self.nodes.len();
        self.nodes.push(child);
        if let Node::Branch(children) = &mut self.nodes[node] {
            let at = children.partition_point(|&(key, _)| key < keysym);
            children.insert(at, (keysym, index));
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEAD_ACUTE: u32 = 0xfe51;
    const MULTI_KEY: u32 = 0xff20;

    fn text(text: &str) -> Outcome {
        Outcome {
            text: Some(text.to_owned()),
            keysym: None,
        }
    }

    fn leaf(table: &ComposeTable, sequence: &[u32]) -> Option<Outcome> {
        let (&last, prefix) = sequence.split_last()?;
        let mut node = ComposeTable::ROOT;
        for &keysym in prefix {
            match table.step(node, keysym) {
                Step::Branch(next) => node = next,
                _ => return None,
            }
        }
        match table.step(node, last) {
            Step::Leaf(outcome) => Some(outcome.clone()),
            _ => None,
        }
    }

    #[test]
    fn sequences_are_found_keysym_by_keysym() {
        let mut table = ComposeTable::default();
        assert!(table.is_empty());
        assert_eq!(table.add(&[DEAD_ACUTE, 0x65], text("é")), Added::New);
        assert_eq!(table.add(&[DEAD_ACUTE, 0x61], text("á")), Added::New);
        assert_eq!(table.add(&[MULTI_KEY, 0x6f, 0x63], text("©")), Added::New);
        assert!(!table.is_empty());
        assert_eq!(leaf(&table, &[DEAD_ACUTE, 0x65]), Some(text("é")));
        assert_eq!(leaf(&table, &[DEAD_ACUTE, 0x61]), Some(text("á")));
        assert_eq!(leaf(&table, &[MULTI_KEY, 0x6f, 0x63]), Some(text("©")));
        assert!(matches!(
            table.step(ComposeTable::ROOT, DEAD_ACUTE),
            Step::Branch(_)
        ));
        assert_eq!(table.step(ComposeTable::ROOT, 0x65), Step::Nowhere);
        assert_eq!(table.add(&[], text("x")), Added::Skipped);
    }

    #[test]
    fn later_productions_win_as_in_libxkbcommon() {
        let mut table = ComposeTable::default();
        table.add(&[DEAD_ACUTE, 0x65], text("é"));
        assert_eq!(table.add(&[DEAD_ACUTE, 0x65], text("é")), Added::Skipped);
        assert_eq!(table.add(&[DEAD_ACUTE, 0x65], text("E")), Added::Overriding);
        assert_eq!(leaf(&table, &[DEAD_ACUTE, 0x65]), Some(text("E")));
        // A sequence starting a longer one is left out...
        table.add(&[MULTI_KEY, 0x6f, 0x63], text("©"));
        assert_eq!(table.add(&[MULTI_KEY, 0x6f], text("o")), Added::Skipped);
        assert_eq!(leaf(&table, &[MULTI_KEY, 0x6f, 0x63]), Some(text("©")));
        // ... and a longer one replaces the shorter it starts with.
        assert_eq!(
            table.add(&[DEAD_ACUTE, 0x65, 0x65], text("ée")),
            Added::Overriding
        );
        assert_eq!(leaf(&table, &[DEAD_ACUTE, 0x65]), None);
        assert_eq!(leaf(&table, &[DEAD_ACUTE, 0x65, 0x65]), Some(text("ée")));
    }

    #[test]
    fn outcomes_type_their_text_or_their_keysyms_character() {
        assert_eq!(text("é").typed().as_deref(), Some("é"));
        let keysym_only = Outcome {
            text: None,
            keysym: Some(0xe9),
        };
        assert_eq!(keysym_only.typed().as_deref(), Some("é"));
        let nothing = Outcome {
            text: None,
            keysym: Some(0xff0d),
        };
        assert_eq!(nothing.typed(), None, "Return types no character");
    }
}
