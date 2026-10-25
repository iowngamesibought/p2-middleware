# Source and third-party notices

## This project

Code in this repository is released under the MIT License (see `LICENSE`), except
where a file or section below says otherwise.

## SteamKit-derived portions (LGPL-2.1-or-later)

The Steam connection encryption in `crates/bridge-core` (RSA public key, OAEP
challenge/CRC exchange, HMAC-IV packet construction, modern logon headers and the
binary KeyValue format used for lobby metadata) is a Rust adaptation of SteamKit at
commit `2049844fdf746420c9f31a9575d4b75c40a5d575`.

Copyright (C) 2018 Ryan Stecker & SteamRE Team. These portions remain under the
GNU Lesser General Public License, version 2.1 or later. The license text and the
upstream notice are in `crates/bridge-core/notices/`. The complete source for the
distributed binaries is this repository, so the LGPL portions can be modified and
rebuilt. `crates/bridge-core/src/steam-public.der` is Steam's public key as
published in SteamKit; it is not a private key.

https://github.com/SteamRE/SteamKit

## Protocol references (not included)

Steam message field numbers were implemented from the public schema in
SteamDatabase/Protobufs at commit `6c35e1ad6b5401e4f95cd0923e8145a23613516c`.
No schema files from that repository are included.

The Steam QR sign-in route and platform fields follow the public protocol behaviour
documented by InvoxiPlayGames/condenstation (`6592b25b2ce56bc48e4c18388ada5b9b412fb95a`).
None of its code is included, linked or compiled.

## Dependencies

Rust dependencies keep their own licenses. Exact versions are pinned in
`Cargo.lock`; `cargo metadata` lists each license. Main direct dependencies:
egui/eframe, qrcode, serde, serde_json, toml, tokio, rand, base64, zeroize and
windows-sys (MIT OR Apache-2.0), and reqwest with native-tls for HTTPS.

## What is not included

No game files, game code, decompiled code, PS3 firmware, Steam or PSN account
data, tickets or packet captures are part of this project.
