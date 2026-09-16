//! `.mar` — Minc archive, a ZIP (like a JAR) holding compiled MINCB plus
//! targeting metadata. Not a Minecraft world and not a JVM jar.

use crate::config::MincConfig;

const LOCAL: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const CENTRAL: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

/// Build a `.mar` ZIP: `META-INF/MANIFEST.MF` + `pack.mincb`.
pub fn build(config: &MincConfig, mincb: &[u8]) -> Vec<u8> {
    let manifest = manifest_text(config);
    write_zip(&[
        ("META-INF/MANIFEST.MF", manifest.as_bytes()),
        ("pack.mincb", mincb),
    ])
}

pub fn manifest_text(config: &MincConfig) -> String {
    format!(
        "\
Manifest-Version: 1.0
Minc-Format: 1
Created-By: minc
Edition: {}
Game-Version: {}
Pack: {}
Name: {}
Score-Revision: {}
Origin: {},{},{}
",
        config.edition.as_str(),
        config.game_version,
        config.pack,
        config.name,
        config.score_revision,
        config.origin[0],
        config.origin[1],
        config.origin[2],
    )
}

pub fn is_mar(bytes: &[u8]) -> bool {
    bytes.starts_with(&LOCAL) || bytes.starts_with(&EOCD)
}

/// Pull `pack.mincb` (and MANIFEST if present) from `.mar` or raw MINCB bytes.
pub fn unwrap_payload(bytes: &[u8]) -> Result<(Vec<u8>, Option<String>), String> {
    if bytes.starts_with(b"MINC") {
        return Ok((bytes.to_vec(), None));
    }
    let ar = open(bytes)?;
    Ok((ar.mincb, Some(ar.manifest)))
}

/// Pull `pack.mincb` and MANIFEST out of a STORE-only `.mar`.
pub fn open(bytes: &[u8]) -> Result<MarArchive, String> {
    let files = read_zip(bytes)?;
    let mincb = files
        .iter()
        .find(|(n, _)| n == "pack.mincb")
        .map(|(_, b)| b.clone())
        .ok_or_else(|| "mar missing pack.mincb".to_string())?;
    let manifest = files
        .iter()
        .find(|(n, _)| n == "META-INF/MANIFEST.MF")
        .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default();
    Ok(MarArchive { mincb, manifest })
}

#[derive(Debug, Clone)]
pub struct MarArchive {
    pub mincb: Vec<u8>,
    pub manifest: String,
}

impl MarArchive {
    pub fn edition(&self) -> Option<&str> {
        manifest_field(&self.manifest, "Edition")
    }

    pub fn game_version(&self) -> Option<&str> {
        manifest_field(&self.manifest, "Game-Version")
    }

    pub fn pack(&self) -> Option<&str> {
        manifest_field(&self.manifest, "Pack")
    }
}

fn manifest_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}: ");
    text.lines()
        .find(|l| l.starts_with(&prefix))
        .map(|l| l[prefix.len()..].trim())
}

struct ZipEntry {
    name: String,
    crc: u32,
    size: u32,
    offset: u32,
}

fn write_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut entries = Vec::new();
    for (name, data) in files {
        let crc = crc32(data);
        let offset = out.len() as u32;
        let name_b = name.as_bytes();
        out.extend_from_slice(&LOCAL);
        out.extend_from_slice(&20u16.to_le_bytes()); // version
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // store
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0u16.to_le_bytes()); // date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(name_b);
        out.extend_from_slice(data);
        entries.push(ZipEntry {
            name: (*name).to_string(),
            crc,
            size: data.len() as u32,
            offset,
        });
    }
    let cd_start = out.len() as u32;
    for e in &entries {
        let name_b = e.name.as_bytes();
        out.extend_from_slice(&CENTRAL);
        out.extend_from_slice(&20u16.to_le_bytes()); // made by
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&e.size.to_le_bytes());
        out.extend_from_slice(&e.size.to_le_bytes());
        out.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(&0u16.to_le_bytes()); // comment
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&e.offset.to_le_bytes());
        out.extend_from_slice(name_b);
    }
    let cd_size = out.len() as u32 - cd_start;
    let n = entries.len() as u16;
    out.extend_from_slice(&EOCD);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut i = 0;
    let mut files = Vec::new();
    while i + 30 <= bytes.len() {
        if bytes[i..].starts_with(&EOCD) || bytes[i..].starts_with(&CENTRAL) {
            break;
        }
        if !bytes[i..].starts_with(&LOCAL) {
            return Err("not a zip/mar archive".into());
        }
        let method = u16::from_le_bytes(bytes[i + 8..i + 10].try_into().unwrap());
        if method != 0 {
            return Err("mar uses STORE compression only".into());
        }
        let size = u32::from_le_bytes(bytes[i + 22..i + 26].try_into().unwrap()) as usize;
        let name_len = u16::from_le_bytes(bytes[i + 26..i + 28].try_into().unwrap()) as usize;
        let extra = u16::from_le_bytes(bytes[i + 28..i + 30].try_into().unwrap()) as usize;
        let name_at = i + 30;
        let data_at = name_at + name_len + extra;
        if data_at + size > bytes.len() {
            return Err("truncated mar".into());
        }
        let name = String::from_utf8_lossy(&bytes[name_at..name_at + name_len]).into_owned();
        files.push((name, bytes[data_at..data_at + size].to_vec()));
        i = data_at + size;
    }
    if files.is_empty() {
        return Err("empty mar".into());
    }
    Ok(files)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MincConfig;

    #[test]
    fn mar_roundtrip_holds_mincb_and_edition() {
        let cfg = MincConfig {
            name: "kit".into(),
            pack: "demo.kit".into(),
            edition: crate::config::Edition::Bedrock,
            game_version: "1.21.70".into(),
            ..MincConfig::default()
        };
        let blob = build(&cfg, b"MINCtest");
        assert!(is_mar(&blob));
        let ar = open(&blob).expect("open");
        assert_eq!(ar.mincb, b"MINCtest");
        assert_eq!(ar.edition(), Some("bedrock"));
        assert_eq!(ar.game_version(), Some("1.21.70"));
        assert_eq!(ar.pack(), Some("demo.kit"));
        assert!(ar.manifest.contains("Minc-Format: 1"));
    }
}
