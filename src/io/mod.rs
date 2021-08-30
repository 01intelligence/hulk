mod buf_reader;
mod pipe;
mod read_ahead;
mod read_at;
mod read_full;

pub use buf_reader::*;
pub use pipe::*;
pub use read_ahead::*;
pub use read_at::*;
pub use read_full::*;

/// Repeats operations that are interrupted
#[macro_export]
macro_rules! uninterruptibly {
    ($e:expr) => {{
        loop {
            match $e {
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                res => break res,
            }
        }
    }};
}
