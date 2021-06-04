#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(type_name_of_val)]

pub mod version;
pub mod log;
pub mod certs;
pub mod config;
pub mod auth;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
