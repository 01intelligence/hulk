//! Support for capturing a stack backtrace of an OS thread
//!
//! This module is a clone of std::backtrace, to export
//! struct BacktraceFrame and its fields.

use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Mutex, Once};
use std::{env, fmt};

use backtrace::BytesOrWideString;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref LOCK: Mutex<()> = Mutex::new(());
}

/// A captured OS thread stack backtrace.
///
/// This type represents a stack backtrace for an OS thread captured at a
/// previous point in time. In some instances the `Backtrace` type may
/// internally be empty due to configuration. For more information see
/// `Backtrace::capture`.
pub struct Backtrace {
    pub inner: Inner,
}

/// The current status of a backtrace, indicating whether it was captured or
/// whether it is empty for some other reason.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum BacktraceStatus {
    /// Capturing a backtrace is not supported, likely because it's not
    /// implemented for the current platform.
    Unsupported,
    /// Capturing a backtrace has been disabled through either the
    /// `RUST_LIB_BACKTRACE` or `RUST_BACKTRACE` environment variables.
    Disabled,
    /// A backtrace has been captured and the `Backtrace` should print
    /// reasonable information when rendered.
    Captured,
}

pub enum Inner {
    Unsupported,
    Disabled,
    Captured(LazilyResolvedCapture),
}

pub struct Capture {
    pub actual_start: usize,
    resolved: bool,
    pub frames: Vec<BacktraceFrame>,
}

pub struct BacktraceFrame {
    pub frame: RawFrame,
    pub symbols: Vec<BacktraceSymbol>,
}

#[derive(Debug)]
pub enum RawFrame {
    Actual(backtrace::Frame),
    #[cfg(test)]
    Fake,
}

pub struct BacktraceSymbol {
    pub name: Option<Vec<u8>>,
    pub filename: Option<BytesOrWide>,
    pub lineno: Option<u32>,
    pub colno: Option<u32>,
}

pub enum BytesOrWide {
    Bytes(Vec<u8>),
    Wide(Vec<u16>),
}

impl fmt::Debug for BytesOrWide {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        output_filename(
            fmt,
            match self {
                BytesOrWide::Bytes(w) => BytesOrWideString::Bytes(w),
                BytesOrWide::Wide(w) => BytesOrWideString::Wide(w),
            },
            backtrace::PrintFmt::Short,
            env::current_dir().as_ref().ok(),
        )
    }
}

impl Backtrace {
    /// Returns whether backtrace captures are enabled through environment
    /// variables.
    fn enabled() -> bool {
        // Cache the result of reading the environment variables to make
        // backtrace captures speedy, because otherwise reading environment
        // variables every time can be somewhat slow.
        static ENABLED: AtomicUsize = AtomicUsize::new(0);
        match ENABLED.load(SeqCst) {
            0 => {}
            1 => return false,
            _ => return true,
        }
        let enabled = match env::var("RUST_LIB_BACKTRACE") {
            Ok(s) => s != "0",
            Err(_) => match env::var("RUST_BACKTRACE") {
                Ok(s) => s != "0",
                Err(_) => false,
            },
        };
        ENABLED.store(enabled as usize + 1, SeqCst);
        enabled
    }

    /// Capture a stack backtrace of the current thread.
    ///
    /// This function will capture a stack backtrace of the current OS thread of
    /// execution, returning a `Backtrace` type which can be later used to print
    /// the entire stack trace or render it to a string.
    ///
    /// This function will be a noop if the `RUST_BACKTRACE` or
    /// `RUST_LIB_BACKTRACE` backtrace variables are both not set. If either
    /// environment variable is set and enabled then this function will actually
    /// capture a backtrace. Capturing a backtrace can be both memory intensive
    /// and slow, so these environment variables allow liberally using
    /// `Backtrace::capture` and only incurring a slowdown when the environment
    /// variables are set.
    ///
    /// To forcibly capture a backtrace regardless of environment variables, use
    /// the `Backtrace::force_capture` function.
    #[inline(never)] // want to make sure there's a frame here to remove
    pub fn capture() -> Backtrace {
        if !Backtrace::enabled() {
            return Backtrace {
                inner: Inner::Disabled,
            };
        }
        Backtrace::create(Backtrace::capture as usize)
    }

