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
use walkdir::WalkDirIterator;

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
        .arg(Arg::with_name("depth").long("depth").takes_value(true))
        .arg(Arg::with_name("alltext").short("t").conflicts_with("all"))
        .arg(Arg::with_name("unrestricted").short("u").conflicts_with("all"))
        .arg(Arg::with_name("searchbinary").long("search-binary"))
        .arg(Arg::with_name("searchhidden").long("hidden"))
        .arg(Arg::with_name("fileswith").short("l"))
        .arg(Arg::with_name("fileswithout").short("L").conflicts_with("fileswith"))
        .arg(Arg::with_name("follow").short("f"))
        ;
    let m = app.get_matches();
    let mut binaries = m.is_present("searchbinary");
    let mut hidden = m.is_present("searchhidden");
    let mut ignores = true;
    if m.is_present("all") {
        binaries = true;
        ignores = false;
    }
    else if m.is_present("alltext") {
        ignores = false;
    }
    else if m.is_present("unrestricted") {
        binaries = true;
        hidden = true;
        ignores = false;
    }
    Opts {
        pattern: m.value_of("pattern").unwrap().into(),
        path: m.value_of("path").unwrap_or(".").into(),
        depth: m.value_of("depth").and_then(|v| v.parse().ok()).unwrap_or(::std::usize::MAX),
        follow_links: m.is_present("follow"),
        do_binaries: binaries,
        do_hidden: hidden,
        check_ignores: ignores,
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
    let walker = walkdir::WalkDir::new(&opts.path)
        .follow_links(opts.follow_links)
        .max_depth(opts.depth);
    let walker = walker.into_iter().filter_entry(|entry| {
        // weed out hidden files
        if let Some(fname) = entry.path().file_name() {
            if !opts.do_hidden && fname.to_string_lossy().starts_with(".") {
                return false;
            }
        }
        true
    });
    for entry in walker {
        if let Ok(entry) = entry {
            // weed out directories and special files
            if !entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                continue;
            }
            // open and search file
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
