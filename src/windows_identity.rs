//! Shell identity shared by the main window and the hidden media-controls window.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READWRITE,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    SetCurrentProcessExplicitAppUserModelID, ShellLink,
};
use windows::core::{BSTR, HSTRING, Interface, w};

const APP_ID: &str = "Spotifast.Spotifast";

pub fn register() -> anyhow::Result<()> {
    // SAFETY: the process has not created its media or visible windows yet.
    unsafe { SetCurrentProcessExplicitAppUserModelID(w!("Spotifast.Spotifast"))? };
    // SAFETY: the returned string belongs to the caller and is freed after copying.
    let shortcut = unsafe {
        let programs = SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None)?;
        let path = PathBuf::from(OsString::from_wide(programs.as_wide()));
        CoTaskMemFree(Some(programs.0.cast()));
        path.join("Spotifast/Spotifast.lnk")
    };
    ensure_shortcut(&shortcut, &std::env::current_exe()?)
}

fn ensure_shortcut(path: &Path, executable: &Path) -> anyhow::Result<()> {
    // SAFETY: COM calls and their objects remain on this thread.
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized != RPC_E_CHANGED_MODE {
        initialized.ok()?;
    }
    let result = write_shortcut(path, executable);
    if initialized.is_ok() {
        // SAFETY: balances our initialization after all COM objects were dropped.
        unsafe { CoUninitialize() };
    }
    result
}

fn write_shortcut(path: &Path, executable: &Path) -> anyhow::Result<()> {
    // SAFETY: the caller initialized COM, and all interfaces stay on this thread.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let file: IPersistFile = link.cast()?;
        let properties: IPropertyStore = link.cast()?;
        if path.exists() {
            file.Load(&HSTRING::from(path.as_os_str()), STGM_READWRITE)?;
            let id = BSTR::try_from(&properties.GetValue(&PKEY_AppUserModel_ID)?)?;
            anyhow::ensure!(id == APP_ID, "existing shortcut belongs to another app");
            let mut target = vec![0_u16; 32768];
            link.GetPath(&mut target, std::ptr::null_mut(), 0)?;
            let end = target
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(target.len());
            if Path::new(&OsString::from_wide(&target[..end])) == executable {
                return Ok(());
            }
        }
        link.SetPath(&HSTRING::from(executable.as_os_str()))?;
        link.SetIconLocation(&HSTRING::from(executable.as_os_str()), 0)?;
        link.SetDescription(w!("Spotifast"))?;
        properties.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(APP_ID))?;
        properties.Commit()?;
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("shortcut has no parent"))?,
        )?;
        let temporary = path.with_extension("lnk.tmp");
        file.Save(&HSTRING::from(temporary.as_os_str()), true)?;
        if let Err(error) = crate::util::replace_file(&temporary, path) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_shortcut_retains_identity_and_follows_a_moved_executable() {
        let directory =
            std::env::temp_dir().join(format!("spotifast-shortcut-{}", std::process::id()));
        let path = directory.join("Spotifast.lnk");
        let executable = std::env::current_exe().unwrap();
        ensure_shortcut(&path, &executable).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        ensure_shortcut(&path, &executable).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let moved = executable.with_file_name("spotifast-moved.exe");
        ensure_shortcut(&path, &moved).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        ensure_shortcut(&path, &moved).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