    /// Forcibly captures a full backtrace, regardless of environment variable
    /// configuration.
    ///
    /// This function behaves the same as `capture` except that it ignores the
    /// values of the `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE` environment
    /// variables, always capturing a backtrace.
    ///
    /// Note that capturing a backtrace can be an expensive operation on some
    /// platforms, so this should be used with caution in performance-sensitive
    /// parts of code.
    #[inline(never)] // want to make sure there's a frame here to remove
    pub fn force_capture() -> Backtrace {
        Backtrace::create(Backtrace::force_capture as usize)
    }

    /// Forcibly captures a disabled backtrace, regardless of environment
    /// variable configuration.
    pub const fn disabled() -> Backtrace {
        Backtrace {
            inner: Inner::Disabled,
        }
    }

    // Capture a backtrace which start just before the function addressed by
    // `ip`
    fn create(ip: usize) -> Backtrace {
        // SAFETY: We don't attempt to lock this reentrantly.
        let _lock = LOCK.lock().unwrap();
        let mut frames = Vec::new();
        let mut actual_start = None;
        unsafe {
            backtrace::trace_unsynchronized(|frame| {
                frames.push(BacktraceFrame {
                    frame: RawFrame::Actual(frame.clone()),
                    symbols: Vec::new(),
                });
                if frame.symbol_address() as usize == ip && actual_start.is_none() {
                    actual_start = Some(frames.len());
                }
                true
            });
        }

        // If no frames came out assume that this is an unsupported platform
        // since `backtrace` doesn't provide a way of learning this right now,
        // and this should be a good enough approximation.
        let inner = if frames.is_empty() {
            Inner::Unsupported
        } else {
            Inner::Captured(LazilyResolvedCapture::new(Capture {
                actual_start: actual_start.unwrap_or(0),
                frames,
                resolved: false,
            }))
        };

        Backtrace { inner }
    }

    /// Returns the status of this backtrace, indicating whether this backtrace
    /// request was unsupported, disabled, or a stack trace was actually
    /// captured.
    pub fn status(&self) -> BacktraceStatus {
        match self.inner {
            Inner::Unsupported => BacktraceStatus::Unsupported,
            Inner::Disabled => BacktraceStatus::Disabled,
            Inner::Captured(_) => BacktraceStatus::Captured,
        }
    }
}

pub struct LazilyResolvedCapture {
    sync: Once,
    capture: UnsafeCell<Capture>,
}

impl LazilyResolvedCapture {
    fn new(capture: Capture) -> Self {
        LazilyResolvedCapture {
            sync: Once::new(),
            capture: UnsafeCell::new(capture),
        }
    }

    pub fn force(&self) -> &Capture {
        self.sync.call_once(|| {
            // SAFETY: This exclusive reference can't overlap with any others
            // `Once` guarantees callers will block until this closure returns
            // `Once` also guarantees only a single caller will enter this closure
            unsafe { &mut *self.capture.get() }.resolve();
        });

        // SAFETY: This shared reference can't overlap with the exclusive reference above
        unsafe { &*self.capture.get() }
    }
}

// SAFETY: Access to the inner value is synchronized using a thread-safe `Once`
// So long as `Capture` is `Sync`, `LazilyResolvedCapture` is too
unsafe impl Sync for LazilyResolvedCapture where Capture: Sync {}

impl Capture {
    fn resolve(&mut self) {
        // If we're already resolved, nothing to do!
        if self.resolved {
            return;
        }
        self.resolved = true;

        // Use the global backtrace lock to synchronize this as it's a
        // requirement of the `backtrace` crate, and then actually resolve
        // everything.
        // SAFETY: We don't attempt to lock this reentrantly.
        let _lock = LOCK.lock().unwrap();
        for frame in self.frames.iter_mut() {
            let symbols = &mut frame.symbols;
            let frame = match &frame.frame {
                RawFrame::Actual(frame) => frame,
                #[cfg(test)]
                RawFrame::Fake => unimplemented!(),
            };
            unsafe {
                backtrace::resolve_frame_unsynchronized(frame, |symbol| {
                    symbols.push(BacktraceSymbol {
                        name: symbol.name().map(|m| m.as_bytes().to_vec()),
                        filename: symbol.filename_raw().map(|b| match b {
                            BytesOrWideString::Bytes(b) => BytesOrWide::Bytes(b.to_owned()),
                            BytesOrWideString::Wide(b) => BytesOrWide::Wide(b.to_owned()),
                        }),
                        lineno: symbol.lineno(),
                        colno: symbol.colno(),
                    });
                });
            }
        }
    }
}

