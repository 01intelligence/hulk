#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(type_name_of_val)]

pub mod auth;
pub mod certs;
pub mod config;
pub mod disk;
pub mod dsync;
pub mod jwt;
pub mod log;
pub mod strset;
pub mod trie;
pub mod version;
pub mod wildcard;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
