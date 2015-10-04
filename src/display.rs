// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::borrow::Cow;


#[allow(unused_variables)]
pub trait DisplayMode {
    fn beforefile(&self, fname: &Cow<str>, firstfile: bool) { }
    fn firstmatch(&self, fname: &Cow<str>, firstfile: bool) -> bool { true }
    fn linematch(&self, fname: &Cow<str>, lineno: usize, line: &str, limits: &[(usize, usize)]) { }
    fn binmatch(&self, fname: &Cow<str>) { }
    fn afterfile(&self, fname: &Cow<str>, matches: usize) { }
}

pub struct DefaultMode;

impl DisplayMode for DefaultMode {
    fn firstmatch(&self, fname: &Cow<str>, firstfile: bool) -> bool {
        if !firstfile {
            println!("");
        }
        println!("{}", fname);
        true
    }

    fn linematch(&self, _fname: &Cow<str>, lineno: usize, line: &str, _limits: &[(usize, usize)]) {
        println!("{}:{}", lineno, line);
    }
}

pub struct FilesOnlyMode;

impl DisplayMode for FilesOnlyMode {
    fn firstmatch(&self, fname: &Cow<str>, _firstfile: bool) -> bool {
        println!("{}", fname);
        false
    }
}

pub struct FilesWithoutMatchMode;

impl DisplayMode for FilesWithoutMatchMode {
    fn afterfile(&self, fname: &Cow<str>, matches: usize) {
        if matches == 0 {
            println!("{}", fname);
        }
    }
}
