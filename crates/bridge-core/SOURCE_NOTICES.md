# bridge-core source notices

| Source | Pinned identity | Used for | Treatment |
| --- | --- | --- | --- |
| SteamRE/SteamKit | `2049844fdf746420c9f31a9575d4b75c40a5d575` | RSA public key, OAEP/challenge/CRC exchange, HMAC-IV construction, modern logon and headers, binary KeyValue format | Rust adaptation in `channel.rs`, `protocol.rs` and `lobby_advertisement.rs`; LGPL-2.1-or-later, see `notices/` |
| SteamDatabase/Protobufs | `6c35e1ad6b5401e4f95cd0923e8145a23613516c` | Login, base message and lobby message field numbers | Field numbers implemented independently; no schema files included |
| InvoxiPlayGames/condenstation | `6592b25b2ce56bc48e4c18388ada5b9b412fb95a` | QR sign-in route and platform fields | Protocol behaviour reference only; no code included, linked or compiled |

Adapted SteamKit portions: Copyright (C) 2018 Ryan Stecker & SteamRE Team, under
the GNU Lesser General Public License, version 2.1 or later. The rest of this crate
is under the MIT License of the repository.

Sign-in uses the official `https://api.steampowered.com/IAuthenticationService/`
API with TLS verification. Steam servers are found through the official Steam
directory.
