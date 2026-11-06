//! The public API of pltrs (Rust) as rustdoc documents it, read from the
//! HTML that stable rustdoc writes: every item each crate's `all.html`
//! lists, and the members (methods, fields, variants, associated items)
//! the item's own page shows.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The crates whose public API counts, by their package names.
pub(super) const CRATES: &[&str] = &[
    "pltrs",
    "pltrs-core",
    "pltrs-codec",
    "pltrs-font",
    "pltrs-raster",
    "pltrs-text",
    "pltrs-render",
    "pltrs-platform",
    "pltrs-ui",
];

/// The member kinds of rustdoc's anchors (`id="method.hist"`).
const MEMBER_ANCHORS: &[&str] = &[
    "method",
    "tymethod",
    "structfield",
    "variant",
    "associatedconstant",
    "associatedtype",
];

/// The `<h2>` sections of a page after which members belong to other
/// items: the implementors of a trait and its impls on foreign types.
const FOREIGN_SECTIONS: &[&str] = &["implementors", "foreign-impls", "synthetic-implementations"];

/// Where a member of an item comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Origin {
    /// The standard library: an implementation of one of its traits
    /// (`Clone`, `From`, `PartialEq`, ...) or a `Deref` to one of its types.
    Std,
    /// The workspace: the item's own definition, an inherent impl, or an
    /// impl of a workspace trait or `Deref` to a workspace type.
    Workspace,
}

/// What a path names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Resolution {
    /// A public item.
    Item,
    /// A member of a public item.
    Member(Origin),
    /// Nothing public.
    Missing,
}

/// One public item: its documentation page and, once read, its members.
#[derive(Debug)]
struct Item {
    page: PathBuf,
    members: OnceCell<BTreeMap<String, Origin>>,
}

/// Every public item of [`CRATES`], by path (`pltrs::axes::Axes`).
#[derive(Debug)]
pub(super) struct Index {
    items: BTreeMap<String, Item>,
}

impl Index {
    /// The index of the documentation in `doc` (cargo's `target/doc`).
    pub(super) fn read(doc: &Path) -> Result<Self, String> {
        let mut items = BTreeMap::new();
        for package in CRATES {
            let name = package.replace('-', "_");
            let all = doc.join(&name).join("all.html");
            let html = fs::read_to_string(&all)
                .map_err(|err| format!("cannot read {}: {err}", all.display()))?;
            for (href, path) in all_items(&html) {
                let item = Item {
                    page: doc.join(&name).join(href),
                    members: OnceCell::new(),
                };
                items.insert(format!("{name}::{path}"), item);
            }
        }
        Ok(Self { items })
    }

    /// The number of items.
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    /// What `path` names: an item, a member of one (`Axes::hist`), or nothing.
    pub(super) fn resolve(&self, path: &str) -> Resolution {
        if self.items.contains_key(path) {
            return Resolution::Item;
        }
        let Some((owner, member)) = path.rsplit_once("::") else {
            return Resolution::Missing;
        };
        self.items
            .get(owner)
            .and_then(|item| item.members().get(member).copied())
            .map_or(Resolution::Missing, Resolution::Member)
    }

    /// The items whose last segment is that of `path`'s item or owner:
    /// where a path that names nothing may have meant to point.
    pub(super) fn namesakes(&self, path: &str) -> Vec<&str> {
        let mut segments = path.rsplit("::");
        let last = segments.next().unwrap_or_default();
        let owner = segments.next().unwrap_or_default();
        self.items
            .keys()
            .filter(|candidate| {
                let name = candidate.rsplit("::").next().unwrap_or_default();
                *candidate != path && (name == last || name == owner)
            })
            .map(String::as_str)
            .collect()
    }
}

impl Item {
    /// The item's members, read from its page the first time.
    fn members(&self) -> &BTreeMap<String, Origin> {
        self.members.get_or_init(|| {
            fs::read_to_string(&self.page)
                .map(|html| page_members(&html))
                .unwrap_or_default()
        })
    }
}

