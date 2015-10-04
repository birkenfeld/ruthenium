// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::path::Path;

use regex_dfa::Program as Regex;

use display::DisplayMode;
use options;

pub fn create_rx(pattern: &str) -> Regex {
    Regex::from_regex(pattern).unwrap()
}

pub fn search<D>(path: &Path, buf: &[u8], regex: &Regex, opts: &options::Opts,
                 display: &D, firstfile: bool) -> usize
    where D: DisplayMode
{
    let len = buf.len();
    let mut start = 0;
    let mut lno = 0;
    let mut matches = 0;
    let fname = path.to_string_lossy();
    display.beforefile(&fname, firstfile);
    while start < len {
        lno += 1;
        let end = buf[start..].iter().position(|&x| x == b'\n').unwrap_or(len - start);
        let line = &buf[start..start+end];
        if let Ok(line) = ::std::str::from_utf8(line) {
            if let Some((s1, s2)) = regex.shortest_match(&line) {
                if matches == 0 {
                    if !display.firstmatch(&fname, firstfile) {
                        break;
                    }
                }
                display.linematch(&fname, lno, line, &[(s1, s2)]);
                matches += 1;
            }
        }
        start += end + 1;
    }
    display.afterfile(&fname, matches);
    matches
}
