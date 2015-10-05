// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::fs::{File, metadata};
use std::io::{BufReader, BufRead};
use std::path::{Path, PathBuf};
use glob::{Pattern, MatchOptions};


/// Represents the ignore patterns for one directory, the `root`.
#[derive(Debug)]
pub struct Ignores {
    /// Path patterns are relative to this directory
    root: PathBuf,
    /// Literal filenames to exclude
    filenames: BTreeSet<String>,
    /// Literal file extensions to exclude
    extensions: BTreeSet<String>,
    /// Patterns to exclude (can have paths)
    patterns: Vec<Pattern>,
    /// "Negated patterns": matched after a file would be excluded,
    /// if it matches, the exclusion is canceled
    negated_patterns: Vec<Pattern>,
}

fn is_literal_filename(s: &str) -> bool {
    s.chars().all(|v| !(v == '*' || v == '?' || v == '[' || v == ']' || v == '/'))
}

fn is_literal_extension(s: &str) -> bool {
    s.chars().all(|v| !(v == '*' || v == '?' || v == '[' || v == ']' || v == '/' || v == '.'))
}

/// Read gitignore-style patterns from a filename and add all recognized
/// patterns to the Ignores object.
fn read_git_patterns_from(path: &Path, ignores: &mut Ignores) {
    // add a complex pattern
    fn add_pat(line: &str, vec: &mut Vec<Pattern>) {
        let pat = Pattern::from_str(
            // if a pattern doesn't start with "/", it is not anchored to the root,
            // so to make glob match any such file we need to start it with "**/"
            if !line.starts_with("/") {
                Cow::Owned(String::from("**/") + line)
            } else {
                Cow::Borrowed(line)
            }.as_ref());
        if let Ok(pat) = pat {
            vec.push(pat);
        }
    }
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let line = line.trim();
                // empty line or comment, ignore
                if line.is_empty() || line.starts_with("#") {
                    continue;
                }
                // negated pattern (no special casing for filenames/exts here)
                if line.starts_with("!") {
                    add_pat(&line[1..], &mut ignores.negated_patterns);
                // simple filename
                } else if is_literal_filename(line) {
                    ignores.filenames.insert(line.into());
                // simple *.ext
                } else if line.starts_with("*.") && is_literal_extension(&line[2..]) {
                    ignores.extensions.insert(line[2..].into());
                // complex non-negated pattern
                } else {
                    add_pat(line, &mut ignores.patterns);
                }
            }
        }
    }
}

/// Read patterns from all recognized and existing ignore files in `dir`.
pub fn read_patterns(dir: &Path) -> Ignores {
    let mut result = Ignores {
        root: dir.to_path_buf(),
        filenames: BTreeSet::new(),
        extensions: BTreeSet::new(),
        patterns: Vec::new(),
        negated_patterns: Vec::new(),
    };
    for gitexcludes in &[".gitignore", ".git/info/excludes"] {
        if metadata(dir.join(gitexcludes)).map(|f| f.is_file()).unwrap_or(false) {
            read_git_patterns_from(&dir.join(gitexcludes), &mut result);
        }
    }
    result
}

/// Return relative path from `base` to `path`.
///
/// Copied from std::path::Path, where it is still unstable.
pub fn relative_path_from<'a, P: AsRef<Path>>(path: &'a Path, base: &'a P) -> Option<&'a Path>
{
    fn iter_after<A, I, J>(mut iter: I, mut prefix: J) -> Option<I> where
        I: Iterator<Item=A> + Clone, J: Iterator<Item=A>, A: PartialEq
    {
        loop {
            let mut iter_next = iter.clone();
            match (iter_next.next(), prefix.next()) {
                (Some(x), Some(y)) => {
                    if x != y { return None }
                }
                (Some(_), None) => return Some(iter),
                (None, None) => return Some(iter),
                (None, Some(_)) => return None,
            }
            iter = iter_next;
        }
    }

    iter_after(path.components(), base.as_ref().components()).map(|c| c.as_path())
}

/// Match `path` against the ignore stack `ignores`, return true if match found.
pub fn match_patterns(path: &Path, ignores: &[Ignores]) -> bool {
    const OPTS: MatchOptions = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let name = path.file_name().and_then(|s| s.to_str());
    let ext = path.extension().and_then(|s| s.to_str());

    let mut is_ignored = false;
    for ignore in ignores {
        if name.is_some() && ignore.filenames.contains(name.unwrap()) {
            is_ignored = true;
        } else if ext.is_some() && ignore.extensions.contains(ext.unwrap()) {
            is_ignored = true;
        } else if !ignore.patterns.is_empty() {
            let relpath = relative_path_from(path, &ignore.root).unwrap();
            for pattern in &ignore.patterns {
                if pattern.matches_path_with(relpath, &OPTS) {
                    is_ignored = true;
                    break;
                }
            }
        }
        // apply negated patterns if necessary
        if is_ignored && !ignore.negated_patterns.is_empty() {
            let relpath = relative_path_from(path, &ignore.root).unwrap();
            for pattern in &ignore.negated_patterns {
                if pattern.matches_path_with(relpath, &OPTS) {
                    is_ignored = false;
                }
            }
        }
        if is_ignored {
            break;
        }
    }
    is_ignored
}
