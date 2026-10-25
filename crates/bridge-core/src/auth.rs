//! Local Steam QR authorization. Refresh tokens remain in memory or an explicit SSH pipe.
use crate::{
    bad,
    gateway::{event, Authorization},
    protocol as p,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::blocking::Client;
use serde_json::json;
use std::{
    io::{self, Read},
    net::SocketAddr,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn http() -> io::Result<Client> {
    Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(15))
        .user_agent("Portal2LocalGateway/0.1")
        .build()
        .map_err(|_| bad("https_client"))
}
fn bounded(
    mut response: reqwest::blocking::Response,
    limit: usize,
) -> io::Result<Zeroizing<Vec<u8>>> {
    if !response.status().is_success() {
        return Err(bad("steam_http_status"));
    }
    if let Some(result) = response.headers().get("x-eresult") {
        let result = result.to_str().map_err(|_| bad("auth_result_header"))?;
        if result != "1" {
            event(
                "auth_api_rejected",
                json!({"eresult":result.parse::<u32>().ok()}),
            );
            return Err(bad("auth_api_rejected"));
        }
    }
    let mut body = Zeroizing::new(Vec::new());
    (&mut response)
        .take((limit + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| bad("http_body_read"))?;
    if body.len() > limit {
        return Err(bad("http_body_limit"));
    }
    Ok(body)
}
pub fn directory() -> io::Result<Vec<SocketAddr>> {
    let body=bounded(http()?.get("https://api.steampowered.com/ISteamDirectory/GetCMListForConnect/v1/?cellid=0&cmtype=netfilter&realm=steamglobal").send().map_err(|_|bad("cm_directory_https"))?,1024*1024)?;
    let value: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| bad("directory_json"))?;
    let list = value["response"]["serverlist"]
        .as_array()
        .ok_or_else(|| bad("directory_schema"))?;
    let mut out = Vec::new();
    for item in list.iter().take(256) {
        if item["type"].as_str() != Some("netfilter") {
            continue;
        }
        if let Some(endpoint) = item["endpoint"]
            .as_str()
            .and_then(|v| v.parse::<SocketAddr>().ok())
        {
            if let std::net::IpAddr::V4(ip) = endpoint.ip() {
                if endpoint.port() != 0
                    && !ip.is_private()
                    && !ip.is_loopback()
                    && !ip.is_unspecified()
                    && !ip.is_multicast()
                {
                    out.push(endpoint);
                }
            }
        }
    }
    if out.is_empty() {
        return Err(bad("directory_no_tcp_cm"));
    }
    Ok(out)
}
fn rpc(client: &Client, method: &str, body: &[u8]) -> io::Result<Zeroizing<Vec<u8>>> {
    let encoded = Zeroizing::new(STANDARD.encode(body));
    bounded(
        client
            .post(format!(
                "https://api.steampowered.com/IAuthenticationService/{method}/v1/"
            ))
            .form(&[("input_protobuf_encoded", encoded.as_str())])
            .send()
            .map_err(|_| bad("auth_https_request"))?,
        65536,
    )
}
fn show_qr(url: &[u8], path: &Path) -> io::Result<()> {
    let url = std::str::from_utf8(url).map_err(|_| bad("qr_utf8"))?;
    let parsed = reqwest::Url::parse(url).map_err(|_| bad("qr_url"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("s.team")
        || !parsed.path().starts_with("/q/")
    {
        return Err(bad("unexpected_qr_origin"));
    }
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|_| bad("qr_encode"))?;
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(384, 384)
        .build();
    std::fs::write(path, svg)?;
    event(
        "steam_approval_ready",
        json!({"qr_file":path.to_string_lossy(),"device_name":"Portal 2 local gateway","password_required_in_gateway":false}),
    );
    Ok(())
}
pub fn authorize(qr: &Path, cm: SocketAddr) -> io::Result<Authorization> {
    let result = authorize_interactive(cm, &|| false, |url| show_qr(url.as_bytes(), qr));
    let _ = std::fs::remove_file(qr);
    result
}

/// QR challenges go directly to the desktop. No token or challenge file is created.
pub fn authorize_interactive(
    cm: SocketAddr,
    cancelled: &dyn Fn() -> bool,
    mut challenge: impl FnMut(&str) -> io::Result<()>,
) -> io::Result<Authorization> {
    check_cancelled(cancelled)?;
    let client = http()?;
    let name = b"Portal 2 local gateway";
    let mut details = Vec::new();
    p::bin(1, name, &mut details);
    p::vi(2, 1, &mut details);
    p::vi(3, 20, &mut details);
    let mut begin = Vec::new();
    p::bin(1, name, &mut begin);
    p::vi(2, 1, &mut begin);
    p::bin(3, &details, &mut begin);
    p::bin(4, b"Client", &mut begin);
    let response = rpc(&client, "BeginAuthSessionViaQR", &begin)?;
    let f = p::fields(&response)?;
    let mut client_id = p::uint(&f, 1)?.ok_or_else(|| bad("auth_missing_clientid"))?;
    let request_id = Zeroizing::new(
        p::bytes(&f, 3)?
            .ok_or_else(|| bad("auth_missing_requestid"))?
            .to_vec(),
    );
    if request_id.is_empty() || request_id.len() > 1024 {
        return Err(bad("auth_requestid_size"));
    }
    let seconds = f
        .iter()
        .find(|x| x.number == 4 && x.wire == 5)
        .map(|x| f32::from_le_bytes(x.value.try_into().unwrap()))
        .unwrap_or(5.0);
    let seconds = if seconds.is_finite() {
        seconds.clamp(2.0, 30.0)
    } else {
        5.0
    };
    emit_challenge(
        p::bytes(&f, 2)?.ok_or_else(|| bad("auth_missing_qr"))?,
        &mut challenge,
        cancelled,
    )?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            wait_cancelled(Duration::from_secs_f32(seconds), cancelled)?;
            let mut poll = Zeroizing::new(Vec::new());
            p::vi(1, client_id, &mut poll);
            p::bin(2, &request_id, &mut poll);
            let response = rpc(&client, "PollAuthSessionStatus", &poll)?;
            let fields = p::fields(&response)?;
            if let Some(id) = p::uint(&fields, 1)? {
                if id != 0 {
                    client_id = id;
                }
            }
            if let Some(url) = p::bytes(&fields, 2)? {
                if !url.is_empty() {
                    emit_challenge(url, &mut challenge, cancelled)?;
                }
            }
            let token = p::bytes(&fields, 3)?.unwrap_or_default();
            let account = p::bytes(&fields, 6)?.unwrap_or_default();
            if !token.is_empty() && !account.is_empty() {
                let auth = Authorization {
                    account_name: std::str::from_utf8(account)
                        .map_err(|_| bad("account_utf8"))?
                        .into(),
                    refresh_token: std::str::from_utf8(token)
                        .map_err(|_| bad("token_utf8"))?
                        .into(),
                    cm,
                };
                auth.validate()?;
                check_cancelled(cancelled)?;
                event(
                    "steam_approval_received",
                    json!({"credentials_stored_on_disk":false}),
                );
                return Ok(auth);
            }
        }
        Err(bad("steam_approval_timeout"))
    })();
    result
}

fn check_cancelled(cancelled: &dyn Fn() -> bool) -> io::Result<()> {
    if cancelled() {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "approval_cancelled",
        ))
    } else {
        Ok(())
    }
}
fn wait_cancelled(duration: Duration, cancelled: &dyn Fn() -> bool) -> io::Result<()> {
    let until = Instant::now() + duration;
    loop {
        check_cancelled(cancelled)?;
        let Some(left) = until.checked_duration_since(Instant::now()) else {
            return Ok(());
        };
        thread::sleep(left.min(Duration::from_millis(100)));
    }
}
fn emit_challenge(
    bytes: &[u8],
    callback: &mut impl FnMut(&str) -> io::Result<()>,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<()> {
    check_cancelled(cancelled)?;
    let text = std::str::from_utf8(bytes).map_err(|_| bad("qr_utf8"))?;
    let url = reqwest::Url::parse(text).map_err(|_| bad("qr_url"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("s.team")
        || !url.path().starts_with("/q/")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err(bad("unexpected_qr_origin"));
    }
    callback(text)
}
