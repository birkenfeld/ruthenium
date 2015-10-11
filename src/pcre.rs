// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

// This file derived from rust-pcre:
// Copyright 2015 The rust-pcre authors.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::fmt;
use std::ptr;
use libc::{c_char, c_int, c_uchar, c_void};

mod ffi {
    use libc::{c_char, c_int, c_uchar, c_ulong, c_void};

    #[allow(non_camel_case_types)]
    pub type compile_options = c_int;
    #[allow(non_camel_case_types)]
    pub type exec_options = c_int;
    #[allow(non_camel_case_types)]
    pub type fullinfo_field = c_int;
    #[allow(non_camel_case_types)]
    pub type study_options = c_int;

    #[link(name = "pcre")]
    extern {
        pub static pcre_free: extern "C" fn(ptr: *mut c_void);

        pub fn pcre_compile(pattern: *const c_char, options: compile_options,
                            errptr: *mut *const c_char, erroffset: *mut c_int,
                            tableptr: *const c_uchar) -> *mut pcre;
        pub fn pcre_exec(code: *const pcre, extra: *const pcre_extra, subject: *const c_char,
                         length: c_int, startoffset: c_int, options: exec_options,
                         ovector: *mut c_int, ovecsize: c_int) -> c_int;
        pub fn pcre_dfa_exec(code: *const pcre, extra: *const pcre_extra,
                             subject: *const c_char, length: c_int, startoffset: c_int,
                             options: exec_options, ovector: *mut c_int, ovecsize: c_int,
                             workspace: *mut c_int, wscount: c_int) -> c_int;
        pub fn pcre_free_study(extra: *mut pcre_extra);
        pub fn pcre_fullinfo(code: *const pcre, extra: *const pcre_extra, what: fullinfo_field,
                             where_: *mut c_void) -> c_int;
        pub fn pcre_study(code: *const pcre, options: study_options,
                          errptr: *mut *const c_char) -> *mut pcre_extra;
        pub fn pcre_version() -> *const c_char;
    }

    pub const PCRE_UTF8: compile_options = 0x00000800;

    // PCRE_NO_UTF8_CHECK is both a compile and exec option
    pub const PCRE_NO_UTF8_CHECK: c_int = 0x00002000;

    pub const PCRE_ERROR_NOMATCH: c_int = -1;
    pub const PCRE_ERROR_NULL: c_int = -2;

    pub const PCRE_INFO_CAPTURECOUNT: fullinfo_field = 2;
    pub const PCRE_INFO_NAMEENTRYSIZE: fullinfo_field = 7;
    pub const PCRE_INFO_NAMECOUNT: fullinfo_field = 8;
    pub const PCRE_INFO_NAMETABLE: fullinfo_field = 9;

    pub const PCRE_STUDY_JIT_COMPILE: c_int = 0x0001;
    pub const PCRE_STUDY_JIT_PARTIAL_SOFT_COMPILE: c_int = 0x0002;
    pub const PCRE_STUDY_JIT_PARTIAL_HARD_COMPILE: c_int = 0x0004;
    pub const PCRE_STUDY_EXTRA_NEEDED: c_int = 0x0008;

    //const PCRE_EXTRA_STUDY_DATA: c_ulong = 0x0001;
    const PCRE_EXTRA_MATCH_LIMIT: c_ulong = 0x0002;
    //const PCRE_EXTRA_CALLOUT_DATA: c_ulong = 0x0004;
    //const PCRE_EXTRA_TABLES: c_ulong = 0x0008;
    const PCRE_EXTRA_MATCH_LIMIT_RECURSION: c_ulong = 0x0010;
    const PCRE_EXTRA_MARK: c_ulong = 0x0020;
    //const PCRE_EXTRA_EXECUTABLE_JIT: c_ulong = 0x0040;

    #[allow(non_camel_case_types)]
    pub enum pcre {}

    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct pcre_extra {
        flags: c_ulong,
        study_data: *mut c_void,
        match_limit: c_ulong,
        callout_data: *mut c_void,
        tables: *const c_uchar,
        match_limit_recursion_: c_ulong,
        mark: *mut *mut c_uchar,
        executable_jit: *mut c_void
    }

    impl pcre_extra {
        /// Returns the match limit, if previously set by [set_match_limit()](#method.set_match_limit).
        ///
        /// The default value for this limit is set when PCRE is built. The default default is 10 million.
        pub fn match_limit(&self) -> Option<usize> {
            if (self.flags & PCRE_EXTRA_MATCH_LIMIT) == 0 {
                None
            } else {
                Some(self.match_limit as usize)
            }
        }

