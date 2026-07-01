// Rust ignores SIGPIPE by default, which turns a closed downstream pipe (e.g. `| head`)
// into a "Broken pipe" I/O error that println! panics on. Restore the default disposition
// so the process instead exits silently, like a normal Unix tool.
#[cfg(unix)]
pub fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
pub fn reset_sigpipe() {}
