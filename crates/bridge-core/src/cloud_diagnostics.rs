//! Read-only diagnostics for Steam Cloud (UFS), user stats and service-method traffic.
//!
//! Never changes a message. Produces a JSON summary that names the message, keeps
//! job ids, eresult and target job name from the protobuf header, and dumps the body
//! as a field tree. Values that could identify or authenticate the account are
//! withheld: SteamID-shaped fixed64 values are redacted, opaque byte strings are
//! reduced to length plus a short SHA-1 prefix, and the UFS login request (which
//! carries a session token) is reduced to its size.
//!
//! EMsg ids: SteamKit Resources/SteamLanguage/emsg.steamd. Stats field layout:
//! SteamDatabase/Protobufs steam/steammessages_clientserver_userstats.proto.
use crate::protocol as p;
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};

const UFS_LOGIN_REQUEST: u32 = 5213;
const MAX_FIELDS: usize = 256;
const MAX_STRING: usize = 260;
const MAX_LEGACY_HEX: usize = 1024;

fn name(id: u32) -> Option<&'static str> {
    Some(match id {
        146 => "ServiceMethod",
        147 => "ServiceMethodResponse",
        151 => "ServiceMethodCallFromClient",
        152 => "ServiceMethodSendToClient",
        818 => "ClientGetUserStats",
        819 => "ClientGetUserStatsResponse",
        820 => "ClientStoreUserStats",
        821 => "ClientStoreUserStatsResponse",
        5202 => "ClientUFSUploadFileRequest",
        5203 => "ClientUFSUploadFileResponse",
        5204 => "ClientUFSUploadFileChunk",
        5205 => "ClientUFSUploadFileFinished",
        5206 => "ClientUFSGetFileListForApp",
        5207 => "ClientUFSGetFileListForAppResponse",
        5210 => "ClientUFSDownloadRequest",
        5211 => "ClientUFSDownloadResponse",
        5212 => "ClientUFSDownloadChunk",
        5213 => "ClientUFSLoginRequest",
        5214 => "ClientUFSLoginResponse",
        5216 => "ClientUFSTransferHeartbeat",
        5219 => "ClientUFSDeleteFileRequest",
        5220 => "ClientUFSDeleteFileResponse",
        5226 => "ClientUFSGetUGCDetails",
        5227 => "ClientUFSGetUGCDetailsResponse",
        5230 => "ClientUFSGetSingleFileInfo",
        5231 => "ClientUFSGetSingleFileInfoResponse",
        5232 => "ClientUFSShareFile",
        5233 => "ClientUFSShareFileResponse",
        5251 => "ClientUFSUploadCommit",
        5252 => "ClientUFSUploadCommitResponse",
        5466 => "ClientStoreUserStats2",
        5467 => "ClientStatsUpdated",
        5480 => "ClientRequestedClientStats",
        _ => return None,
    })
}

fn interesting(id: u32) -> bool {
    (5200..=5299).contains(&id) || name(id).is_some()
}

fn service(id: u32) -> bool {
    matches!(id, 146 | 147 | 151 | 152)
}