        /// Sets the match limit to `limit` instead of using PCRE's default.
        pub fn set_match_limit(&mut self, limit: u32) {
            self.flags |= PCRE_EXTRA_MATCH_LIMIT;
            self.match_limit = limit as c_ulong;
        }

        /// Returns the recursion depth limit, if previously set by
        /// [set_match_limit_recursion()](#method.set_match_limit_recursion).
        ///
        /// The default value for this limit is set when PCRE is built.
        pub fn match_limit_recursion(&self) -> Option<usize> {
            if (self.flags & PCRE_EXTRA_MATCH_LIMIT_RECURSION) == 0 {
                None
            } else {
                Some(self.match_limit_recursion_ as usize)
            }
        }

        /// Sets the recursion depth limit to `limit` instead of using PCRE's default.
        pub fn set_match_limit_recursion(&mut self, limit: u32) {
            self.flags |= PCRE_EXTRA_MATCH_LIMIT_RECURSION;
            self.match_limit = limit as c_ulong;
        }
    }
}

pub unsafe fn pcre_compile(pattern: *const c_char, options: ffi::compile_options,
                           tableptr: *const c_uchar) -> Result<*mut ffi::pcre, (String, c_int)> {
    assert!(!pattern.is_null());
    // the pattern is always UTF-8
    let options = options | ffi::PCRE_UTF8 | ffi::PCRE_NO_UTF8_CHECK;
    let mut err: *const c_char = ptr::null();
    let mut erroffset: c_int = 0;
    let code = ffi::pcre_compile(pattern, options, &mut err, &mut erroffset, tableptr);

    if code.is_null() {
        // "Otherwise, if  compilation  of  a  pattern fails, pcre_compile() returns
        // NULL, and sets the variable pointed to by errptr to point to a textual
        // error message. This is a static string that is part of the library. You
        // must not try to free it."
        Err((CStr::from_ptr(err).to_string_lossy().into_owned(), erroffset))
    } else {
        assert!(!code.is_null());
        assert_eq!(erroffset, 0);
        Ok(code)
    }
}

pub unsafe fn pcre_exec(code: *const ffi::pcre, extra: *const ffi::pcre_extra,
                        subject: *const c_char, length: c_int, startoffset: c_int,
                        options: ffi::compile_options,
                        ovector: *mut c_int, ovecsize: c_int) -> Result<c_int, ()> {
    assert!(!code.is_null());
    assert!(ovecsize >= 0 && ovecsize % 3 == 0);
    let rc = ffi::pcre_exec(code, extra, subject, length, startoffset, options, ovector, ovecsize);
    if rc == ffi::PCRE_ERROR_NOMATCH {
        Ok(-1)
    } else if rc < 0 {
        Err(())
    } else {
        Ok(rc)
    }
}

pub unsafe fn pcre_dfa_exec(code: *const ffi::pcre, extra: *const ffi::pcre_extra,
                            subject: *const c_char, length: c_int, startoffset: c_int,
                            options: ffi::compile_options,
                            ovector: *mut c_int, ovecsize: c_int) -> Result<c_int, ()> {
    assert!(!code.is_null());
    assert!(ovecsize >= 0 && ovecsize % 3 == 0);
    let mut workspace = [0 as c_int; 30];
    let rc = ffi::pcre_dfa_exec(code, extra, subject, length, startoffset, options,
                                ovector, ovecsize,
                                workspace.as_mut_ptr() as *mut c_int,
                                workspace.len() as c_int);
    if rc == ffi::PCRE_ERROR_NOMATCH {
        Ok(-1)
    } else if rc < 0 {
        Err(())
    } else {
        Ok(rc)
    }
}

pub unsafe fn pcre_free(ptr: *mut c_void) {
    ffi::pcre_free(ptr);
}

pub unsafe fn pcre_free_study(extra: *mut ffi::pcre_extra) {
    ffi::pcre_free_study(extra);
}

pub unsafe fn pcre_fullinfo(code: *const ffi::pcre, extra: *const ffi::pcre_extra,
                            what: ffi::fullinfo_field, where_: *mut c_void) {
    assert!(!code.is_null());
    let rc = ffi::pcre_fullinfo(code, extra, what, where_);
    if rc < 0 && rc != ffi::PCRE_ERROR_NULL {
        panic!("pcre_fullinfo");
    }
}

