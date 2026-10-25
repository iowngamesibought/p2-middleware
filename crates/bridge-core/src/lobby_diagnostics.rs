//! Fail-open lobby-list diagnostics and explicit opt-in PS3 query widening.
//!
//! Field numbers: SteamDatabase/Protobufs at
//! 6c35e1ad6b5401e4f95cd0923e8145a23613516c,
//! steam/steammessages_clientserver_mms.proto, CMsgClientMMSGetLobbyList{,Response}.
//! EMsg IDs: retained SteamKit 2049844fdf746420c9f31a9575d4b75c40a5d575.
//! This is schema-based synthetic coverage; no historical PS3 bodies were retained.
use crate::{bad, protocol as p};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::io;

/// Experimental query widening only; it does not relax any game/version check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchPlatform {
    #[default]
    Preserve,
    Any,
}

impl SearchPlatform {
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "any" => Ok(Self::Any),
            _ => Err(bad("search_platform_option")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Any => "any",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchDistance {
    #[default]
    Preserve,
    Worldwide,
}
impl SearchDistance {
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "worldwide" => Ok(Self::Worldwide),
            _ => Err(bad("search_distance_option")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Worldwide => "worldwide",
        }
    }
}

const MAX_BODY: usize = 64 * 1024;
const MAX_FIELDS: usize = 1024;
const MAX_FILTERS: usize = 32;
const MAX_FILTER_BYTES: usize = 1024;

fn fields(bytes: &[u8]) -> io::Result<Vec<p::Field<'_>>> {
    if bytes.len() > MAX_BODY {
        return Err(bad("diagnostic_limit"));
    }
    let fields = p::fields(bytes)?;
    if fields.len() > MAX_FIELDS {
        return Err(bad("diagnostic_limit"));
    }
    Ok(fields)
}

