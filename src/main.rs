// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

#[macro_use]
extern crate clap;
extern crate libc;
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
mod pcre;

use std::cmp::max;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use memmap::{Mmap, Protection};
use scoped_threadpool::Pool;
use walkdir::WalkDirIterator;

use display::DisplayMode;
use search::FileResult;
use options::Opts;


/// Walk a directory (given in Opts) and check all found files.
///
/// The channel is used to send result structs to the main thread, which gives
/// them to the DisplayMode for output.
///
/// The thread of this function only does the directory walking, it spawns a
/// number of worker threads in a pool to grep individual files.
fn walk(chan: SyncSender<FileResult>, opts: &Opts) {
    // thread pool for individual file grep worker threads
    let mut pool = Pool::new(max(opts.workers - 1, 1));
    // create the regex object
    let regex = search::create_rx(&opts);

    let walker = walkdir::WalkDir::new(&opts.path)
        .follow_links(opts.follow_links)
        .max_depth(opts.depth);
    pool.scoped(|scope| {
        let rx = &regex;  // borrow for closures
        // stack of directories being walked, maintained in the filter closure
        let mut parent_stack: Vec<::std::path::PathBuf> = Vec::new();
        // stack of Ignore structs per directory in parent_stack, they accumulate
        // XXX: add global ignores from cmdline and a config file here
        let mut ignore_stack = Vec::new();
        let walker = walker.into_iter().filter_entry(|entry| {
            // remove parents from stack that are not applicable anymore
            let new_parent = entry.path().parent().unwrap();
            while !parent_stack.is_empty() &&
                parent_stack.last().unwrap().as_path() != new_parent
            {
                ignore_stack.pop();
                parent_stack.pop();
            }
            // weed out hidden files (this is separate from ignored)
            let path = entry.path();
            if let Some(fname) = path.file_name() {
                if !opts.do_hidden && fname.to_string_lossy().starts_with(".") {
                    return false;
                }
            }
            // weed out ignored files and directories (if we return false here for
            // directories, the contents are pruned from the iterator)
            if opts.check_ignores && ignore::match_patterns(path, &ignore_stack) {
                return false;
            }
            // we got a new dir? put it onto the stack
            if entry.file_type().is_dir() {
                let new_path = entry.path().to_path_buf();
                // read ignore patterns specific to this directory
                ignore_stack.push(ignore::read_patterns(&new_path));
                parent_stack.push(new_path);
            }
            true
        });
        for entry in walker {
            if let Ok(entry) = entry {
                // only touch normal files
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

/// Run the main action.  This is separated from `main` so that it can get a generic
/// DisplayMode argument.
///
/// Spawns the walker thread and prints the results.
fn run<D: DisplayMode>(display: &mut D, opts: Opts) {
    let (w_chan, r_chan) = sync_channel(2 * opts.workers as usize);
    thread::spawn(move || {
        walk(w_chan, &opts);
    });
    while let Ok(r) = r_chan.recv() {
        display.print_result(r);
    }
}

/// Main entry point.
fn main() {
    let mut opts = Opts::from_cmdline();
    let colors = opts.colors.take().unwrap();  // guaranteed to be Some()

    // determine which display mode we are using
    if opts.only_count {
        run(&mut display::CountMode::new(colors), opts);
    } else if opts.only_files == Some(true) {
        run(&mut display::FilesOnlyMode::new(colors, true), opts);
    } else if opts.only_files == Some(false) {
        run(&mut display::FilesOnlyMode::new(colors, false), opts);
    } else if opts.ackmate_format {
        run(&mut display::AckMateMode::new(), opts);
    } else if opts.vimgrep_format {
        run(&mut display::VimGrepMode::new(), opts);
    } else {
        run(&mut display::DefaultMode::new(colors, opts.show_break, opts.show_heading), opts);
    }
}
