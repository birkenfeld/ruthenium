// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

#[macro_use]
extern crate clap;
extern crate regex_dfa;
extern crate walkdir;
extern crate memmap;
extern crate scoped_threadpool;

mod search;
mod display;
mod options;

use std::thread;
use std::sync::mpsc::{channel, Sender};
use memmap::{Mmap, Protection};
use scoped_threadpool::Pool;
use walkdir::WalkDirIterator;

use display::DisplayMode;
use search::{create_rx, search, FileResult};
use options::Opts;


fn walk(chan: Sender<FileResult>, opts: &Opts) {
    let mut pool = Pool::new(3);
    let regex = create_rx(&opts);

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
    pool.scoped(|scope| {
        for entry in walker {
            if let Ok(entry) = entry {
                // weed out directories and special files
                if !entry.metadata().map(|m| m.is_file()).unwrap_or(false) {
                    continue;
                }
                let rx = regex.clone();
                let ch = chan.clone();
                scope.execute(move || {
                    // open and search file
                    if let Ok(map) = Mmap::open_path(entry.path(), Protection::Read) {
                        let buf = unsafe { map.as_slice() };
                        search(ch, &rx, &opts, entry.path(), buf);
                    }
                });
            }
        }
    });
}

fn run<D: DisplayMode>(display: &mut D, opts: Opts) {
    let (w_chan, r_chan) = channel();
    thread::spawn(move || {
        walk(w_chan, &opts);
    });
    while let Ok(r) = r_chan.recv() {
        display.print_result(r);
    }
}

fn main() {
    let mut opts = Opts::from_cmdline();
    let colors = opts.colors.take().unwrap();
    if opts.only_count {
        run(&mut display::CountMode::new(colors), opts);
    } else if opts.only_files == Some(true) {
        run(&mut display::FilesOnlyMode::new(colors, true), opts);
    } else if opts.only_files == Some(false) {
        run(&mut display::FilesOnlyMode::new(colors, false), opts);
    } else if !opts.show_heading {
        run(&mut display::OneLineMode::new(colors, opts.show_break), opts)
    } else {
        run(&mut display::DefaultMode::new(colors, opts.show_break), opts);
    }
}