/// The `(page, path)` of every item `all.html` lists, the path relative
/// to the crate (`axes::Axes`).
fn all_items(html: &str) -> impl Iterator<Item = (&str, &str)> {
    html.split("<li><a href=\"").skip(1).filter_map(|entry| {
        let (href, rest) = entry.split_once("\">")?;
        let (path, _) = rest.split_once("</a>")?;
        let page = href.rsplit('/').next().unwrap_or(href);
        (!href.starts_with('#') && page != "index.html").then_some((href, path))
    })
}

/// The members a page documents, with where each comes from (the
/// workspace wins when a name comes from both).
fn page_members(html: &str) -> BTreeMap<String, Origin> {
    let mut members = BTreeMap::new();
    let mut origin = Origin::Workspace;
    for (tag, id, after) in anchors(html) {
        if tag == "h2" {
            if FOREIGN_SECTIONS.contains(&id) {
                break;
            }
            origin = if id.starts_with("deref-methods-") {
                deref_origin(after)
            } else {
                Origin::Workspace
            };
        } else if let Some(trait_name) = impl_trait(id) {
            origin = trait_origin(trait_name, after);
        } else if let Some((kind, name)) = id.split_once('.')
            && MEMBER_ANCHORS.contains(&kind)
        {
            let name = name.split_once('.').map_or(name, |(first, _)| first);
            let entry = members
                .entry(strip_counter(name).to_owned())
                .or_insert(origin);
            *entry = (*entry).max(origin);
        }
    }
    members
}

/// Every `id` attribute of `html`, with the name of the tag it is on and
/// the text after it.
fn anchors(html: &str) -> impl Iterator<Item = (&str, &str, &str)> {
    html.match_indices(" id=\"").filter_map(|(at, marker)| {
        let open = html[..at].rfind('<')?;
        let tag = html[open + 1..at].split(' ').next()?;
        let (id, after) = html[at + marker.len()..].split_once('"')?;
        Some((tag, id, after))
    })
}

/// The trait an impl section implements, from its anchor
/// (`impl-Clone-for-Axes` is `Clone`; `impl-Axes-3`, an inherent impl,
/// none).
fn impl_trait(id: &str) -> Option<&str> {
    let body = id.strip_prefix("impl-")?;
    let (trait_part, _) = body.split_once("-for-")?;
    Some(trait_part.split("%3C").next().unwrap_or(trait_part))
}

/// Where the methods of an impl of `trait_name` come from, from the link
/// to the trait in the impl's header (`header`, the page after the anchor):
/// its title names the trait's crate (`trait core::clone::Clone`).
fn trait_origin(trait_name: &str, header: &str) -> Origin {
    let header = header.split("</h3>").next().unwrap_or(header);
    let suffix = format!("::{trait_name}\"");
    header
        .split("title=\"trait ")
        .skip(1)
        .find_map(|title| {
            let end = title.find('"')? + 1;
            title[..end]
                .ends_with(&suffix)
                .then(|| crate_origin(&title[..end]))
        })
        .unwrap_or(Origin::Std)
}

/// Where the methods of a `Deref` section come from: the crate of its
/// target, the link after `Target =` in the heading (`header`).
fn deref_origin(header: &str) -> Origin {
    let header = header.split("</h2>").next().unwrap_or(header);
    header
        .split_once("Target = ")
        .and_then(|(_, target)| target.split("title=\"").nth(1))
        .and_then(|title| title.split_once(' '))
        .map_or(Origin::Std, |(_, path)| crate_origin(path))
}

/// The origin of an item at `path` (`core::clone::Clone"...`).
fn crate_origin(path: &str) -> Origin {
    let krate = path.split("::").next().unwrap_or_default();
    if CRATES
        .iter()
        .any(|package| package.replace('-', "_") == krate)
    {
        Origin::Workspace
    } else {
        Origin::Std
    }
}

/// `name` without rustdoc's counter for repeated anchors (`new-1`).
fn strip_counter(name: &str) -> &str {
    match name.rsplit_once('-') {
        Some((base, counter)) if counter.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => name,
    }
}
