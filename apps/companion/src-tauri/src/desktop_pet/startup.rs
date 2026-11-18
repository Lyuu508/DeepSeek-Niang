//! Explicit current-user startup only. No installer or service changes this registry value.
use std::{ffi::OsString, io};
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE},
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn command_line() -> io::Result<OsString> {
    let mut value = OsString::from("\"");
    value.push(std::env::current_exe()?);
    value.push("\"");
    Ok(value)
}

pub fn enabled(app_id: &str) -> io::Result<bool> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    match root
        .open_subkey_with_flags(RUN_KEY, KEY_READ)
        .and_then(|key| key.get_value::<OsString, _>(app_id))
    {
        Ok(value) => Ok(value == command_line()?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn set(app_id: &str, enabled: bool) -> io::Result<()> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        return root
            .create_subkey(RUN_KEY)?
            .0
            .set_value(app_id, &command_line()?);
    }
    match root
        .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
        .and_then(|key| key.delete_value(app_id))
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
