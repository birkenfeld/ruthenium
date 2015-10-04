// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use search::FileResult;
use options::Colors;


#[allow(unused_variables)]
pub trait DisplayMode: Send + Clone + 'static {
    fn print_result(&mut self, res: FileResult) { }
}

#[derive(Clone)]
pub struct DefaultMode {
    colors: Option<Colors>,
    is_first: bool,
}

impl DefaultMode {
    pub fn new(colors: Option<Colors>) -> DefaultMode {
        DefaultMode {
            colors: colors,
            is_first: true,
        }
    }
}

impl DisplayMode for DefaultMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first {
            println!("");
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            println!("{}", res.fname);
            for m in res.matches {
                println!("{}:{}", m.lineno, m.line);
            }
        }
        self.is_first = false;
    }
}

#[derive(Clone)]
pub struct FilesOnlyMode;

impl DisplayMode for FilesOnlyMode {
    fn print_result(&mut self, res: FileResult) {
        if !res.matches.is_empty() {
            println!("{}", res.fname);
        }
    }
}

#[derive(Clone)]
pub struct FilesWithoutMatchMode;

impl DisplayMode for FilesWithoutMatchMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            println!("{}", res.fname);
        }
    }
}
