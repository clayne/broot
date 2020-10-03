
use {
    crate::{
        errors::{ConfError, PatternError},
    },
};

/// where to search
enum SearchObject {
    Name,
    Path,
    Content,
}
/// how to search
enum SearchKind {
    Exact,
    Fuzzy,
    Regex,
    Unspecified,
}

/// a valid combination of SearchObject and SearchKind,
/// determine how a pattern will be used
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMode {
    NameExact,
    NameFuzzy,
    NameRegex,
    PathExact,
    PathFuzzy,
    PathRegex,
    ContentExact,
    ContentRegex,
}

impl SearchMode {
    fn new(search_object: SearchObject, search_kind: SearchKind) -> Option<Self> {
        use {
            SearchObject::*,
            SearchKind::*,
        };
        match (search_object, search_kind) {
            (Name, Unspecified) => Some(Self::NameFuzzy),
            (Name, Exact) => Some(Self::NameExact),
            (Name, Fuzzy) => Some(Self::NameFuzzy),
            (Name, Regex) => Some(Self::NameRegex),

            (Path, Unspecified) => Some(Self::PathFuzzy),
            (Path, Exact) => Some(Self::PathExact),
            (Path, Fuzzy) => Some(Self::PathFuzzy),
            (Path, Regex) => Some(Self::PathRegex),

            (Content, Unspecified) => Some(Self::ContentExact),
            (Content, Exact) => Some(Self::ContentExact),
            (Content, Fuzzy) => None, // unsupported for now - could be but why ?
            (Content, Regex) => Some(Self::ContentRegex),
        }
    }
}

/// define a mapping from a search mode which can be typed in
/// the input to a SearchMode value
#[derive(Debug, Clone)]
pub struct SearchModeMapEntry {
    key: Option<String>,
    mode: SearchMode,
}

/// manage how to find the search mode to apply to a
/// pattern taking the config in account.
#[derive(Debug, Clone)]
pub struct SearchModeMap {
    entries: Vec<SearchModeMapEntry>,
}

impl SearchModeMapEntry {
    pub fn parse(conf_key: &str, conf_mode: &str) -> Result<Self, ConfError> {
        let s = conf_mode.to_lowercase();
        let s = s.trim();

        let name = s.contains("name");
        let path = s.contains("path");
        let content = s.contains("content");
        let search_object = match (name, path, content) {
            //(false, false, false) => SearchObject::Unspecified,
            (true, false, false) => SearchObject::Name,
            (false, true, false) => SearchObject::Path,
            (false, false, true) => SearchObject::Content,
            _ => {
                return Err(ConfError::InvalidSearchMode {
                    details: "you must have exactly one of \"name\", \"path\" or \"content".to_string()
                });
            }
        };

        let exact = s.contains("exact");
        let fuzzy = s.contains("fuzzy");
        let regex = s.contains("regex");
        let search_kind = match (exact, fuzzy, regex) {
            (false, false, false) => SearchKind::Unspecified,
            (true, false, false) => SearchKind::Exact,
            (false, true, false) => SearchKind::Fuzzy,
            (false, false, true) => SearchKind::Regex,
            _ => {
                return Err(ConfError::InvalidSearchMode {
                    details: "you may have at most one of \"exact\", \"fuzzy\" or \"regex\"".to_string()
                });
            }
        };

        let mode = match SearchMode::new(search_object, search_kind) {
            Some(mode) => mode,
            None => {
                return Err(ConfError::InvalidSearchMode {
                    details: "Unsupported combination of search object and kind".to_string()
                });
            },
        };

        let key = if conf_key.is_empty() || conf_key == "<empty>" {
            // serde toml parser doesn't handle correctly empty keys so we accept as
            // alternative the `"<empty>" = "fuzzy name"` solution.
            // TODO look at issues and/or code in serde-toml
            None
        } else if regex!(r"^\w*/$").is_match(conf_key) {
            Some(conf_key[0..conf_key.len()-1].to_string())
        } else {
            return Err(ConfError::InvalidKey {
                raw: conf_key.to_string(),
            });
        };
        Ok(SearchModeMapEntry { key, mode })
    }
}

impl Default for SearchModeMap {
    fn default() -> Self {
        let mut smm = SearchModeMap {
            entries: Vec::new(),
        };
        // the last keys are prefered
        smm.setm(&["ne", "en", "e"], SearchMode::NameExact);
        smm.setm(&["nf", "fn", "n", "f"], SearchMode::NameFuzzy);
        smm.setm(&["r", "nr", "rn", ""], SearchMode::NameRegex);
        smm.setm(&["pe", "ep"], SearchMode::PathExact);
        smm.setm(&["pf", "fp", "p"], SearchMode::PathFuzzy);
        smm.setm(&["pr", "rp"], SearchMode::PathRegex);
        smm.setm(&["ce", "ec", "c"], SearchMode::ContentExact);
        smm.setm(&["rx", "cr"], SearchMode::ContentRegex);
        smm.set(SearchModeMapEntry { key: None, mode: SearchMode::NameFuzzy });
        smm
    }
}

impl SearchModeMap {
    pub fn setm(&mut self, keys: &[&str], mode: SearchMode) {
        for key in keys {
            self.set(SearchModeMapEntry {
                key: Some(key.to_string()),
                mode,
            });
        }
    }
    /// we don't remove existing entries to ensure there's always a matching entry in
    /// mode->key (but search iterations will be done in reverse)
    pub fn set(&mut self, entry: SearchModeMapEntry) {
        self.entries.push(entry);
    }
    pub fn search_mode(&self, key: Option<&String>) -> Result<SearchMode, PatternError> {
        for entry in self.entries.iter().rev() {
            if entry.key.as_ref() == key {
                return Ok(entry.mode);
            }
        }
        Err(PatternError::InvalidMode {
            mode: if let Some(key) = key {
                format!("{}/", key)
            } else {
                "".to_string()
            },
        })
    }
    pub fn key(&self, search_mode: SearchMode) -> Option<&String> {
        for entry in self.entries.iter().rev() {
            if entry.mode == search_mode {
                return entry.key.as_ref();
            }
        }
        warn!("search mode key not found for {:?}", search_mode); // should not happen
        None
    }
}

