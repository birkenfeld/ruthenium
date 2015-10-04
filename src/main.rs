// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

#[macro_use]
extern crate clap;
extern crate regex_dfa;
extern crate walkdir;
extern crate memmap;

mod search;
mod display;
mod options;

use clap::{App, Arg};

use display::DisplayMode;
use search::create_rx;
use search::search;
use options::Opts;


fn get_options() -> Opts {
    let version = format!("v{}", crate_version!());
    let app = App::new("Ruthenium")
        .version(&version)
        .usage("ru [options] PATTERN [PATH]")
        .about("Recursively search for a pattern, like ack")
        .arg(Arg::with_name("pattern").required(true).index(1))
        .arg(Arg::with_name("path").index(2))
        .arg(Arg::with_name("all").short("a"))
        .arg(Arg::with_name("fileswith").short("l"))
        .arg(Arg::with_name("fileswithout").short("L").conflicts_with("fileswith"))
        ;
    let m = app.get_matches();
    Opts {
        pattern: m.value_of("pattern").unwrap().into(),
        path: m.value_of("path").unwrap_or(".").into(),
        all_files: m.is_present("all"),
        only_files: if m.is_present("fileswith") {
            Some(true)
        } else if m.is_present("fileswithout") {
            Some(false)
        } else { None },
    }
}

fn walk<D: DisplayMode>(display: &D, opts: &Opts) {
    let regex = create_rx(&opts.pattern);

    let mut first = true;
    let walker = walkdir::WalkDir::new(&opts.path).follow_links(true);
    for entry in walker.into_iter() {
        if let Ok(entry) = entry {
            if !entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                continue;
            }
            if let Ok(map) = memmap::Mmap::open_path(entry.path(), memmap::Protection::Read) {
                let buf = unsafe { map.as_slice() };
                if search(entry.path(), buf, &regex, &opts, display, first) > 0 {
                    first = false;
                }
            }
        }
    }
}

fn main() {
    let opts = get_options();

    if opts.only_files == Some(true) {
        walk(&display::FilesOnlyMode, &opts);
    } else if opts.only_files == Some(false) {
        walk(&display::FilesWithoutMatchMode, &opts);
    } else {
        walk(&display::DefaultMode, &opts);
    }
}
