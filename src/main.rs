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
extern crate num_cpus;
extern crate glob;

mod search;
mod ignore;
mod display;
mod options;

use std::cell::RefCell;
use std::cmp::max;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use memmap::{Mmap, Protection};
use scoped_threadpool::Pool;
use walkdir::WalkDirIterator;

use display::DisplayMode;
use search::FileResult;
use options::Opts;


fn walk(chan: Sender<FileResult>, opts: &Opts) {
    let mut pool = Pool::new(max(opts.workers - 1, 1));
    let regex = search::create_rx(&opts);

    let walker = walkdir::WalkDir::new(&opts.path)
        .follow_links(opts.follow_links)
        .max_depth(opts.depth);
    pool.scoped(|scope| {
        let rx = &regex;
        let mut parent_stack: Vec<::std::path::PathBuf> = Vec::new();
        let ignore_stack = RefCell::new(Vec::new()); // XXX: add global ignores from cmdline/config?
        let walker = walker.into_iter().filter_entry(|entry| {
            // weed out hidden files
            let path = entry.path();
            if let Some(fname) = path.file_name() {
                if !opts.do_hidden && fname.to_string_lossy().starts_with(".") {
                    return false;
                }
            }
            if opts.check_ignores && ignore::match_patterns(path, &ignore_stack.borrow()) {
                return false;
            }
            true
        });
        for entry in walker {
            if let Ok(entry) = entry {
                // we got a new dir?
                if entry.file_type().is_dir() {
                    let mut ignore_stack = ignore_stack.borrow_mut();
                    let new_path = entry.path().to_path_buf();
                    // find the parent of this new directory on the stack
                    while !parent_stack.is_empty() &&
                        parent_stack.last().unwrap().as_path() != new_path.parent().unwrap()
                    {
                        ignore_stack.pop();
                        parent_stack.pop();
                    }
                    // read ignore patterns specific to this directory
                    ignore_stack.push(ignore::read_patterns(&new_path));
                    parent_stack.push(new_path);
                    continue;
                }
                // weed out further special files
                if !entry.file_type().is_file() {
                    continue;
                }
                // open and search file in one of the worker threads
                let ch = chan.clone();
                scope.execute(move || {
                    let path = entry.path();
                    if let Ok(map) = Mmap::open_path(path, Protection::Read) {
                        let buf = unsafe { map.as_slice() };
                        let res = search::search(rx, &opts, path, buf);
                        ch.send(res).unwrap();
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
    } else if opts.ackmate_format {
        run(&mut display::AckMateMode::new(), opts);
    } else if opts.vimgrep_format {
        run(&mut display::VimGrepMode, opts);
    } else if !opts.show_heading {
        run(&mut display::OneLineMode::new(colors, opts.show_break), opts);
    } else {
        run(&mut display::DefaultMode::new(colors, opts.show_break), opts);
    }
}