impl RawFrame {
    pub fn ip(&self) -> *mut c_void {
        match self {
            RawFrame::Actual(frame) => frame.ip(),
            #[cfg(test)]
            RawFrame::Fake => 1 as *mut c_void,
        }
    }
}

impl fmt::Display for Backtrace {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let capture = match &self.inner {
            Inner::Unsupported => return fmt.write_str("unsupported backtrace"),
            Inner::Disabled => return fmt.write_str("disabled backtrace"),
            Inner::Captured(c) => c.force(),
        };

        let full = fmt.alternate();
        let (frames, style) = if full {
            (&capture.frames[..], backtrace::PrintFmt::Full)
        } else {
            (
                &capture.frames[capture.actual_start..],
                backtrace::PrintFmt::Short,
            )
        };

        // When printing paths we try to strip the cwd if it exists, otherwise
        // we just print the path as-is. Note that we also only do this for the
        // short format, because if it's full we presumably want to print
        // everything.
        let cwd = env::current_dir();
        let mut print_path = move |fmt: &mut fmt::Formatter<'_>, path: BytesOrWideString<'_>| {
            output_filename(fmt, path, style, cwd.as_ref().ok())
        };

        let mut f = backtrace::BacktraceFmt::new(fmt, style, &mut print_path);
        f.add_context()?;
        for frame in frames {
            let mut f = f.frame();
            if frame.symbols.is_empty() {
                f.print_raw(frame.frame.ip(), None, None, None)?;
            } else {
                for symbol in frame.symbols.iter() {
                    f.print_raw_with_column(
                        frame.frame.ip(),
                        symbol.name.as_ref().map(|b| backtrace::SymbolName::new(b)),
                        symbol.filename.as_ref().map(|b| match b {
                            BytesOrWide::Bytes(w) => BytesOrWideString::Bytes(w),
                            BytesOrWide::Wide(w) => BytesOrWideString::Wide(w),
                        }),
                        symbol.lineno,
                        symbol.colno,
                    )?;
                }
            }
        }
        f.finish()?;
        Ok(())
    }
}

/// Prints the filename of the backtrace frame.
pub fn output_filename(
    fmt: &mut fmt::Formatter<'_>,
    bows: BytesOrWideString<'_>,
    print_fmt: backtrace::PrintFmt,
    cwd: Option<&PathBuf>,
) -> fmt::Result {
    let file: std::borrow::Cow<'_, Path> = match bows {
        #[cfg(unix)]
        BytesOrWideString::Bytes(bytes) => {
            use std::os::unix::prelude::*;
            Path::new(std::ffi::OsStr::from_bytes(bytes)).into()
        }
        #[cfg(not(unix))]
        BytesOrWideString::Bytes(bytes) => {
            Path::new(std::str::from_utf8(bytes).unwrap_or("<unknown>")).into()
        }
        #[cfg(windows)]
        BytesOrWideString::Wide(wide) => {
            use std::os::windows::prelude::*;
            std::borrow::Cow::Owned(std::ffi::OsString::from_wide(wide).into())
        }
        #[cfg(not(windows))]
        BytesOrWideString::Wide(_wide) => Path::new("<unknown>").into(),
    };
    if print_fmt == backtrace::PrintFmt::Short && file.is_absolute() {
        if let Some(cwd) = cwd {
            if let Ok(stripped) = file.strip_prefix(&cwd) {
                if let Some(s) = stripped.to_str() {
                    return write!(fmt, ".{}{}", std::path::MAIN_SEPARATOR, s);
                }
            }
        }
    }
    fmt::Display::fmt(&file.display(), fmt)
}
