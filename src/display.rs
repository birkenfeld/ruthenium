// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::borrow::Cow;


#[allow(unused_variables)]
pub trait DisplayMode {
    fn beforefile(&self, path: &Cow<str>, firstfile: bool) { }
    fn firstmatch(&self, path: &Cow<str>, firstfile: bool) -> bool { true }
    fn linematch(&self, path: &Cow<str>, lno: usize, line: &str, limits: &[(usize, usize)]) { }
    fn binmatch(&self, path: &Cow<str>) { }
    fn afterfile(&self, path: &Cow<str>, matches: usize) { }
}

pub struct DefaultMode;

impl DisplayMode for DefaultMode {
    fn firstmatch(&self, path: &Cow<str>, firstfile: bool) -> bool {
        if !firstfile {
            println!("");
        }
        println!("{}", path);
        true
    }

    fn linematch(&self, _path: &Cow<str>, lno: usize, line: &str, _limits: &[(usize, usize)]) {
        println!("{}:{}", lno, line);
    }
}

pub struct FilesOnlyMode;

impl DisplayMode for FilesOnlyMode {
    fn firstmatch(&self, path: &Cow<str>, _firstfile: bool) -> bool {
        println!("{}", path);
        false
    }
}

pub struct FilesWithoutMatchMode;

impl DisplayMode for FilesWithoutMatchMode {
    fn afterfile(&self, path: &Cow<str>, matches: usize) {
        if matches == 0 {
            println!("{}", path);
        }
    }
}
