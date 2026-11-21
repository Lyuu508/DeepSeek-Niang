use companion_update::Download;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, WAIT_OBJECT_0},
    Security::WinTrust::{
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO,
        WTD_CHOICE_FILE, WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY,
        WTD_UI_NONE, WinVerifyTrust,
    },
    Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
    },
    System::Threading::{OpenProcess, WaitForSingleObject},
};

type Reply<T> = Result<T, &'static str>;
pub const WORKER_ARG: &str = "--apply-downloaded-update";
const WORKER_NAME: &str = "update-worker.exe";
const APP_NAME: &str = "companion-app.exe";

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain([0]).collect()
}

pub fn verify(path: &Path) -> Reply<()> {
    let path = wide(path.as_os_str());
    let mut info = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: path.as_ptr(),
        ..Default::default()
    };
    let mut trust = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut info },
        dwStateAction: WTD_STATEACTION_VERIFY,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // WinTrust owns its provider state until WTD_STATEACTION_CLOSE is sent.
    let success = unsafe {
        WinVerifyTrust(
            null_mut(),
            &mut action,
            (&mut trust as *mut WINTRUST_DATA).cast(),
        )
    } == 0;
    trust.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            null_mut(),
            &mut action,
            (&mut trust as *mut WINTRUST_DATA).cast(),
        );
    }
    success.then_some(()).ok_or("UPDATE_SIGNATURE_INVALID")
}

#[derive(Serialize, Deserialize)]
struct Job {
    original: PathBuf,
    parent_pid: u32,
    sha256: String,
    version: String,
}

fn launch_error<T>(_: T) -> &'static str {
    "UPDATE_INSTALLER_FAILED"
}

pub fn start(download: &Download) -> Reply<()> {
    let original = std::env::current_exe().map_err(launch_error)?;
    if original.file_name() != Some(OsStr::new(APP_NAME)) {
        return Err("UPDATE_INSTALLER_FAILED");
    }
    let stage = download.path.parent().ok_or("UPDATE_STORAGE_FAILED")?;
    let worker = stage.join(WORKER_NAME);
    fs::copy(&original, &worker).map_err(launch_error)?;
    let loader = original.with_file_name("WebView2Loader.dll");
    if loader.is_file() {
        fs::copy(loader, stage.join("WebView2Loader.dll")).map_err(launch_error)?;
    }
    let job = Job {
        original,
        parent_pid: std::process::id(),
        sha256: download.sha256.clone(),
        version: download.version.clone(),
    };
    fs::write(
        stage.join("job.json"),
        serde_json::to_vec(&job).map_err(launch_error)?,
    )
    .map_err(launch_error)?;
    Command::new(worker)
        .arg(WORKER_ARG)
        .spawn()
        .map_err(launch_error)?;
    Ok(())
}

fn installed_version(path: &Path) -> Option<String> {
    let path = wide(path.as_os_str());
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), null_mut()) };
    if size == 0 || size > 8 * 1024 * 1024 {
        return None;
    }
    // Use aligned storage because the query can return a pointer into this buffer.
    let mut buffer = vec![0_u64; (size as usize).div_ceil(8)];
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, buffer.as_mut_ptr().cast()) } == 0 {
        return None;
    }
    let mut info = null_mut();
    let mut length = 0;
    if unsafe {
        VerQueryValueW(
            buffer.as_ptr().cast(),
            wide(OsStr::new("\\")).as_ptr(),
            &mut info,
            &mut length,
        )
    } == 0
        || info.is_null()
        || length < size_of::<VS_FIXEDFILEINFO>() as u32
    {
        return None;
    }
    let info = unsafe { info.cast::<VS_FIXEDFILEINFO>().read_unaligned() };
    Some(format!(
        "{}.{}.{}",
        info.dwFileVersionMS >> 16,
        info.dwFileVersionMS & 0xffff,
        info.dwFileVersionLS >> 16
    ))
}

fn await_exit(pid: u32) -> Reply<()> {
    // SYNCHRONIZE access is sufficient. If the parent has already exited there
    // is nothing to wait for, and no handle survives this function.
    let process = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if process.is_null() {
        return Ok(());
    }
    let result = unsafe { WaitForSingleObject(process, 60_000) };
    unsafe {
        CloseHandle(process);
    }
    (result == WAIT_OBJECT_0)
        .then_some(())
        .ok_or("UPDATE_INSTALLER_FAILED")
}

fn restore(original: &Path, worker: &Path, folder: &Path) -> Reply<()> {
    let mut previous = tempfile::NamedTempFile::new_in(folder).map_err(launch_error)?;
    std::io::copy(
        &mut fs::File::open(worker).map_err(launch_error)?,
        &mut previous,
    )
    .map_err(launch_error)?;
    previous.flush().map_err(launch_error)?;
    previous.as_file().sync_all().map_err(launch_error)?;
    previous.persist(original).map_err(launch_error)?;
    Ok(())
}

pub fn run_worker() -> Reply<()> {
    let worker = std::env::current_exe().map_err(launch_error)?;
    if worker.file_name() != Some(OsStr::new(WORKER_NAME)) {
        return Err("UPDATE_INSTALLER_FAILED");
    }
    let stage = worker.parent().ok_or("UPDATE_STORAGE_FAILED")?;
    let cache = stage.parent().ok_or("UPDATE_STORAGE_FAILED")?;
    if cache.file_name() != Some(OsStr::new("companion-updates")) {
        return Err("UPDATE_INSTALLER_FAILED");
    }
    let failed = cache.join("install-failed");
    let job: Job = serde_json::from_slice(&fs::read(stage.join("job.json")).map_err(launch_error)?)
        .map_err(launch_error)?;
    let _ = fs::remove_file(stage.join("job.json"));
    if job.original.file_name() != Some(OsStr::new(APP_NAME))
        || !job.original.is_absolute()
        || job.version.split('.').count() != 3
        || !job.version.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    {
        return Err("UPDATE_INSTALLER_FAILED");
    }
    let target = job.original.parent().ok_or("UPDATE_INSTALLER_FAILED")?;
    let installer = stage.join(format!(
        "DeepSeek-Niang_{}_windows-x64-setup.exe",
        job.version
    ));
    await_exit(job.parent_pid)?;
    let result = (|| {
        companion_update::verify_file(&installer, &job.sha256)?;
        verify(&installer)?;
        // Launch the separate Tauri NSIS installer without first uninstalling
        // the existing product. NSIS expects /D= to be last and unquoted.
        let mut destination = std::ffi::OsString::from("/D=");
        destination.push(target);
        let status = Command::new(&installer)
            .args(["/P", "/UPDATE"])
            .raw_arg(destination)
            .status()
            .map_err(launch_error)?;
        if !status.success()
            || installed_version(&job.original).as_deref() != Some(job.version.as_str())
        {
            return Err("UPDATE_INSTALLER_FAILED");
        }
        Command::new(&job.original).spawn().map_err(launch_error)?;
        Ok(())
    })();
    let _ = fs::remove_file(installer);
    if result.is_ok() {
        let _ = fs::remove_file(failed);
    } else {
        let _ = fs::write(failed, b"install failed");
        restore(&job.original, &worker, target)?;
        Command::new(&job.original).spawn().map_err(launch_error)?;
    }
    result
}
