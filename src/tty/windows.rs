// Copyright 2016 Joe Wilm, The Alacritty Project Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//! windows console related functionality
//!
use std::ffi::CStr;
use std::fs::File;
use std::process::{Command, Stdio};
use libc::{self, c_int};

use term::SizeInfo;
use display::{OnResize};
use config::{Config, Shell};
use cli::Options;

static mut SHOULD_EXIT: bool = false;

struct winsize {
    ws_row: libc::c_ushort,
    ws_col: libc::c_ushort,
    ws_xpixel: libc::c_ushort,
    ws_ypixel: libc::c_ushort,
}


pub fn process_should_exit() -> bool {
    unsafe { SHOULD_EXIT }
}

/// Create a new process and return a handle to interact with it.
pub fn new<T: ToWinsize>(config: &Config, options: &Options, size: T, window_id: Option<usize>) -> Pty {
    // TODO: start the shell process
}


pub struct Pty {
    fd: c_int,
}

impl Pty {
    /// Get reader for the TTY
    ///
    pub fn reader(&self) -> File {
        unsafe {
            File::from_raw_fd(self.fd)
        }
    }

    /// Resize the pty
    ///
    pub fn resize<T: ToWinsize>(&self, size: T) {
        // do nothing
    }
}

/// Types that can produce a `libc::winsize`
pub trait ToWinsize {
    /// Get a `libc::winsize`
    fn to_winsize(&self) -> winsize;
}

impl<'a> ToWinsize for &'a SizeInfo {
    fn to_winsize(&self) -> winsize {
        winsize {
            ws_row: self.lines().0 as libc::c_ushort,
            ws_col: self.cols().0 as libc::c_ushort,
            ws_xpixel: self.width as libc::c_ushort,
            ws_ypixel: self.height as libc::c_ushort,
        }
    }
}

impl OnResize for Pty {
    fn on_resize(&mut self, size: &SizeInfo) {
        self.resize(size);
    }
}
