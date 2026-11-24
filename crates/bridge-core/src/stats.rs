//! Opt-in handling of rejected user-stats writes for Steam -> PS3 traffic.
//!
//! Observed 2026-11-24: right after login the PS3 stores its locally held stats
//! (ClientStoreUserStats2, 5466). Steam answers ClientStoreUserStatsResponse (821)
//! with eresult 8 and a `stats_failed_validation` list carrying the old server values,
//! after which the console's progress appears reverted. With `accept-local`, a
//! failed response is rewritten to plain success so the PS3 keeps its own values.
//! Steam's copy is not changed by this; it stays at whatever Steam accepted.
//!
//! Field numbers: SteamDatabase/Protobufs steam/steammessages_clientserver_userstats.proto
//! (game_id 1, eresult 2, crc_stats 3, stats_failed_validation 4, stats_out_of_date 5).
use crate::{bad, protocol as p};
use serde_json::{json, Value};
use std::io;

const STORE_USER_STATS_RESPONSE: u32 = 821;
const ERESULT_OK: u64 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatsStore {
    #[default]
    Preserve,
    AcceptLocal,
}
impl StatsStore {
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "accept-local" => Ok(Self::AcceptLocal),
            _ => Err(bad("stats_store_option")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::AcceptLocal => "accept-local",
        }
    }
}

/// Returns a replacement message and audit when a failed store response is rewritten.
pub fn adapt(message: &[u8], policy: StatsStore) -> Option<(Vec<u8>, Value)> {
    if policy == StatsStore::Preserve
        || p::id(message).ok()? != STORE_USER_STATS_RESPONSE
        || message.len() < 8
        || message[3] & 128 == 0
    {
        return None;
    }
    let (_, header, body) = p::proto(message).ok()?;
    let fields = p::fields(body).ok()?;
    // Proto default for eresult is 2 (Fail).
    let eresult = p::uint(&fields, 2).ok()?.unwrap_or(2);
    let reverted: Vec<Value> = fields
        .iter()
        .filter(|f| f.number == 4)
        .filter_map(|f| {
            let s = p::fields(f.value).ok()?;
            Some(json!({"stat_id":p::uint(&s, 1).ok()??,"server_value":p::uint(&s, 2).ok()?}))
        })
        .collect();
    let out_of_date = p::uint(&fields, 5).ok()?.unwrap_or(0) != 0;
    if eresult == ERESULT_OK && reverted.is_empty() && !out_of_date {
        return None;
    }
    let mut out = Vec::new();
    for f in &fields {
        if f.number == 1 || f.number == 3 {
            out.extend(f.raw);
        }
    }
    p::vi(2, ERESULT_OK, &mut out);
    Some((
        p::message(STORE_USER_STATS_RESPONSE, header, &out),
        json!({"emsg":STORE_USER_STATS_RESPONSE,"action":"store_failure_reported_as_success",
               "steam_eresult":eresult,"steam_reverted":reverted,"stats_out_of_date":out_of_date}),
    ))
}
