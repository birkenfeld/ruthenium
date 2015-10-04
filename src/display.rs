// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::borrow::Cow;
use std::sync::mpsc::Sender;


#[allow(unused_variables)]
pub trait DisplayMode: Send + Clone + 'static {
    fn beforefile(&self, fname: &Cow<str>, firstfile: bool) { }
    fn firstmatch(&self, fname: &Cow<str>, firstfile: bool) -> bool { true }
    fn linematch(&self, fname: &Cow<str>, lineno: usize, line: &str, limits: &[(usize, usize)]) { }
    fn binmatch(&self, fname: &Cow<str>) { }
    fn afterfile(&self, fname: &Cow<str>, matches: usize) { }
}

#[derive(Clone)]
pub struct DefaultMode(pub Sender<String>);

impl DisplayMode for DefaultMode {
    fn firstmatch(&self, fname: &Cow<str>, firstfile: bool) -> bool {
        if !firstfile {
            self.0.send("".into());
        }
        // XXX into!
        self.0.send(fname.to_owned().into_owned());
        true
    }

    fn linematch(&self, _fname: &Cow<str>, lineno: usize, line: &str, _limits: &[(usize, usize)]) {
        self.0.send(format!("{}:{}", lineno, line));
    }
}

#[derive(Clone)]
pub struct FilesOnlyMode(pub Sender<String>);

impl DisplayMode for FilesOnlyMode {
    fn firstmatch(&self, fname: &Cow<str>, _firstfile: bool) -> bool {
        println!("{}", fname);
        false
    }
}

#[derive(Clone)]
pub struct FilesWithoutMatchMode(pub Sender<String>);

impl DisplayMode for FilesWithoutMatchMode {
    fn afterfile(&self, fname: &Cow<str>, matches: usize) {
        if matches == 0 {
            println!("{}", fname);
        }
    }
}