fn short_hash(b: &[u8]) -> String {
    Sha1::digest(b)[..6]
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn printable(b: &[u8]) -> Option<&str> {
    let s = std::str::from_utf8(b).ok()?;
    let s = s.strip_suffix('\0').unwrap_or(s);
    (!s.is_empty() && s.len() <= MAX_STRING && s.chars().all(|c| !c.is_control())).then_some(s)
}

fn steamid_shaped(v: u64) -> bool {
    v >> 56 == 1 && v & 0xffff_ffff != 0
}

/// Dump a protobuf body as `[{"f":number,"v":value}]`, recursing into plausible
/// sub-messages up to `depth`.
fn tree(body: &[u8], depth: u8) -> Option<Value> {
    let fields = p::fields(body).ok()?;
    let mut out = Vec::new();
    for f in fields.iter().take(MAX_FIELDS) {
        let v = match f.wire {
            0 => json!(p::varint(f.value, &mut 0).ok()?),
            1 => {
                let n = u64::from_le_bytes(f.value.try_into().ok()?);
                if steamid_shaped(n) {
                    json!("steamid_redacted")
                } else {
                    json!(n)
                }
            }
            5 => json!(u32::from_le_bytes(f.value.try_into().ok()?)),
            _ => {
                if let Some(s) = printable(f.value) {
                    json!(s)
                } else if let Some(nested) = (depth > 0 && !f.value.is_empty())
                    .then(|| tree(f.value, depth - 1))
                    .flatten()
                {
                    nested
                } else {
                    json!({"len":f.value.len(),"sha1":short_hash(f.value)})
                }
            }
        };
        out.push(json!({"f":f.number,"v":v}));
    }
    if fields.len() > MAX_FIELDS {
        out.push(json!({"truncated_fields":fields.len() - MAX_FIELDS}));
    }
    Some(Value::Array(out))
}

const GET_USER_STATS_RESPONSE: u32 = 819;
const OTHER_LANGUAGES: &[&str] = &[
    "arabic",
    "brazilian",
    "bulgarian",
    "czech",
    "danish",
    "dutch",
    "finnish",
    "french",
    "german",
    "greek",
    "hungarian",
    "indonesian",
    "italian",
    "japanese",
    "koreana",
    "latam",
    "norwegian",
    "polish",
    "portuguese",
    "romanian",
    "russian",
    "schinese",
    "spanish",
    "swedish",
    "tchinese",
    "thai",
    "turkish",
    "ukrainian",
    "vietnamese",
];

/// Binary KeyValues to JSON. Non-English localized strings are dropped.
fn keyvalues(b: &[u8]) -> Option<Value> {
    fn cstr(b: &[u8], p: &mut usize) -> Option<String> {
        let n = b.get(*p..)?.iter().position(|&c| c == 0)?;
        let s = String::from_utf8_lossy(&b[*p..*p + n]).into_owned();
        *p += n + 1;
        Some(s)
    }
    fn take<const N: usize>(b: &[u8], p: &mut usize) -> Option<[u8; N]> {
        let v = b.get(*p..*p + N)?.try_into().ok()?;
        *p += N;
        Some(v)
    }
    fn object(b: &[u8], p: &mut usize, depth: u8) -> Option<Map<String, Value>> {
        let mut m = Map::new();
        loop {
            let kind = *b.get(*p)?;
            *p += 1;
            if kind == 8 || kind == 11 {
                return Some(m);
            }
            let key = cstr(b, p)?;
            let v = match kind {
                0 if depth < 16 => Value::Object(object(b, p, depth + 1)?),
                1 => json!(cstr(b, p)?),
                2 | 4 | 6 => json!(i32::from_le_bytes(take(b, p)?)),
                3 => json!(f32::from_le_bytes(take(b, p)?)),
                7 => json!(u64::from_le_bytes(take(b, p)?)),
                10 => json!(i64::from_le_bytes(take(b, p)?)),
                _ => return None,
            };
            if !OTHER_LANGUAGES.contains(&key.as_str()) {
                m.insert(key, v);
            }
        }
    }
    let mut p = 0;
    let mut root = Map::new();
    while p < b.len() && b[p] != 8 && b[p] != 11 {
        if b[p] != 0 {
            return None;
        }
        p += 1;
        let key = cstr(b, &mut p)?;
        root.insert(key, Value::Object(object(b, &mut p, 0)?));
    }
    Some(Value::Object(root))
}

fn header_summary(header: &[u8]) -> Map<String, Value> {
    let mut m = Map::new();
    let Ok(h) = p::fields(header) else {
        m.insert("header".into(), json!("unparsed"));
        return m;
    };
    if let Ok(Some(v)) = p::uint(&h, 10) {
        m.insert("job_source".into(), json!(v));
    }
    if let Ok(Some(v)) = p::uint(&h, 11) {
        m.insert("job_target".into(), json!(v));
    }
    if let Ok(Some(v)) = p::bytes(&h, 12) {
        m.insert("target_job_name".into(), json!(String::from_utf8_lossy(v)));
    }
    if let Ok(Some(v)) = p::uint(&h, 13) {
        m.insert("header_eresult".into(), json!(v as i32));
    }
    if let Ok(Some(v)) = p::bytes(&h, 14) {
        m.insert("error_message".into(), json!(String::from_utf8_lossy(v)));
    }
    m
}

/// `direction` is "client" (PS3 -> Steam) or "server" (Steam -> PS3).
/// Call with the message as received, before any rewrite.
pub fn summarize(message: &[u8], direction: &str) -> Option<Value> {
    let id = p::id(message).ok()?;
    if !interesting(id) {
        return None;
    }
    let mut m = Map::new();
    m.insert("direction".into(), json!(direction));
    m.insert("emsg".into(), json!(id));
    m.insert("name".into(), json!(name(id).unwrap_or("UFS_unnamed")));
    m.insert("bytes".into(), json!(message.len()));
    if message.len() >= 8 && message[3] & 128 != 0 {
        m.insert("format".into(), json!("protobuf"));
        match p::proto(message) {
            Ok((_, header, body)) => {
                m.extend(header_summary(header));
                m.insert("body_bytes".into(), json!(body.len()));
                if id == UFS_LOGIN_REQUEST {
                    m.insert("body".into(), json!("withheld_contains_session_token"));
                } else if service(id) {
                    m.insert("body_sha1".into(), json!(short_hash(body)));
                } else {
                    m.insert(
                        "body".into(),
                        tree(body, 3).unwrap_or_else(|| json!("unparsed")),
                    );
                }
                if id == GET_USER_STATS_RESPONSE {
                    // Field 4 is the stats schema (binary KeyValues): names and
                    // validation rules such as min, max, maxchange and permission.
                    let schema = p::fields(body)
                        .ok()
                        .and_then(|f| p::bytes(&f, 4).ok().flatten())
                        .map(|b| keyvalues(b).unwrap_or_else(|| json!("unparsed")));
                    if let Some(schema) = schema {
                        m.insert("schema".into(), schema);
                    }
                }
            }
            Err(_) => {
                m.insert("body".into(), json!("malformed"));
            }
        }
    } else {
        // Legacy struct form. Skip the header (it carries SteamID and session id).
        let header = if message.len() >= 36 && message[4] == 36 && message[5..7] == [2, 0] {
            36
        } else {
            20
        };
        m.insert("format".into(), json!("legacy"));
        m.insert("header_bytes".into(), json!(header));
        let body = message.get(header..).unwrap_or_default();
        m.insert("body_bytes".into(), json!(body.len()));
        if id == UFS_LOGIN_REQUEST {
            m.insert("body".into(), json!("withheld_contains_session_token"));
        } else if !service(id) {
            let mut shown = body[..body.len().min(MAX_LEGACY_HEX)].to_vec();
            // Individual public SteamIDs: high dword 0x01100001. Overwrite with 0xee.
            let mut i = 0;
            while i + 8 <= shown.len() {
                if shown[i + 4..i + 8] == [0x01, 0x00, 0x10, 0x01] {
                    shown[i..i + 8].fill(0xee);
                    i += 8;
                } else {
                    i += 1;
                }
            }
            m.insert(
                "body_hex".into(),
                json!(shown.iter().map(|x| format!("{x:02x}")).collect::<String>()),
            );
            m.insert("body_sha1".into(), json!(short_hash(body)));
        }
    }
    Some(Value::Object(m))
}