pub unsafe fn pcre_study(code: *const ffi::pcre, options: ffi::study_options)
                         -> Result<*mut ffi::pcre_extra, String> {
    assert!(!code.is_null());
    let converted_options = options;
    let mut err: *const c_char = ptr::null();
    let extra = ffi::pcre_study(code, converted_options, &mut err);
    // "The third argument for pcre_study() is a pointer for an error message. If
    // studying succeeds (even if no data is returned), the variable it points to is
    // set to NULL. Otherwise it is set to point to a textual error message. This is
    // a static string that is part of the library. You must not try to free it."
    // http://pcre.org/pcre.txt
    if !err.is_null() {
        Err(CStr::from_ptr(err).to_string_lossy().into_owned())
    } else {
        assert!(err.is_null());
        Ok(extra)
    }
}

// pub fn pcre_version() -> String {
//     let version_cstr = unsafe { CStr::from_ptr(ffi::pcre_version()) };
//     version_cstr.to_string_lossy().into_owned()
// }

pub type Pcre = ffi::pcre;
pub type PcreExtra = ffi::pcre_extra;
pub type CompileOptions = ffi::compile_options;
pub type ExecOptions = ffi::exec_options;
pub type StudyOptions = ffi::study_options;

/// Wrapper for libpcre's `pcre` object (representing a compiled regular expression).
#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Regex {
    code: *const Pcre,
    extra: *mut PcreExtra,
    capture_count: c_int,
}

/// Represents a match of a subject string against a regular expression.
pub struct Match<'s> {
    subject: &'s [u8],
    partial_ovector: Vec<c_int>,
    string_count: c_int
}

/// Iterator type for iterating matches within a subject string.
pub struct MatchIterator<'r, 's> {
    regex: &'r Regex,
    subject: &'s [u8],
    offset: c_int,
    options: ExecOptions,
    ovector: Vec<c_int>
}

#[derive(Debug)]
pub struct CompilationError {
    err: String,
    erroffset: c_int
}

impl CompilationError {
    pub fn message(&self) -> Option<String> {
        Some(self.err.clone())
    }

    pub fn offset(&self) -> usize {
        self.erroffset as usize
    }
}

impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "compilation failed at offset {}: {}", self.erroffset, self.err)
    }
}

impl Regex {
    pub fn compile(pattern: &str) -> Result<Regex, CompilationError> {
        Regex::compile_with_options(pattern, 0)
    }

    pub fn from_regex(pattern: &str) -> Result<Regex, CompilationError> {
        Regex::compile_with_options(pattern, 0).map(|mut rx| {
            rx.study_with_options(ffi::PCRE_STUDY_JIT_COMPILE);
            rx
        })
    }

    pub fn compile_with_options(pattern: &str, options: CompileOptions)
                                -> Result<Regex, CompilationError> {
        let pattern_cstring = CString::new(pattern).unwrap();
        unsafe {
            // Use the default character tables.
            let tableptr: *const c_uchar = ptr::null();
            match pcre_compile(pattern_cstring.as_ptr(), options, tableptr) {
                Err((err, erroffset)) => Err(CompilationError {
                    err: err,
                    erroffset: erroffset
                }),
                Ok(mut_code) => {
                    let code = mut_code as *const Pcre;
                    assert!(!code.is_null());

                    let extra: *mut PcreExtra = ptr::null_mut();
                    let mut capture_count: c_int = 0;
                    pcre_fullinfo(code, extra as *const PcreExtra, ffi::PCRE_INFO_CAPTURECOUNT,
                                  &mut capture_count as *mut c_int as *mut c_void);

                    Ok(Regex {
                        code: code,
                        extra: extra,
                        capture_count: capture_count,
                    })
                }
            }
        }
    }

    pub fn capture_count(&self) -> usize {
        self.capture_count as usize
    }

    /// Returns the extra block, if one has been created.
    pub fn extra(&mut self) -> Option<&mut PcreExtra> {
        unsafe {
            if self.extra.is_null() {
                None
            } else {
                Some(&mut *(self.extra))
            }
        }
    }

