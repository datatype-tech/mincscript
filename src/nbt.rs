//! Named Binary Tag encoder for structure files.
//!
//! Java structures are big-endian (optionally gzip-compressed). Bedrock
//! `.mcstructure` is uncompressed little-endian NBT.

use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Big,
    Little,
}

#[derive(Clone, Debug)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Value>),
    Compound(Vec<(String, Value)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Value {
    pub fn compound(pairs: Vec<(&str, Value)>) -> Self {
        Value::Compound(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn int_list(vals: impl IntoIterator<Item = i32>) -> Self {
        Value::List(vals.into_iter().map(Value::Int).collect())
    }

    fn tag_id(&self) -> u8 {
        match self {
            Value::Byte(_) => 1,
            Value::Short(_) => 2,
            Value::Int(_) => 3,
            Value::Long(_) => 4,
            Value::Float(_) => 5,
            Value::Double(_) => 6,
            Value::ByteArray(_) => 7,
            Value::String(_) => 8,
            Value::List(_) => 9,
            Value::Compound(_) => 10,
            Value::IntArray(_) => 11,
            Value::LongArray(_) => 12,
        }
    }
}

/// Encode a root named compound (standard NBT file payload, uncompressed).
pub fn encode_named(name: &str, value: &Value, endian: Endian) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(value.tag_id());
    write_string(&mut out, name, endian);
    write_payload(&mut out, value, endian);
    out
}

pub fn gzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bytes).map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

fn write_payload(out: &mut Vec<u8>, value: &Value, endian: Endian) {
    match value {
        Value::Byte(v) => out.push(*v as u8),
        Value::Short(v) => write_i16(out, *v, endian),
        Value::Int(v) => write_i32(out, *v, endian),
        Value::Long(v) => write_i64(out, *v, endian),
        Value::Float(v) => write_u32(out, v.to_bits(), endian),
        Value::Double(v) => write_u64(out, v.to_bits(), endian),
        Value::ByteArray(v) => {
            write_i32(out, v.len() as i32, endian);
            out.extend_from_slice(v);
        }
        Value::String(s) => write_string(out, s, endian),
        Value::List(items) => {
            let ty = items.first().map(Value::tag_id).unwrap_or(0);
            out.push(ty);
            write_i32(out, items.len() as i32, endian);
            for item in items {
                write_payload(out, item, endian);
            }
        }
        Value::Compound(pairs) => {
            for (name, v) in pairs {
                out.push(v.tag_id());
                write_string(out, name, endian);
                write_payload(out, v, endian);
            }
            out.push(0);
        }
        Value::IntArray(v) => {
            write_i32(out, v.len() as i32, endian);
            for n in v {
                write_i32(out, *n, endian);
            }
        }
        Value::LongArray(v) => {
            write_i32(out, v.len() as i32, endian);
            for n in v {
                write_i64(out, *n, endian);
            }
        }
    }
}

fn write_string(out: &mut Vec<u8>, s: &str, endian: Endian) {
    let bytes = s.as_bytes();
    write_u16(out, bytes.len() as u16, endian);
    out.extend_from_slice(bytes);
}

fn write_u16(out: &mut Vec<u8>, v: u16, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn write_i16(out: &mut Vec<u8>, v: i16, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn write_i32(out: &mut Vec<u8>, v: i32, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn write_u32(out: &mut Vec<u8>, v: u32, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn write_i64(out: &mut Vec<u8>, v: i64, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn write_u64(out: &mut Vec<u8>, v: u64, endian: Endian) {
    match endian {
        Endian::Big => out.extend_from_slice(&v.to_be_bytes()),
        Endian::Little => out.extend_from_slice(&v.to_le_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_compound_big_endian() {
        let bytes = encode_named("", &Value::Compound(Vec::new()), Endian::Big);
        assert_eq!(bytes, vec![10, 0, 0, 0]);
    }

    #[test]
    fn gzip_roundtrip_header() {
        let raw = encode_named("", &Value::Compound(Vec::new()), Endian::Big);
        let gz = gzip(&raw).unwrap();
        assert_eq!(&gz[0..2], &[0x1f, 0x8b]);
    }
}
