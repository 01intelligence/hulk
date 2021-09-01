use crate::errors::StorageError;

pub fn check_path_length(path_name: &str) -> anyhow::Result<(), StorageError> {
    // Apple OS X path length is limited to 1016.
    if cfg!(macos) && path_name.len() > 1016 {
        return Err(StorageError::FileNameTooLong);
    }

    // Disallow more than 1024 characters on windows, there
    // are no known name_max limits on Windows.
    if cfg!(windows) && path_name.len() > 1024 {
        return Err(StorageError::FileNameTooLong);
    }

    // On Unix we reject paths if they are just '.', '..' or '/'.
    if path_name == "." || path_name == ".." || path_name == crate::globals::SLASH_SEPARATOR {
        return Err(StorageError::FileAccessDenied);
    }

    // Check each path segment length is > 255 on all Unix
    // platforms, look for this value as NAME_MAX in
    // /usr/include/linux/limits.h
    let mut count = 0;
    for p in path_name.chars() {
        match p {
            '/' => {
                count = 0;
            }
            '\\' => {
                if cfg!(windows) {
                    count = 0;
                }
            }
            _ => {
                count += 1;
                if count > 255 {
                    return Err(StorageError::FileNameTooLong);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Check path length restrictions are not same on windows/darwin
    #[cfg(target_family = "unix")]
    fn test_check_path_length() {
        let cases: [(&str, bool, StorageError); 5] = [
            (".", true, StorageError::FileAccessDenied),
            ("/", true, StorageError::FileAccessDenied),
            ("..", true, StorageError::FileAccessDenied),
            ("data/G_792/srv-tse/c/users/denis/documents/gestion!20locative/heritier/propri!E9taire/20190101_a2.03!20-!20m.!20heritier!20re!B4mi!20-!20proce!60s-verbal!20de!20livraison!20et!20de!20remise!20des!20cle!B4s!20acque!B4reurs!20-!204-!20livraison!20-!20lp!20promotion!20toulouse!20-!20encre!20et!20plume!20-!205!20de!B4c.!202019!20a!60!2012-49.pdf.ecc", true, StorageError::FileNameTooLong),
            ("data/G_792/srv-tse/c/users/denis/documents/gestionlocative.txt", false, StorageError::Unexpected),
        ];
        for (path, is_err, expected_err) in cases.iter() {
            let result = check_path_length(path);
            match result {
                Ok(result) => {
                    assert!(!is_err);
                    assert_eq!(result, ())
                }
                Err(err) => {
                    assert!(is_err);
                    assert_eq!(err, *expected_err)
                }
            }
        }
    }
}
