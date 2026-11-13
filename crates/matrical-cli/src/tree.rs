//! Folders as the listing gives them: finding things by path, and drawing
//! them as a tree.

use matrical_protocol::{ContentItem, Folder, Listing};
use std::collections::BTreeMap;

/// `research/2026/Pricing study` as its folder and its name.
pub fn split(path: &str) -> (String, String) {
    let parts = parts(path);
    match parts.split_last() {
        Some((name, folder)) => (folder.join("/"), name.clone()),
        None => (String::new(), String::new()),
    }
}

/// A path's names, without empty ones, so `/a//b/` is `a/b`.
pub fn parts(path: &str) -> Vec<String> {
    path.split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Paths compare as the server compares names: ignoring case.
fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// What a path names.
#[derive(Debug, PartialEq)]
pub enum Found<'a> {
    Top,
    Folder(&'a Folder),
    Item(&'a ContentItem),
    /// Items from before folders, at the top, sharing one name.
    Several(Vec<&'a ContentItem>),
    Nothing,
}

pub struct Tree<'a> {
    listing: &'a Listing,
    by_id: BTreeMap<&'a str, &'a Folder>,
}

impl<'a> Tree<'a> {
    pub fn new(listing: &'a Listing) -> Self {
        let by_id = listing.folders.iter().map(|f| (f.id.as_str(), f)).collect();
        Self { listing, by_id }
    }

    pub fn folder_path(&self, id: Option<&str>) -> String {
        id.and_then(|id| self.by_id.get(id))
            .map(|folder| folder.path.clone())
            .unwrap_or_default()
    }

    /// Where an item is, its name included.
    pub fn item_path(&self, item: &ContentItem) -> String {
        match self.folder_path(item.folder.as_deref()) {
            folder if folder.is_empty() => item.name.clone(),
            folder => format!("{folder}/{}", item.name),
        }
    }

    pub fn folder(&self, path: &str) -> Option<&'a Folder> {
        let wanted = parts(path).join("/");
        self.listing.folders.iter().find(|f| same(&f.path, &wanted))
    }

    pub fn find(&self, path: &str) -> Found<'a> {
        let (folder, name) = split(path);
        if name.is_empty() {
            return Found::Top;
        }
        if let Some(found) = self.folder(path) {
            return Found::Folder(found);
        }
        let parent = if folder.is_empty() {
            None
        } else {
            match self.folder(&folder) {
                Some(found) => Some(found.id.as_str()),
                None => return Found::Nothing,
            }
        };
        let items: Vec<&ContentItem> = self
            .listing
            .items
            .iter()
            .filter(|item| item.folder.as_deref() == parent && same(&item.name, &name))
            .collect();
        match items.as_slice() {
            [] => Found::Nothing,
            [one] => Found::Item(one),
            _ => Found::Several(items),
        }
    }

    /// Folders, then items, directly in a folder, each by name.
    pub fn children(&self, parent: Option<&str>) -> (Vec<&'a Folder>, Vec<&'a ContentItem>) {
        let mut folders: Vec<&Folder> = self
            .listing
            .folders
            .iter()
            .filter(|f| f.parent.as_deref() == parent)
            .collect();
        folders.sort_by_key(|f| (f.name.to_lowercase(), f.id.clone()));
        let mut items: Vec<&ContentItem> = self
            .listing
            .items
            .iter()
            .filter(|item| self.placed(item) == parent)
            .collect();
        items.sort_by_key(|item| (item.name.to_lowercase(), item.id.clone()));
        (folders, items)
    }

    /// Whether a folder (None for the top) sits at or under `top`.
    fn within(&self, at: Option<&str>, top: Option<&str>) -> bool {
        let Some(top) = top else { return true };
        let mut at = at.map(str::to_owned);
        while let Some(here) = at {
            if here == top {
                return true;
            }
            at = self.by_id.get(here.as_str()).and_then(|f| f.parent.clone());
        }
        false
    }

    /// Every folder and item at or under a folder; everything for the top.
    pub fn under(&self, top: Option<&str>) -> Listing {
        Listing {
            folders: self
                .listing
                .folders
                .iter()
                .filter(|f| self.within(Some(&f.id), top))
                .cloned()
                .collect(),
            items: self
                .listing
                .items
                .iter()
                .filter(|item| self.within(self.placed(item), top))
                .cloned()
                .collect(),
            ..self.listing.clone()
        }
    }

    /// The folder an item shows in: one the listing does not hold is the top.
    fn placed<'b>(&self, item: &'b ContentItem) -> Option<&'b str> {
        item.folder
            .as_deref()
            .filter(|id| self.by_id.contains_key(id))
    }

    /// How many items are in a folder and every folder under it.
    pub fn count(&self, id: &str) -> usize {
        self.listing
            .items
            .iter()
            .filter(|item| self.placed(item).is_some() && self.within(self.placed(item), Some(id)))
            .count()
    }

    /// The folder drawn as `tree` does: `depth` levels at most (all when
    /// None), and at most `limit` entries in each folder before "… n more".
    pub fn draw(&self, top: Option<&str>, depth: Option<usize>, limit: Option<usize>) -> String {
        let mut out = String::new();
        let title = match top {
            Some(id) => format!("{}/", self.folder_path(Some(id))),
            None => ".".to_owned(),
        };
        out.push_str(&title);
        out.push('\n');
        self.draw_into(&mut out, top, "", 1, depth, limit);
        out
    }

    fn draw_into(
        &self,
        out: &mut String,
        parent: Option<&str>,
        indent: &str,
        level: usize,
        depth: Option<usize>,
        limit: Option<usize>,
    ) {
        let (folders, items) = self.children(parent);
        let total = folders.len() + items.len();
        let shown = limit.unwrap_or(total).min(total);
        let more = total - shown;
        let entries = folders
            .iter()
            .map(|f| Entry::Folder(f))
            .chain(items.iter().map(|i| Entry::Item(i)))
            .take(shown);
        for (n, entry) in entries.enumerate() {
            let last = n + 1 == shown && more == 0;
            let (branch, next) = if last {
                ("└── ", "    ")
            } else {
                ("├── ", "│   ")
            };
            match entry {
                Entry::Folder(folder) => {
                    let deeper = depth.is_none_or(|depth| level < depth);
                    if deeper {
                        out.push_str(&format!("{indent}{branch}{}/\n", folder.name));
                        let indent = format!("{indent}{next}");
                        self.draw_into(out, Some(&folder.id), &indent, level + 1, depth, limit);
                    } else {
                        let n = self.count(&folder.id);
                        let items = if n == 1 { "item" } else { "items" };
                        out.push_str(&format!(
                            "{indent}{branch}{}/  ({n} {items})\n",
                            folder.name
                        ));
                    }
                }
                Entry::Item(item) => out.push_str(&format!("{indent}{branch}{}\n", item.name)),
            }
        }
        if more > 0 {
            out.push_str(&format!("{indent}└── … {more} more\n"));
        }
    }
}

