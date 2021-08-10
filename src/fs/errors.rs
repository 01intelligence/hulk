pub fn err_no_space(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::ENOSPC)
}

pub fn err_invalid_arg(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::InvalidInput {
        return true;
    }
    is_libc_err(err, libc::EINVAL)
}

pub fn err_io(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::EIO)
}

pub fn err_is_dir(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::EISDIR)
}

pub fn err_not_dir(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::ENOTDIR)
}

pub fn err_too_long(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::ENAMETOOLONG)
}

pub fn err_too_many_symlinks(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::ELOOP)
}

pub fn err_dir_not_empty(err: &std::io::Error) -> bool {
    if is_libc_err(err, libc::ENOTEMPTY) {
        return true;
    }
    if cfg!(windows) {
        return is_libc_err(err, 0x91);
    }
    false
}

pub fn err_not_found(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::NotFound {
        return true;
    }
    if cfg!(windows) {
        return is_libc_err(err, 6); // ERROR_INVALID_HANDLE
    }
    false
}

pub fn err_cross_device(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::EXDEV)
}

pub fn err_too_many_files(err: &std::io::Error) -> bool {
    is_libc_err(err, libc::ENFILE) || is_libc_err(err, libc::EMFILE)
}

pub fn err_permission(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::PermissionDenied
}

pub fn err_already_exists(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::AlreadyExists
}

fn is_libc_err(err: &std::io::Error, libc_err: i32) -> bool {
    if let Some(err) = err.raw_os_error() {
        err == libc_err
    } else {
        false
    }
}