// Reject ambiguous singular fields and incorrect wire types only in the diagnostic.
fn singular<'a>(fields: &'a [p::Field<'a>], n: u32, wire: u8) -> io::Result<Option<&'a [u8]>> {
    let mut matches = fields.iter().filter(|f| f.number == n);
    let Some(field) = matches.next() else {
        return Ok(None);
    };
    if field.wire != wire || matches.next().is_some() {
        return Err(bad("diagnostic_field"));
    }
    Ok(Some(field.value))
}

fn uint(fields: &[p::Field<'_>], n: u32) -> io::Result<Option<u32>> {
    singular(fields, n, 0)?
        .map(|v| u32::try_from(p::varint(v, &mut 0)?).map_err(|_| bad("diagnostic_uint32")))
        .transpose()
}

fn int(fields: &[p::Field<'_>], n: u32) -> io::Result<Option<i32>> {
    singular(fields, n, 0)?
        .map(|v| p::varint(v, &mut 0).map(|v| v as i32))
        .transpose()
}

fn bounded_decimal(value: &[u8], max: u32) -> Option<u32> {
    if value.is_empty() || value.len() > 2 || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let number = value
        .iter()
        .fold(0u32, |n, digit| n * 10 + u32::from(digit - b'0'));
    (number <= max).then_some(number)
}

fn filter(bytes: &[u8]) -> io::Result<Value> {
    if bytes.len() > MAX_FILTER_BYTES {
        return Err(bad("diagnostic_limit"));
    }
    let f = fields(bytes)?;
    let key = singular(&f, 1, 2)?.unwrap_or_default();
    let value = singular(&f, 2, 2)?.unwrap_or_default();
    let filter_type = uint(&f, 4)?;
    // Emit only fixed categories and allowlisted enum words. Unknown keys/values
    // can contain player identities or arbitrary metadata, so never print them.
    let (category, words): (&str, &[&str]) = match key {
        b"game:platform" | b"/game:platform" | b"game/platform" => (
            "platform",
            &[
                "ps3", "pc", "x360", "xbox360", "windows", "linux", "mac", "osx", "all", "any",
                "default",
            ],
        ),
        b"game:mode" | b"/game:mode" | b"game/mode" => (
            "mode",
            &["coop", "sp", "mp", "coop_challenge", "coop_community"],
        ),
        b"game:state" | b"/game:state" | b"game/state" => (
            "state",
            &["lobby", "game", "ingame", "in-game", "waiting", "playing"],
        ),
        b"game:mp_coop_start" | b"/game:mp_coop_start" | b"game/mp_coop_start"
            if filter_type == Some(3) =>
        {
            ("coop_start_preference", &[])
        }
        b"game:sixense_features" | b"/game:sixense_features" | b"game/sixense_features"
            if filter_type == Some(3) =>
        {
            ("sixense_preference", &[])
        }
        _ => ("redacted", &[]),
    };
    let safe_value = words
        .iter()
        .find(|word| value.eq_ignore_ascii_case(word.as_bytes()));
    let comparison = int(&f, 3)?;
    let type_name = match filter_type {
        Some(0) => "string",
        Some(1) => "numerical",
        Some(2) => "slots_available",
        Some(3) => "near_value",
        Some(4) => "distance",
        _ => "unknown",
    };
    let slots = (filter_type == Some(2) && key.is_empty())
        .then(|| bounded_decimal(value, 16))
        .flatten();
    let distance = (filter_type == Some(4) && key.is_empty())
        .then(|| bounded_decimal(value, 3))
        .flatten();
    let distance_name = distance.map(|n| ["close", "default", "far", "worldwide"][n as usize]);
    Ok(json!({
        "category":category,
        "value":safe_value,
        "value_redacted":safe_value.is_none(),
        "comparison":comparison.filter(|v| (-2..=3).contains(v)),
        "comparison_unrecognized":comparison.is_some_and(|v| !(-2..=3).contains(&v)),
        "filter_type":filter_type.filter(|v| (0..=8).contains(v)),
        "filter_type_unrecognized":filter_type.is_some_and(|v| !(0..=8).contains(&v)),
        "filter_type_name":type_name,"empty_or_missing_key":key.is_empty(),
        "slots_available":slots,"distance_enum":distance,"distance_name":distance_name
    }))
}

fn decode(message: &[u8]) -> io::Result<Value> {
    let (id, _, body) = p::proto(message)?;
    let f = fields(body)?;
    let app_id = uint(&f, 1)?;
    if id == 6604 {
        let mut count = 0;
        for field in f.iter().filter(|field| field.number == 9) {
            if field.wire != 2 {
                return Err(bad("diagnostic_member_wire"));
            }
            fields(field.value)?;
            count += 1;
        }
        Ok(json!({
            "emsg":id,"status":"decoded","app_id":app_id,
            "chat_room_enter_response":int(&f, 3)?,
            "max_members":int(&f, 4)?,"member_count":count
        }))
    } else if id == 6607 {
        let count = f.iter().filter(|f| f.number == 6).count();
        let mut filters = Vec::new();
        let mut distance_present = false;
        for field in f.iter().filter(|f| f.number == 6) {
            if field.wire != 2 {
                return Err(bad("diagnostic_filter_wire"));
            }
            let summary = filter(field.value)?;
            distance_present |= summary["filter_type"] == 4;
            if filters.len() < MAX_FILTERS {
                filters.push(summary);
            }
        }
        Ok(json!({
            "emsg":id,"status":"decoded","app_id":app_id,
            "num_lobbies_requested":int(&f, 3)?,
            "filter_count":count,"filters":filters,
            "filters_omitted":count.saturating_sub(MAX_FILTERS),
            "explicit_distance_filter_present":distance_present
        }))
    } else {
        let mut count = 0;
        for field in f.iter().filter(|f| f.number == 4) {
            if field.wire != 2 {
                return Err(bad("diagnostic_lobby_wire"));
            }
            // Validate protobuf shape without reading identities or metadata.
            fields(field.value)?;
            count += 1;
        }
        let result = int(&f, 3)?;
        Ok(json!({
            "emsg":id,"status":"decoded","app_id":app_id,
            "eresult":result.unwrap_or(2),"eresult_defaulted":result.is_none(),
            "lobby_count":count
        }))
    }
}

/// An invalid/oversized body produces a constant unavailable marker. No error is
/// propagated to the relay, and no byte of the original message is changed.
pub fn summarize(message: &[u8]) -> Option<Value> {
    let id = p::id(message).ok()?;
    if ![6604, 6607, 6608].contains(&id) {
        return None;
    }
    Some(decode(message).unwrap_or_else(
        |_| json!({"emsg":id,"status":"unavailable","reason":"malformed_or_limit"}),
    ))
}

// Empty-key string value3, comparison0, filter_type4. This independent literal
// follows the pinned SteamKit DistanceFilter serializer and enum contract.
const WORLDWIDE_FILTER: &[u8] = b"\x32\x09\x0a\x00\x12\x013\x18\x00\x20\x04";

struct Rewrite {
    changed: Option<Vec<u8>>,
    removed: usize,
    reason: &'static str,
    distance_action: &'static str,
}
impl Rewrite {
    fn unchanged(reason: &'static str, distance_action: &'static str) -> Self {
        Self {
            changed: None,
            removed: 0,
            reason,
            distance_action,
        }
    }
}

// Validate every candidate before constructing any changed bytes, so combined
// platform/distance adaptation is atomic when either candidate is unsupported.
fn widen(
    message: &[u8],
    platform: SearchPlatform,
    distance: SearchDistance,
) -> io::Result<Rewrite> {
    let (id, header, body) = p::proto(message)?;
    if id != 6607 {
        return Ok(Rewrite::unchanged("other_message", "not_applicable"));
    }
    let f = fields(body)?;
    if uint(&f, 1)? != Some(620) {
        return Ok(Rewrite::unchanged("other_or_missing_app", "not_applicable"));
    }
    for n in [3, 4, 5] {
        singular(&f, n, 0)?;
    }
    for n in [7, 8] {
        singular(&f, n, 2)?;
    }
    let mut remove = Vec::new();
    let mut filter_count = 0;
    let mut distance_index = None;
    let mut distance_value = None;
    for (index, field) in f.iter().enumerate().filter(|(_, field)| field.number == 6) {
        filter_count += 1;
        if filter_count > MAX_FILTERS || field.wire != 2 || field.value.len() > MAX_FILTER_BYTES {
            return Err(bad("widen_filter_limit_or_wire"));
        }
        let filter = fields(field.value)?;
        let key = singular(&filter, 1, 2)?;
        let value = singular(&filter, 2, 2)?;
        // Do not truncate large varints into a recognized comparison/type.
        let comparison = uint(&filter, 3)?;
        let filter_type = uint(&filter, 4)?;
        if platform == SearchPlatform::Any
            && matches!(
                key,
                Some(b"game:platform" | b"/game:platform" | b"game/platform")
            )
        {
            if filter.iter().any(|field| !(1..=4).contains(&field.number))
                || !value.is_some_and(|value| value.eq_ignore_ascii_case(b"ps3"))
                || comparison != Some(0)
                || filter_type != Some(0)
            {
                return Err(bad("widen_unsupported_platform_filter"));
            }
            remove.push(index);
        }
        if distance == SearchDistance::Worldwide && filter_type == Some(4) {
            if distance_index.is_some()
                || filter.iter().any(|field| !(1..=4).contains(&field.number))
                || key.is_some_and(|key| !key.is_empty())
                || comparison != Some(0)
            {
                return Err(bad("widen_unsupported_distance_filter"));
            }
            distance_value = Some(
                value
                    .and_then(|value| bounded_decimal(value, 3))
                    .ok_or_else(|| bad("widen_unsupported_distance_value"))?,
            );
            distance_index = Some(index);
        }
    }
    let distance_action = match distance {
        SearchDistance::Preserve => "preserved",
        SearchDistance::Worldwide => match distance_value {
            Some(3) => "already_worldwide",
            Some(_) => "replaced_worldwide",
            None => "added_worldwide",
        },
    };
    let distance_changed = matches!(distance_action, "added_worldwide" | "replaced_worldwide");
    if remove.is_empty() && !distance_changed {
        return Ok(Rewrite::unchanged(
            "no_recognized_ps3_filter",
            distance_action,
        ));
    }
    if distance_action == "added_worldwide" && filter_count - remove.len() >= MAX_FILTERS {
        return Err(bad("widen_result_filter_limit"));
    }
    if f.len() - remove.len() + usize::from(distance_action == "added_worldwide") > MAX_FIELDS {
        return Err(bad("widen_result_field_limit"));
    }
    // Preserve even noncanonical-but-valid encodings of the message/header and
    // retained fields, rather than rebuilding through generated protobuf code.
    let mut changed = message[..8 + header.len()].to_vec();
    for (index, field) in f.iter().enumerate() {
        if !remove.contains(&index) {
            if distance_action == "replaced_worldwide" && distance_index == Some(index) {
                changed.extend_from_slice(WORLDWIDE_FILTER);
            } else {
                changed.extend_from_slice(field.raw);
            }
        }
    }
    if distance_action == "added_worldwide" {
        changed.extend_from_slice(WORLDWIDE_FILTER);
    }
    if changed.len() - (8 + header.len()) > MAX_BODY {
        return Err(bad("widen_result_body_limit"));
    }
    Ok(Rewrite {
        changed: Some(changed),
        removed: remove.len(),
        reason: "applied",
        distance_action,
    })
}

/// Never propagates a query-adaptation error to relay. Original and effective
/// diagnostics contain only the same allowlisted, bounded fields as summarize.
pub fn adapt(message: &[u8], policy: SearchPlatform) -> (Cow<'_, [u8]>, Option<Value>) {
    adapt_with_distance(message, policy, SearchDistance::Preserve)
}

pub fn adapt_with_distance(
    message: &[u8],
    policy: SearchPlatform,
    distance: SearchDistance,
) -> (Cow<'_, [u8]>, Option<Value>) {
    if p::id(message).ok() != Some(6607) {
        return (Cow::Borrowed(message), None);
    }
    let result = if policy == SearchPlatform::Any || distance == SearchDistance::Worldwide {
        widen(message, policy, distance).unwrap_or(Rewrite::unchanged(
            "unsupported_malformed_or_limit",
            "not_applied",
        ))
    } else {
        Rewrite::unchanged("preserved", "preserved")
    };
    let effective = result.changed.map_or(Cow::Borrowed(message), Cow::Owned);
    let audit = json!({
        "policy":policy.name(),"search_distance":distance.name(),
        "removed_filters":result.removed,"reason":result.reason,"distance_action":result.distance_action,
        "original":summarize(message),"effective":summarize(&effective)
    });
    (effective, Some(audit))
}
