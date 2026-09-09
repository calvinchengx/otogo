//! Path globbing with `**` semantics.
//!
//! Hand-rolled rather than pulling in `regex` or `globset`, for two reasons:
//! the binary stays small, and the semantics stay pinned to exactly what the
//! reference implementation did — `loop.json` files are shared between
//! implementations, so a pattern must not change meaning underneath a repo.
//!
//!   `**/`  zero or more whole path components
//!   `**`   anything, including separators
//!   `*`    anything except a separator
//!   `?`    one character except a separator

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// `**/` — zero or more complete components.
    StarStarSlash,
    /// `**` — anything at all.
    StarStar,
    /// `*` — anything within one component.
    Star,
    /// `?` — one character within one component.
    Any,
    Lit(char),
}

#[derive(Debug, Clone)]
pub struct Pattern {
    source: String,
    toks: Vec<Tok>,
}

impl Pattern {
    pub fn new(source: &str) -> Self {
        let mut toks = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
                if i + 2 < chars.len() && chars[i + 2] == '/' {
                    toks.push(Tok::StarStarSlash);
                    i += 3;
                } else {
                    toks.push(Tok::StarStar);
                    i += 2;
                }
            } else if chars[i] == '*' {
                toks.push(Tok::Star);
                i += 1;
            } else if chars[i] == '?' {
                toks.push(Tok::Any);
                i += 1;
            } else {
                toks.push(Tok::Lit(chars[i]));
                i += 1;
            }
        }
        Pattern {
            source: source.to_string(),
            toks,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn matches(&self, path: &str) -> bool {
        let p: Vec<char> = path.chars().collect();
        matches_from(&self.toks, 0, &p, 0)
    }
}

fn matches_from(toks: &[Tok], ti: usize, path: &[char], pi: usize) -> bool {
    if ti == toks.len() {
        return pi == path.len();
    }
    match &toks[ti] {
        Tok::Lit(c) => {
            pi < path.len() && path[pi] == *c && matches_from(toks, ti + 1, path, pi + 1)
        }
        Tok::Any => pi < path.len() && path[pi] != '/' && matches_from(toks, ti + 1, path, pi + 1),
        Tok::Star => {
            // Consume anything up to the next separator.
            let mut k = pi;
            loop {
                if matches_from(toks, ti + 1, path, k) {
                    return true;
                }
                if k >= path.len() || path[k] == '/' {
                    return false;
                }
                k += 1;
            }
        }
        Tok::StarStar => {
            let mut k = pi;
            loop {
                if matches_from(toks, ti + 1, path, k) {
                    return true;
                }
                if k >= path.len() {
                    return false;
                }
                k += 1;
            }
        }
        Tok::StarStarSlash => {
            // Zero components, or any number of complete ones.
            if matches_from(toks, ti + 1, path, pi) {
                return true;
            }
            let mut k = pi;
            while k < path.len() {
                if path[k] == '/' && matches_from(toks, ti + 1, path, k + 1) {
                    return true;
                }
                k += 1;
            }
            false
        }
    }
}

/// An ordered set of patterns; `match_of` reports the first that matched, so
/// violation messages can name the rule that caught them.
#[derive(Debug, Clone, Default)]
pub struct GlobSet {
    pats: Vec<Pattern>,
}

impl GlobSet {
    pub fn new<I: IntoIterator<Item = String>>(patterns: I) -> Self {
        GlobSet {
            pats: patterns.into_iter().map(|p| Pattern::new(&p)).collect(),
        }
    }

    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        GlobSet::new(
            v.and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
    }

    pub fn match_of(&self, rel: &str) -> Option<&str> {
        self.pats
            .iter()
            .find(|p| p.matches(rel))
            .map(|p| p.source())
    }

    pub fn is_match(&self, rel: &str) -> bool {
        self.match_of(rel).is_some()
    }

    pub fn sources(&self) -> Vec<&str> {
        self.pats.iter().map(|p| p.source()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pat: &str, path: &str) -> bool {
        Pattern::new(pat).matches(path)
    }

    #[test]
    fn trailing_double_star_covers_everything_beneath() {
        assert!(m("goals/corpus/**", "goals/corpus/a.txt"));
        assert!(m("goals/corpus/**", "goals/corpus/deep/b.txt"));
        assert!(!m("goals/corpus/**", "goals/other.txt"));
    }

    #[test]
    fn leading_double_star_slash_matches_zero_components() {
        assert!(m("**/*_test.go", "a_test.go"));
        assert!(m("**/*_test.go", "internal/store/a_test.go"));
        assert!(!m("**/*_test.go", "internal/store/a.go"));
    }

    #[test]
    fn single_star_does_not_cross_a_separator() {
        assert!(m("docs/*.md", "docs/parity.md"));
        assert!(!m("docs/*.md", "docs/nested/parity.md"));
    }

    #[test]
    fn question_matches_one_non_separator() {
        assert!(m("v?.json", "v1.json"));
        assert!(!m("v?.json", "v10.json"));
        assert!(!m("a?b", "a/b"));
    }

    #[test]
    fn bare_double_star_matches_the_whole_tree() {
        assert!(m("**", "anything/at/all.txt"));
        assert!(m("**", "top.txt"));
    }

    #[test]
    fn first_matching_pattern_is_reported() {
        let gs = GlobSet::new(vec!["docs/**".into(), "**".into()]);
        assert_eq!(gs.match_of("docs/x.md"), Some("docs/**"));
    }
}
