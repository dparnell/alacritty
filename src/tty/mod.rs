#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod unix;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
pub use tty::unix::{Pty, process_should_exit};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use tty::windows::{Pty, process_should_exit};
