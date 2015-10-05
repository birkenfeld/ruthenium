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


#[derive(Debug)]
pub struct Ignores {
    root: PathBuf,
    filenames: BTreeSet<String>,
    extensions: BTreeSet<String>,
    patterns: Vec<Pattern>,
    negated_patterns: Vec<Pattern>,
}

fn is_literal_filename(s: &str) -> bool {
    s.chars().all(|v| !(v == '*' || v == '?' || v == '[' || v == ']' || v == '/'))
}

fn is_literal_extension(s: &str) -> bool {
    s.chars().all(|v| !(v == '*' || v == '?' || v == '[' || v == ']' || v == '/' || v == '.'))
}

fn read_git_patterns_from(path: &Path, ignores: &mut Ignores) {
    fn add_pat(line: &str, vec: &mut Vec<Pattern>) {
        let pat = Pattern::from_str(
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
                if line.is_empty() || line.starts_with("#") {
                    continue;
                }
                if line.starts_with("!") {
                    add_pat(&line[1..], &mut ignores.negated_patterns);
                } else if is_literal_filename(line) {
                    ignores.filenames.insert(line.into());
                } else if line.starts_with("*.") && is_literal_extension(&line[2..]) {
                    ignores.extensions.insert(line[2..].into());
                } else {
                    add_pat(line, &mut ignores.patterns);
                }
            }
        }
    }
}

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

// unstable, from std::path::Path
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
