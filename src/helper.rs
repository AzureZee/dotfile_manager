use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use crate::winapi::*;

pub fn hide_dotfile_in_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    if !path.as_ref().is_dir() {
        return Err(io::Error::other("no dir"));
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
    let path_str = to_wide(path_to_str(path.as_ref())?);
    let path_ptr = path_str.as_ptr();
    if !is_hidden(path_ptr)? {
        hide(path_ptr)?;
    }
    Ok(())
}

pub fn path_to_str(path: &Path) -> io::Result<&str> {
    path.to_str().ok_or(io::Error::other("invalid char"))
}

pub fn check(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn hide(path_ptr: PCWSTR) -> io::Result<()> {
    let attrs = get_attrs(path_ptr)?;

    set_attrs(path_ptr, attrs | FILE_ATTRIBUTE_HIDDEN)
}

#[allow(unused)]
pub fn unhide(path_ptr: PCWSTR) -> io::Result<()> {
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

    let ignore = dir.join(".gitignore");
    let git_dir = dir.join(".git");

    let git_dir = to_wide(path_to_str(&git_dir)?);
    let ignore = to_wide(path_to_str(&ignore)?);

    let yes = is_hidden(git_dir.as_ptr())?;
    let no = is_hidden(ignore.as_ptr())?;

    assert!(yes);
    assert!(!no);

    let path_ptr = ignore.as_ptr();

    hide(path_ptr)?;
    let now_yes = is_hidden(path_ptr)?;
    assert!(now_yes);

    unhide(path_ptr)?;
    let now_no = is_hidden(path_ptr)?;
    assert!(!now_no);
    Ok(())
}