enum Entry<'a> {
    Folder(&'a Folder),
    Item(&'a ContentItem),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(id: &str, name: &str, parent: Option<&str>, path: &str) -> Folder {
        Folder {
            id: id.into(),
            name: name.into(),
            parent: parent.map(Into::into),
            path: path.into(),
        }
    }

    fn item(id: &str, name: &str, folder: Option<&str>) -> ContentItem {
        ContentItem {
            id: id.into(),
            name: name.into(),
            folder: folder.map(Into::into),
            ..Default::default()
        }
    }

    fn listing() -> Listing {
        Listing {
            folders: vec![
                folder("r", "research", None, "research"),
                folder("y", "2026", Some("r"), "research/2026"),
                folder("e", "empty", None, "empty"),
            ],
            items: vec![
                item("1", "Pricing", Some("y")),
                item("2", "Costs", Some("y")),
                item("3", "Roadmap", None),
                item("4", "Glossary", None),
                item("5", "twin", None),
                item("6", "Twin", None),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_path_splits_into_its_folder_and_its_name() {
        assert_eq!(
            split("research/2026/Pricing study"),
            ("research/2026".into(), "Pricing study".into())
        );
        assert_eq!(split(" /Plan/ "), (String::new(), "Plan".into()));
        assert_eq!(split(""), (String::new(), String::new()));
    }

    #[test]
    fn a_path_finds_a_folder_an_item_or_nothing_ignoring_case() {
        let listing = listing();
        let tree = Tree::new(&listing);
        assert!(matches!(tree.find("Research/2026"), Found::Folder(f) if f.id == "y"));
        assert!(matches!(tree.find("research/2026/pricing"), Found::Item(i) if i.id == "1"));
        assert!(matches!(tree.find("Roadmap"), Found::Item(i) if i.id == "3"));
        assert!(matches!(tree.find("twin"), Found::Several(found) if found.len() == 2));
        assert_eq!(tree.find("research/nope/Pricing"), Found::Nothing);
        assert_eq!(tree.find("/"), Found::Top);
        assert_eq!(tree.item_path(&listing.items[0]), "research/2026/Pricing");
    }

    #[test]
    fn the_tree_draws_folders_first_and_stops_where_asked() {
        let listing = listing();
        let tree = Tree::new(&listing);
        assert_eq!(
            tree.draw(None, None, None),
            ".\n\
             ├── empty/\n\
             ├── research/\n\
             │   └── 2026/\n\
             │       ├── Costs\n\
             │       └── Pricing\n\
             ├── Glossary\n\
             ├── Roadmap\n\
             ├── twin\n\
             └── Twin\n"
        );
        assert_eq!(
            tree.draw(None, Some(1), Some(2)),
            ".\n\
             ├── empty/  (0 items)\n\
             ├── research/  (2 items)\n\
             └── … 4 more\n"
        );
        let research = Some("r");
        assert_eq!(
            tree.draw(research, Some(1), None),
            "research/\n└── 2026/  (2 items)\n"
        );
    }
}
