// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::path::Path;

use regex_dfa::Program as Regex;

use display::DisplayMode;
use options;

pub fn create_rx(pattern: &str, literal: bool) -> Regex {
    let mut pattern = pattern.to_owned();
    if literal {
        const ESCAPE: &'static str = ".?*+|^$(){}[]\\";
        pattern = pattern.chars().map(|c| {
            if ESCAPE.find(c).is_some() {
                format!("\\{}", c)
            } else {
                format!("{}", c)
            }
        }).collect();
    }
    Regex::from_regex(&pattern).unwrap()
}

pub fn is_binary(buf: &[u8], len: usize) -> bool {
    if len == 0 {
        return false;
    }
    if len >= 3 && &buf[0..3] == b"\xEF\xBB\xBF" {
        // UTF-8 BOM
        return false;
    }
    let n = min(512, len);
    for b in buf[..n].iter() {
        if *b == b'\x00' {
            return true;  // null byte always means binary
        }
    }
    false
}

pub fn search<D>(path: &Path, buf: &[u8], regex: &Regex, opts: &options::Opts,
                 display: &D, firstfile: bool) -> usize
    where D: DisplayMode
{
    let len = buf.len();
    let mut start = 0;
    let mut lineno = 0;
    let mut matches = 0;
    let fname = path.to_string_lossy();
    display.beforefile(&fname, firstfile);
    if is_binary(buf, len) {
        if opts.do_binaries {
            if let Ok(content) = ::std::str::from_utf8(buf) {
                if let Some((_, _)) = regex.shortest_match(&content) {
                    display.binmatch(&fname);
                }
            }
        }
        display.afterfile(&fname, matches);
        return 0;
    }
    while start < len {
        lineno += 1;
        let end = buf[start..].iter().position(|&x| x == b'\n').unwrap_or(len - start);
        let line = &buf[start..start+end];
        if let Ok(line) = ::std::str::from_utf8(line) {
            if let Some(idx) = regex.shortest_match(&line) {
                if matches == 0 {
                    if !display.firstmatch(&fname, firstfile) {
                        break;
                    }
                }
                display.linematch(&fname, lineno, line, &[idx]);
                matches += 1;
            }
        }
        start += end + 1;
    }
    display.afterfile(&fname, matches);
    matches
}
