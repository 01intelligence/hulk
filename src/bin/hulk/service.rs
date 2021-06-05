use lazy_static;
use std::convert::Infallible;
use std::ffi::{CStr, CString};

enum ServiceSignal {
    Restart,
    Stop,
    ReloadDynamic,
}

lazy_static! {
    static ref globalServiceSignalCh: async_std::channel::Channel = async_std::channel::unbounded();
}

pub fn restart_process() -> anyhow::Result<Infallible> {
    let args: Vec<String> = std::env::args().collect();
    let path = which::which(&args[0]).unwrap();
    let path = CString::new(path.to_str().unwrap()).unwrap();
    let args: Vec<CString> = args
        .iter()
        .map(|a| CString::new(a as &str).unwrap())
        .collect();
    let envs: Vec<CString> = std::env::vars()
        .map(|(k, v)| CString::new(format!("{}={}", k, v).as_str()).unwrap())
        .collect();
    let infallible = nix::unistd::execve(&path, &args, &envs)?;
    Ok(infallible)
}
