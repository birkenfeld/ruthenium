// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::str::FromStr;
use std::fs::{File, metadata};
use std::io::{BufReader, BufRead};
use std::path::{Path, PathBuf};
use glob::Pattern;


#[derive(Debug)]
pub struct Ignores {
    root: PathBuf,
    extensions: BTreeSet<String>,
    patterns: Vec<Pattern>,
}

fn is_literal(s: &str) -> bool {
    s.chars().all(|v| v.is_alphanumeric())
}

fn read_patterns_from(path: &Path, ignores: &mut Ignores) {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let line = line.trim();
                if line.is_empty() || line.starts_with("#") {
                    continue;
                }
                if line.starts_with("*.") && is_literal(&line[2..]) {
                    ignores.extensions.insert(line[2..].into());
                } else {
                    if let Ok(pat) = Pattern::from_str(line) {
                        ignores.patterns.push(pat);
                    }
                }
            }
        }
    }
}

pub fn read_patterns(dir: &Path) -> Ignores {
    let mut result = Ignores {
        root: dir.to_path_buf(),
        extensions: BTreeSet::new(),
        patterns: Vec::new(),
    };
    for gitexcludes in &[".gitignore", ".git/info/excludes"] {
        if metadata(dir.join(gitexcludes)).map(|f| f.is_file()).unwrap_or(false) {
            read_patterns_from(&dir.join(gitexcludes), &mut result);
        }
    }
    result
}

// unstable, from std::path::Path

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

pub fn relative_path_from<'a, P: ?Sized + AsRef<Path>>(path: &'a Path, base: &'a P) -> Option<&'a Path>
{
    iter_after(path.components(), base.as_ref().components()).map(|c| c.as_path())
}

pub fn match_patterns(path: &Path, ignores: &[Ignores]) -> bool {
    let ext = path.extension().and_then(|s| s.to_str());
    for ignore in ignores {
        if ext.is_some() && ignore.extensions.contains(ext.unwrap()) {
            return true;
        }
        for pattern in &ignore.patterns {
            if pattern.matches_path(relative_path_from(path, &ignore.root).unwrap()) {
                //println!("{:?} matches {}", path, pattern);
                return true;
            }
        }
    }
    false
}
