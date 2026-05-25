#[allow(clippy::upper_case_acronyms)]
pub type PCWSTR = *const u16;

#[allow(non_camel_case_types)]
pub type FILE_FLAGS_AND_ATTRIBUTES = u32;

pub const FILE_ATTRIBUTE_HIDDEN: FILE_FLAGS_AND_ATTRIBUTES = 0x00000002;
pub const INVALID_FILE_ATTRIBUTES: FILE_FLAGS_AND_ATTRIBUTES = 0xFFFFFFFF;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetFileAttributesW(lpFileName: PCWSTR) -> u32;
    pub fn SetFileAttributesW(
        lpFileName: PCWSTR,
        dwFileAttributes: FILE_FLAGS_AND_ATTRIBUTES,
    ) -> i32;
}
