//! Explicit user-initiated UAC launch. No credentials or network configuration in arguments.
use std::{
    io, mem,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE},
        Threading::{GetExitCodeProcess, GetProcessId, WaitForSingleObject},
    },
    UI::{
        Shell::{
            ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
            SHELLEXECUTEINFOW,
        },
        WindowsAndMessaging::SW_HIDE,
    },
};

pub struct Process(OwnedHandle);
impl Process {
    pub fn id(&self) -> u32 {
        unsafe { GetProcessId(self.0.as_raw_handle()) }
    }
    pub fn finished(&self) -> io::Result<Option<bool>> {
        match unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                if unsafe { GetExitCodeProcess(self.0.as_raw_handle(), &mut code) } == 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(Some(code == 0))
                }
            }
            _ => Err(io::Error::last_os_error()),
        }
    }
}
pub fn launch(path: &Path, name: &str, parent: u32) -> io::Result<Process> {
    crate::control::validate_name(name, parent)?;
    if !path.is_absolute() || path.file_name().and_then(|n| n.to_str()) != Some("p2-route.exe") {
        return Err(io::Error::other("helper_path"));
    }
    let file: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain([0]).collect();
    // The validated pipe grammar has no quotes, spaces or shell metacharacters.
    let args: Vec<u16> = format!("--pipe {name} --parent {parent}")
        .encode_utf16()
        .chain([0])
        .collect();
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        let result = CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        );
        if result < 0 {
            return Err(io::Error::other("helper_com_init"));
        }
        let _com = Com;
        let mut info = SHELLEXECUTEINFOW {
            cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: args.as_ptr(),
            nShow: SW_HIDE,
            ..mem::zeroed()
        };
        if ShellExecuteExW(&mut info) == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.hProcess.is_null() {
            return Err(io::Error::other("helper_process_missing"));
        }
        Ok(Process(OwnedHandle::from_raw_handle(info.hProcess)))
    }
}