    #[inline]
    pub fn exec<'a>(&self, subject: &'a [u8]) -> Option<Match<'a>> {
        self.exec_from(subject, 0)
    }

    #[inline]
    pub fn exec_from<'a>(&self, subject: &'a [u8], startoffset: usize) -> Option<Match<'a>> {
        self.exec_from_with_options(subject, startoffset, 0)
    }

    #[inline]
    pub fn exec_from_with_options<'a>(&self, subject: &'a [u8], startoffset: usize,
                                      options: ExecOptions) -> Option<Match<'a>> {
        let ovecsize = (self.capture_count + 1) * 3;
        let mut ovector = vec![0 as c_int; ovecsize as usize];

        let rc = unsafe {
            pcre_exec(self.code,
                      self.extra as *const PcreExtra,
                      subject.as_ptr() as *const c_char,
                      subject.len() as c_int,
                      startoffset as c_int,
                      options,
                      ovector.as_mut_ptr(),
                      ovecsize as c_int)
        };
        match rc {
            Ok(rc) if rc >= 0 => {
                Some(Match {
                    subject: subject,
                    partial_ovector: ovector[..(((self.capture_count + 1) * 2) as usize)].to_vec(),
                    string_count: rc
                })
            }
            _ => { None }
        }
    }

    /// For compatibility with regex-dfa.
    #[inline]
    pub fn shortest_match(&self, subject: &[u8]) -> Option<(usize, usize)> {
        self.exec(subject).map(|m| (m.group_start(0), m.group_end(0)))
    }

    #[inline]
    pub fn matches<'r, 's>(&'r self, subject: &'s [u8]) -> MatchIterator<'r, 's> {
        self.matches_with_options(subject, 0)
    }

    #[inline]
    pub fn matches_with_options<'r, 's>(&'r self, subject: &'s [u8], options: ExecOptions)
                                        -> MatchIterator<'r, 's> {
        let ovecsize = (self.capture_count + 1) * 3;
        MatchIterator {
            regex: self,
            subject: subject,
            offset: 0,
            options: options.clone(),
            ovector: vec![0 as c_int; ovecsize as usize]
        }
    }

    pub fn study(&mut self) -> bool {
        self.study_with_options(0)
    }

    pub fn study_with_options(&mut self, options: StudyOptions) -> bool {
        let extra = unsafe {
            // Free any current study data.
            pcre_free_study(self.extra as *mut PcreExtra);
            self.extra = ptr::null_mut();
            pcre_study(self.code, options)
        };
        match extra {
            Ok(extra) => {
                self.extra = extra;
                !extra.is_null()
            }
            Err(_) => false
        }
    }
}

impl Drop for Regex {
    fn drop(&mut self) {
        unsafe {
            pcre_free_study(self.extra as *mut PcreExtra);
            pcre_free(self.code as *mut Pcre as *mut c_void);
        }
        self.extra = ptr::null_mut();
        self.code = ptr::null();
    }
}

impl<'a> Match<'a> {
    /// Returns the start index within the subject string of capture group `n`.
    pub fn group_start(&self, n: usize) -> usize {
        self.partial_ovector[(n * 2) as usize] as usize
    }

    /// Returns the end index within the subject string of capture group `n`.
    pub fn group_end(&self, n: usize) -> usize {
        self.partial_ovector[(n * 2 + 1) as usize] as usize
    }

    /// Returns the length of the substring for capture group `n`.
    pub fn group_len(&self, n: usize) -> usize {
        let group_offsets = &self.partial_ovector[((n * 2) as usize)..];
        (group_offsets[1] - group_offsets[0]) as usize
    }

    /// Returns the substring for capture group `n` as a slice.
    #[inline]
    pub fn group(&'a self, n: usize) -> &'a [u8] {
        let group_offsets = &self.partial_ovector[((n * 2) as usize)..];
        let start = group_offsets[0];
        let end = group_offsets[1];
        &self.subject[(start as usize)..(end as usize)]
    }

    /// Returns the number of substrings captured.
    pub fn string_count(&self) -> usize {
        self.string_count as usize
    }
}

impl<'r, 's> Clone for MatchIterator<'r, 's> {
    #[inline]
    fn clone(&self) -> MatchIterator<'r, 's> {
        MatchIterator {
            regex: self.regex,
            subject: self.subject,
            offset: self.offset,
            options: self.options.clone(),
            ovector: self.ovector.clone()
        }
    }
}

impl<'r, 's> Iterator for MatchIterator<'r, 's> {
    type Item = Match<'s>;

    /// Gets the next match.
    #[inline]
    fn next(&mut self) -> Option<Match<'s>> {
        let rc = unsafe {
            pcre_exec(self.regex.code,
                      self.regex.extra,
                      self.subject.as_ptr() as *const c_char,
                      self.subject.len() as c_int,
                      self.offset,
                      self.options,
                      self.ovector.as_mut_ptr(),
                      self.ovector.len() as c_int)
        };
        match rc {
            Ok(rc) if rc >= 0 => {
                // Update the iterator state.
                self.offset = self.ovector[1];

                let cc = self.regex.capture_count;
                Some(Match {
                    subject: self.subject,
                    partial_ovector: self.ovector[..(((cc + 1) * 2) as usize)].to_vec(),
                    string_count: rc
                })
            }
            _ => None
        }
    }
}

/// Read-only access is guaranteed to be thread-safe.
unsafe impl Sync for Regex {}
