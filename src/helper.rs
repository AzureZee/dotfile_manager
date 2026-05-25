use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use crate::winapi::*;

pub fn hide_dotfile_in_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    if !path.as_ref().is_dir() {
        return Err(io::Error::from(io::ErrorKind::NotADirectory));
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name_string = entry.file_name();
        let name_str = name_string.to_string_lossy();

        if name_str.starts_with('.') {
            let path = entry.path();
            hide_single(&path)?;
        }
    }
    Ok(())
}

pub fn hide_single<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path_str = to_wide(path.as_ref().as_os_str());
    let path_ptr = path_str.as_ptr();
    if !is_hidden(path_ptr)? {
        set_hidden(path_ptr)?;
    }
    Ok(())
}

pub fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

pub fn set_hidden(path_ptr: PCWSTR) -> io::Result<()> {
    let attrs = get_attrs(path_ptr)?;

    set_attrs(path_ptr, attrs | FILE_ATTRIBUTE_HIDDEN)
}

#[allow(unused)]
pub fn unset_hidden(path_ptr: PCWSTR) -> io::Result<()> {
    let attrs = get_attrs(path_ptr)?;

    set_attrs(path_ptr, attrs & !FILE_ATTRIBUTE_HIDDEN)
}

pub fn is_hidden(path_ptr: PCWSTR) -> io::Result<bool> {
    let attrs = get_attrs(path_ptr)?;

    Ok((attrs & FILE_ATTRIBUTE_HIDDEN) != 0)
}

pub fn get_attrs(path_ptr: PCWSTR) -> io::Result<FILE_FLAGS_AND_ATTRIBUTES> {
    let attrs = unsafe { GetFileAttributesW(path_ptr) };
    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(io::Error::last_os_error());
    }
    Ok(attrs)
}

pub fn set_attrs(path_ptr: PCWSTR, new_attrs: FILE_FLAGS_AND_ATTRIBUTES) -> io::Result<()> {
    unsafe {
        let result = SetFileAttributesW(path_ptr, new_attrs);
        check(result)
    }
}

#[cfg(target_os = "windows")]
#[test]
fn test_hide() -> io::Result<()> {
    use std::path::PathBuf;
    let dir = env!("CARGO_MANIFEST_DIR");
    let dir = PathBuf::from(dir);

    let path_gitignore = dir.join(".gitignore");
    let path_git = dir.join(".git");

    let path_str = to_wide(path_gitignore.as_os_str());
    let gitignore = path_str.as_ptr();

    let yes = is_hidden(to_wide(path_git.as_os_str()).as_ptr())?;
    let no = is_hidden(gitignore)?;

    assert!(yes);
    assert!(!no);

    set_hidden(gitignore)?;
    let now_yes = is_hidden(gitignore)?;
    assert!(now_yes);

    unset_hidden(gitignore)?;
    let now_no = is_hidden(gitignore)?;
    assert!(!now_no);
    Ok(())
}
