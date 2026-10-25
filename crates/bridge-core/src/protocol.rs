//! Bounded wire codec. Unknown fields retain their original bytes.
use crate::bad;
use flate2::read::GzDecoder;
use std::io::{self, Read};
pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;
pub const MAX_MULTI: usize = 16 * 1024 * 1024;
pub const MODERN_PROTOCOL: u64 = 65581;

#[derive(Clone, Debug)]
pub struct Field<'a> {
    pub number: u32,
    pub wire: u8,
    pub value: &'a [u8],
    pub raw: &'a [u8],
}
pub fn varint(b: &[u8], p: &mut usize) -> io::Result<u64> {
    let mut n = 0;
    for i in 0..10 {
        let x = *b.get(*p).ok_or_else(|| bad("truncated_varint"))?;
        *p += 1;
        if i == 9 && x > 1 {
            return Err(bad("varint_overflow"));
        }
        n |= ((x & 127) as u64) << (7 * i);
        if x & 128 == 0 {
            return Ok(n);
        }
    }
    Err(bad("long_varint"))
}
pub fn put_varint(mut n: u64, b: &mut Vec<u8>) {
    while n >= 128 {
        b.push((n as u8) | 128);
        n >>= 7;
    }
    b.push(n as u8);
}
pub fn fields(b: &[u8]) -> io::Result<Vec<Field<'_>>> {
    if b.len() > MAX_MULTI {
        return Err(bad("protobuf_size"));
    }
    let mut p = 0;
    let mut out = Vec::new();
    while p < b.len() {
        if out.len() >= 65536 {
            return Err(bad("field_budget"));
        }
        let start = p;
        let tag = varint(b, &mut p)?;
        if tag >> 3 == 0 || tag >> 3 > 0x1fff_ffff {
            return Err(bad("field_number"));
        }
        let wire = (tag & 7) as u8;
        let mut v = p;
        match wire {
            0 => {
                varint(b, &mut p)?;
            }
            1 => p = p.checked_add(8).ok_or_else(|| bad("length_overflow"))?,
            2 => {
                let n = usize::try_from(varint(b, &mut p)?).map_err(|_| bad("length_overflow"))?;
                v = p;
                p = p.checked_add(n).ok_or_else(|| bad("length_overflow"))?;
            }
            5 => p = p.checked_add(4).ok_or_else(|| bad("length_overflow"))?,
            _ => return Err(bad("unsupported_wire_type")),
        }
        if p > b.len() {
            return Err(bad("truncated_field"));
        }
        out.push(Field {
            number: (tag >> 3) as u32,
            wire,
            value: &b[v..p],
            raw: &b[start..p],
        });
    }
    Ok(out)
}
pub fn uint(f: &[Field<'_>], number: u32) -> io::Result<Option<u64>> {
    match f.iter().rev().find(|x| x.number == number) {
        None => Ok(None),
        Some(x) => match x.wire {
            0 => Ok(Some(varint(x.value, &mut 0)?)),
            1 => Ok(Some(u64::from_le_bytes(x.value.try_into().unwrap()))),
            5 => Ok(Some(u32::from_le_bytes(x.value.try_into().unwrap()) as u64)),
            _ => Err(bad("numeric_wire_type")),
        },
    }
}
pub fn bytes<'a>(f: &[Field<'a>], number: u32) -> io::Result<Option<&'a [u8]>> {
    match f.iter().rev().find(|x| x.number == number) {
        None => Ok(None),
        Some(x) if x.wire == 2 => Ok(Some(x.value)),
        _ => Err(bad("bytes_wire_type")),
    }
}
pub fn vi(n: u32, v: u64, b: &mut Vec<u8>) {
    put_varint((n as u64) << 3, b);
    put_varint(v, b);
}
pub fn bin(n: u32, v: &[u8], b: &mut Vec<u8>) {
    put_varint(((n as u64) << 3) | 2, b);
    put_varint(v.len() as u64, b);
    b.extend(v);
}
pub fn fixed64(n: u32, v: u64, b: &mut Vec<u8>) {
    put_varint(((n as u64) << 3) | 1, b);
    b.extend(v.to_le_bytes());
}
pub fn without(b: &[u8], numbers: &[u32]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    for f in fields(b)? {
        if !numbers.contains(&f.number) {
            out.extend(f.raw);
        }
    }
    Ok(out)
}
pub fn id(b: &[u8]) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(..4)
            .ok_or_else(|| bad("short_message"))?
            .try_into()
            .unwrap(),
    ) & 0x7fff_ffff)
}
pub fn proto(b: &[u8]) -> io::Result<(u32, &[u8], &[u8])> {
    if b.len() < 8 || b[3] & 128 == 0 {
        return Err(bad("expected_protobuf_message"));
    }
    let n = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    if n > b.len() - 8 || n > 65536 {
        return Err(bad("header_length"));
    }
    fields(&b[8..8 + n])?;
    Ok((id(b)?, &b[8..8 + n], &b[8 + n..]))
}
pub fn message(id: u32, header: &[u8], body: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + header.len() + body.len());
    b.extend((id | 0x8000_0000).to_le_bytes());
    b.extend((header.len() as u32).to_le_bytes());
    b.extend(header);
    b.extend(body);
    b
}
pub fn old_message(id: u32, body: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(id.to_le_bytes());
    b.extend(u64::MAX.to_le_bytes());
    b.extend(u64::MAX.to_le_bytes());
    b.extend(body);
    b
}
pub fn identity(b: &[u8], steamid: u64, session: u32) -> io::Result<Vec<u8>> {
    if b.len() >= 4 && b[3] & 128 != 0 {
        let (id, h, p) = proto(b)?;
        let mut h = without(h, &[1, 2])?;
        fixed64(1, steamid, &mut h);
        vi(2, session as i32 as i64 as u64, &mut h);
        Ok(message(id, &h, p))
    } else {
        // Extended Steam header: size=36, version=2, canary at 23, SteamID at 24, session at 32.
        if b.len() < 36 || b[4] != 36 || b[5..7] != [2, 0] || b[23] != 239 {
            return Err(bad("unsupported_legacy_header"));
        }
        let mut out = b.to_vec();
        out[24..32].copy_from_slice(&steamid.to_le_bytes());
        out[32..36].copy_from_slice(&session.to_le_bytes());
        Ok(out)
    }
}
pub fn flatten(b: &[u8]) -> io::Result<Vec<Vec<u8>>> {
    fn visit(b: &[u8], depth: u8, remaining: &mut usize, out: &mut Vec<Vec<u8>>) -> io::Result<()> {
        if depth > 4 || out.len() >= 1024 || b.len() > *remaining {
            return Err(bad("multi_budget"));
        }
        *remaining -= b.len();
        if id(b)? != 1 {
            out.push(b.to_vec());
            return Ok(());
        }
        let (_, _, body) = proto(b)?;
        let f = fields(body)?;
        let expected = uint(&f, 1)?.unwrap_or(0) as usize;
        let packed = bytes(&f, 2)?.ok_or_else(|| bad("multi_missing_body"))?;
        if expected > MAX_MULTI {
            return Err(bad("multi_inflate_limit"));
        }
        let decoded;
        let payload = if expected != 0 {
            let mut d = Vec::new();
            GzDecoder::new(packed)
                .take((expected + 1) as u64)
                .read_to_end(&mut d)
                .map_err(|_| bad("multi_gzip"))?;
            if d.len() != expected {
                return Err(bad("multi_inflate_length"));
            }
            decoded = d;
            &decoded[..]
        } else {
            packed
        };
        let mut p = 0;
        while p < payload.len() {
            let h = payload
                .get(p..p + 4)
                .ok_or_else(|| bad("multi_short_length"))?;
            p += 4;
            let n = u32::from_le_bytes(h.try_into().unwrap()) as usize;
            if !(4..=MAX_MESSAGE).contains(&n) {
                return Err(bad("multi_member_length"));
            }
            let end = p.checked_add(n).ok_or_else(|| bad("multi_member_length"))?;
            visit(
                payload.get(p..end).ok_or_else(|| bad("multi_truncated"))?,
                depth + 1,
                remaining,
                out,
            )?;
            p = end;
        }
        Ok(())
    }
    let mut out = Vec::new();
    visit(b, 0, &mut (MAX_MULTI * 2), &mut out)?;
    Ok(out)
}
