//! Opt-in Steam Cloud download blocking for Steam -> PS3 traffic.
//!
//! When enabled, the PS3 is told that Steam Cloud holds no files for it and any
//! download it still attempts is answered with "file not found". The console then
//! keeps its local save data. Uploads from the PS3 are not touched.
//!
//! EMsg ids: SteamKit Resources/SteamLanguage/emsg.steamd (BaseUFSRange 5200).
//! Field numbers: SteamDatabase/Protobufs c8545f47fb47a8f94916634973bed21d58cd72c2
//! steam/steammessages_clientserver_ufs.proto.
use crate::{bad, protocol as p};
use serde_json::{json, Value};
use std::io;

const GET_FILE_LIST_RESPONSE: u32 = 5207;
const DOWNLOAD_RESPONSE: u32 = 5211;
const DOWNLOAD_CHUNK: u32 = 5212;
const GET_SINGLE_FILE_INFO_RESPONSE: u32 = 5231;
const ERESULT_FILE_NOT_FOUND: u64 = 9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloudDownloads {
    #[default]
    Preserve,
    Block,
}
impl CloudDownloads {
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "block" => Ok(Self::Block),
            _ => Err(bad("cloud_downloads_option")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Block => "block",
        }
    }
}

/// What to do with one Steam -> PS3 message.
pub enum Action {
    /// Send the message unchanged.
    Forward,
    /// Send this rewritten message instead.
    Replace(Vec<u8>, Value),
    /// Do not send anything to the PS3.
    Drop(Value),
}

/// Call only for Steam -> PS3 traffic, after the logon response handling.
pub fn adapt(message: &[u8], policy: CloudDownloads) -> io::Result<Action> {
    if policy == CloudDownloads::Preserve {
        return Ok(Action::Forward);
    }
    let id = p::id(message)?;
    if ![
        GET_FILE_LIST_RESPONSE,
        DOWNLOAD_RESPONSE,
        DOWNLOAD_CHUNK,
        GET_SINGLE_FILE_INFO_RESPONSE,
    ]
    .contains(&id)
    {
        return Ok(Action::Forward);
    }
    if id == DOWNLOAD_CHUNK {
        // No download is ever granted, so stray file data is never delivered.
        return Ok(Action::Drop(json!({"emsg":id,"action":"chunk_dropped"})));
    }
    if message.len() < 8 || message[3] & 128 == 0 {
        // Non-protobuf form is not understood; withhold it rather than leak cloud data.
        return Ok(Action::Drop(
            json!({"emsg":id,"action":"dropped_unrecognized_format"}),
        ));
    }
    let (_, header, body) = p::proto(message)?;
    let fields = p::fields(body)?;
    let mut out = Vec::new();
    let audit = match id {
        GET_FILE_LIST_RESPONSE => {
            // files = 1 removed; path_prefixes = 2 and unknown fields kept.
            let hidden = fields.iter().filter(|f| f.number == 1).count();
            for f in &fields {
                if f.number != 1 {
                    out.extend(f.raw);
                }
            }
            json!({"emsg":id,"action":"file_list_emptied","files_hidden":hidden})
        }
        DOWNLOAD_RESPONSE => {
            // Keep only app_id = 2 and answer eresult = 1 with FileNotFound.
            p::vi(1, ERESULT_FILE_NOT_FOUND, &mut out);
            for f in &fields {
                if f.number == 2 {
                    out.extend(f.raw);
                }
            }
            json!({"emsg":id,"action":"download_refused"})
        }
        _ => {
            // GetSingleFileInfoResponse: keep app_id = 2 and file_name = 3.
            p::vi(1, ERESULT_FILE_NOT_FOUND, &mut out);
            for f in &fields {
                if f.number == 2 || f.number == 3 {
                    out.extend(f.raw);
                }
            }
            json!({"emsg":id,"action":"file_info_refused"})
        }
    };
    Ok(Action::Replace(p::message(id, header, &out), audit))
}
