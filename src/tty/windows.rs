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
    let win = size.to_winsize();
    let mut buf = [0; 1024];

    let shell = config.shell();

    let initial_command = options.command().unwrap_or(&shell);

    let mut builder = Command::new(initial_command.program());
    for arg in initial_command.args() {
        builder.arg(arg);
    }

    // Setup child stdin/stdout/stderr as slave fd of pty
    // Ownership of fd is transferred to the Stdio structs and will be closed by them at the end of
    // this scope. (It is not an issue that the fd is closed three times since File::drop ignores
    // error on libc::close.)
    builder.stdin(unsafe { Stdio::from_raw_fd(slave) });
    builder.stderr(unsafe { Stdio::from_raw_fd(slave) });
    builder.stdout(unsafe { Stdio::from_raw_fd(slave) });

    // Setup environment
    builder.env("TERM", "xterm-256color"); // default term until we can supply our own
    if let Some(window_id) = window_id {
        builder.env("WINDOWID", format!("{}", window_id));
    }
    for (key, value) in config.env().iter() {
        builder.env(key, value);
    }

    builder.before_exec(move || {
        // Create a new process group
        unsafe {
            let err = libc::setsid();
            if err == -1 {
                die!("Failed to set session id: {}", errno());
            }
        }

        set_controlling_terminal(slave);

        // No longer need slave/master fds
        unsafe {
            libc::close(slave);
            libc::close(master);
        }

        Ok(())
    });

    // Handle set working directory option
    if let Some(ref dir) = options.working_dir {
        builder.current_dir(dir.as_path());
    }

    match builder.spawn() {
        Ok(child) => {
            unsafe {
                // Set PID for SIGCHLD handler
                PID = child.id() as _;
            }

            let pty = Pty { fd: master };
            pty.resize(size);
            pty
        },
        Err(err) => {
            die!("Command::spawn() failed: {}", err);
        }
    }
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
