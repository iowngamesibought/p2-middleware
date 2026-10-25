//! Local, bounded control protocol. Steam credentials never cross this channel.
use crate::windows_host::Request;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    io, mem,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions},
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    },
    System::{
        Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId},
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

pub const LIMIT: usize = 16384;
#[derive(Serialize, Deserialize)]
#[serde(tag = "command", content = "request", deny_unknown_fields)]
pub enum Command {
    /// One elevated owner for startup cleanup, configuration and close cleanup.
    Session,
    Reset,
    Stop,
    Start(Request),
}
impl Command {
    pub fn validate(&self) -> io::Result<()> {
        match self {
            Self::Start(request) => request.validate(),
            _ => Ok(()),
        }
    }
}

pub fn runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}
pub fn pipe_name(parent: u32) -> String {
    let mut random = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut random);
    let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
    format!(r"\\.\pipe\p2-middleware-{parent}-{suffix}")
}
pub fn validate_name(name: &str, parent: u32) -> io::Result<()> {
    let prefix = format!(r"\\.\pipe\p2-middleware-{parent}-");
    let valid = parent != 0
        && name
            .strip_prefix(&prefix)
            .is_some_and(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()));
    if valid {
        Ok(())
    } else {
        Err(io::Error::other("invalid_pipe_name"))
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

struct LocalAllocation(*mut std::ffi::c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn current_sid() -> io::Result<String> {
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let token = OwnedHandle::from_raw_handle(token);
        let mut size = 0;
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            ptr::null_mut(),
            0,
            &mut size,
        );
        if size == 0 || size > 65536 {
            return Err(io::Error::other("token_size"));
        }
        // usize storage keeps TOKEN_USER aligned while Windows fills its variable-sized SID.
        let mut buffer = vec![0usize; (size as usize).div_ceil(mem::size_of::<usize>())];
        if GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut sid = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(io::Error::last_os_error());
        }
        let _allocation = LocalAllocation(sid.cast());
        let mut len = 0;
        while *sid.add(len) != 0 && len < 256 {
            len += 1;
        }
        if len == 256 {
            return Err(io::Error::other("sid_size"));
        }
        String::from_utf16(std::slice::from_raw_parts(sid, len))
            .map_err(|_| io::Error::other("sid_format"))
    }
}
pub fn server(name: &str) -> io::Result<NamedPipeServer> {
    validate_name(name, std::process::id())?;
    // Explicit DACL: the desktop user, administrators (including alternate UAC credentials), SYSTEM.
    let sddl = wide(&format!(
        "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{})",
        current_sid()?
    ));
    unsafe {
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let _allocation = LocalAllocation(descriptor);
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .max_instances(1)
            .create_with_security_attributes_raw(
                name,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )
    }
}
pub fn verify_client(pipe: &NamedPipeServer, expected: u32) -> io::Result<()> {
    let mut actual = 0;
    if unsafe { GetNamedPipeClientProcessId(pipe.as_raw_handle(), &mut actual) } == 0 {
        return Err(io::Error::last_os_error());
    }
    verify_pid(actual, expected)
}
fn verify_pid(actual: u32, expected: u32) -> io::Result<()> {
    if expected != 0 && actual == expected {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "control_peer_mismatch",
        ))
    }
}
pub fn client(name: &str, parent: u32) -> io::Result<NamedPipeClient> {
    validate_name(name, parent)?;
    // SECURITY_IDENTIFICATION is tokio's default: the server cannot impersonate the elevated helper.
    let pipe = ClientOptions::new().open(name)?;
    let mut actual = 0;
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &mut actual) } == 0 {
        return Err(io::Error::last_os_error());
    }
    verify_pid(actual, parent)?;
    Ok(pipe)
}

/// Maintains partial frames across cancelled reads/timeouts; never allocates above LIMIT+1.
#[derive(Default)]
pub struct Lines {
    pending: Vec<u8>,
}
impl Lines {
    pub async fn next(
        &mut self,
        input: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<Option<Vec<u8>>> {
        loop {
            if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                return Ok(Some(self.pending.drain(..=end).collect()));
            }
            if self.pending.len() >= LIMIT {
                return Err(io::Error::other("control_frame_limit"));
            }
            let mut chunk = [0u8; 1024];
            let capacity = chunk.len().min(LIMIT - self.pending.len());
            let count = input.read(&mut chunk[..capacity]).await?;
            if count == 0 {
                return if self.pending.is_empty() {
                    Ok(None)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "control_partial_frame",
                    ))
                };
            }
            self.pending.extend_from_slice(&chunk[..count]);
        }
    }
}
pub async fn send(
    output: &mut (impl AsyncWrite + Unpin),
    value: &impl Serialize,
) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= LIMIT {
        return Err(io::Error::other("control_frame_limit"));
    }
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(3), output.write_all(&bytes))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "control_write_timeout"))?
}
