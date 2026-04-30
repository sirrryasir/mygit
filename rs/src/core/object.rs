use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

pub fn read_object(sha: &str) -> (String, Vec<u8>) {
    read_object_raw(sha).expect(&format!("Object {} not found", sha))
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub path: String,
    pub sha1: String,
}

pub fn read_tree(sha: &str) -> Vec<TreeEntry> {
    let (obj_type, data) = read_object(sha);
    if obj_type != "tree" {
        panic!("Object {} is not a tree", sha);
    }

    let mut entries = Vec::new();
    let mut i = 0;
    while i < data.len() {
        // Mode
        let space_pos = data[i..].iter().position(|&b| b == b' ').unwrap();
        let mode = String::from_utf8_lossy(&data[i..i + space_pos]).to_string();
        i += space_pos + 1;

        // Path
        let null_pos = data[i..].iter().position(|&b| b == 0).unwrap();
        let path = String::from_utf8_lossy(&data[i..i + null_pos]).to_string();
        i += null_pos + 1;

        // SHA1 (binary)
        let sha1 = hex::encode(&data[i..i + 20]);
        i += 20;

        entries.push(TreeEntry { mode, path, sha1 });
    }
    entries
}

pub fn read_object_raw(sha: &str) -> Option<(String, Vec<u8>)> {
    // 1. Try loose object
    let path = format!(".git/objects/{}/{}", &sha[..2], &sha[2..]);
    if let Ok(data) = fs::read(path) {
        let mut decoder = ZlibDecoder::new(&data[..]);
        let mut decompressed = Vec::new();
        if decoder.read_to_end(&mut decompressed).is_ok() {
            if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
                let header = String::from_utf8_lossy(&decompressed[..null_pos]);
                let parts: Vec<&str> = header.split(' ').collect();
                return Some((parts[0].to_string(), decompressed[null_pos + 1..].to_vec()));
            }
        }
    }

    // 2. Try packfiles
    let packs_dir = Path::new(".git/objects/pack");
    if packs_dir.exists() {
        if let Ok(entries) = fs::read_dir(packs_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "pack") {
                        if let Some(obj) = search_packfile(&path, sha) {
                            return Some(obj);
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn search_packfile(pack_path: &Path, target_sha: &str) -> Option<(String, Vec<u8>)> {
    let idx_path = pack_path.with_extension("idx");
    if !idx_path.exists() {
        return None;
    }

    let idx_data = fs::read(idx_path).ok()?;
    let pack_data = fs::read(pack_path).ok()?;

    if &idx_data[0..4] != b"\xFFtOc" {
        return None;
    }

    let num_objects = u32::from_be_bytes(idx_data[255 * 4 + 4..255 * 4 + 8].try_into().unwrap());
    let target_bytes = hex::decode(target_sha).ok()?;

    let mut low = 0;
    let mut high = num_objects;
    let mut found_idx = None;

    while low < high {
        let mid = (low + high) / 2;
        let start = 8 + 1024 + (mid as usize * 20);
        let current_sha = &idx_data[start..start + 20];
        if current_sha == target_bytes.as_slice() {
            found_idx = Some(mid);
            break;
        } else if current_sha < target_bytes.as_slice() {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    let idx = found_idx?;
    let offset_start =
        8 + 1024 + (num_objects as usize * 20) + (num_objects as usize * 4) + (idx as usize * 4);
    let offset =
        u32::from_be_bytes(idx_data[offset_start..offset_start + 4].try_into().unwrap()) as usize;

    unpack_object(&pack_data, offset, pack_path)
}

fn unpack_object(pack_data: &[u8], offset: usize, pack_path: &Path) -> Option<(String, Vec<u8>)> {
    let mut cursor = offset;
    let mut byte = pack_data[cursor];
    cursor += 1;
    let obj_type = (byte >> 4) & 0x7;
    let mut _size = (byte & 0xF) as u64;
    let mut shift = 4;
    while byte & 0x80 != 0 {
        byte = pack_data[cursor];
        cursor += 1;
        _size |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
    }

    match obj_type {
        1..=4 => {
            let mut decoder = ZlibDecoder::new(&pack_data[cursor..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).ok()?;
            let type_str = match obj_type {
                1 => "commit",
                2 => "tree",
                3 => "blob",
                4 => "tag",
                _ => "unknown",
            };
            Some((type_str.to_string(), decompressed))
        }
        6 => {
            // OFS_DELTA
            let mut byte = pack_data[cursor];
            cursor += 1;
            let mut rel_offset = (byte & 0x7F) as usize;
            while byte & 0x80 != 0 {
                rel_offset += 1;
                byte = pack_data[cursor];
                cursor += 1;
                rel_offset = (rel_offset << 7) | ((byte & 0x7F) as usize);
            }
            let base_offset = offset - rel_offset;
            let (base_type, base_data) = unpack_object(pack_data, base_offset, pack_path)?;

            let mut decoder = ZlibDecoder::new(&pack_data[cursor..]);
            let mut delta_data = Vec::new();
            decoder.read_to_end(&mut delta_data).ok()?;

            let result = apply_delta(&base_data, &delta_data);
            Some((base_type, result))
        }
        7 => {
            // REF_DELTA
            let base_sha = hex::encode(&pack_data[cursor..cursor + 20]);
            cursor += 20;
            let (base_type, base_data) = read_object_raw(&base_sha)?;

            let mut decoder = ZlibDecoder::new(&pack_data[cursor..]);
            let mut delta_data = Vec::new();
            decoder.read_to_end(&mut delta_data).ok()?;

            let result = apply_delta(&base_data, &delta_data);
            Some((base_type, result))
        }
        _ => None,
    }
}

fn apply_delta(base: &[u8], delta: &[u8]) -> Vec<u8> {
    let mut cursor = 0;

    fn read_size(data: &[u8], cursor: &mut usize) -> usize {
        let mut size = 0;
        let mut shift = 0;
        loop {
            let byte = data[*cursor];
            *cursor += 1;
            size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        size
    }

    let _base_size = read_size(delta, &mut cursor);
    let result_size = read_size(delta, &mut cursor);
    let mut result = Vec::with_capacity(result_size);

    while cursor < delta.len() {
        let cmd = delta[cursor];
        cursor += 1;
        if cmd & 0x80 != 0 {
            // Copy
            let mut offset = 0usize;
            if cmd & 0x01 != 0 {
                offset |= delta[cursor] as usize;
                cursor += 1;
            }
            if cmd & 0x02 != 0 {
                offset |= (delta[cursor] as usize) << 8;
                cursor += 1;
            }
            if cmd & 0x04 != 0 {
                offset |= (delta[cursor] as usize) << 16;
                cursor += 1;
            }
            if cmd & 0x08 != 0 {
                offset |= (delta[cursor] as usize) << 24;
                cursor += 1;
            }

            let mut size = 0usize;
            if cmd & 0x10 != 0 {
                size |= delta[cursor] as usize;
                cursor += 1;
            }
            if cmd & 0x20 != 0 {
                size |= (delta[cursor] as usize) << 8;
                cursor += 1;
            }
            if cmd & 0x40 != 0 {
                size |= (delta[cursor] as usize) << 16;
                cursor += 1;
            }
            if size == 0 {
                size = 0x10000;
            }

            result.extend_from_slice(&base[offset..offset + size]);
        } else if cmd > 0 {
            // Insert
            let size = cmd as usize;
            result.extend_from_slice(&delta[cursor..cursor + size]);
            cursor += size;
        }
    }
    result
}

pub fn write_object(obj_type: &str, data: &[u8]) -> String {
    let header = format!("{} {}\0", obj_type, data.len());
    let mut full_data = Vec::new();
    full_data.extend_from_slice(header.as_bytes());
    full_data.extend_from_slice(data);

    let mut hasher = Sha1::new();
    hasher.update(&full_data);
    let sha = hex::encode(hasher.finalize());

    let dir = format!(".git/objects/{}", &sha[..2]);
    if !Path::new(&dir).exists() {
        fs::create_dir_all(&dir).unwrap();
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&full_data).unwrap();
    let compressed_data = encoder.finish().unwrap();

    fs::write(format!("{}/{}", dir, &sha[2..]), compressed_data).unwrap();
    sha
}

pub fn hash_object_internal(path: &Path, write: bool) -> String {
    let content = fs::read(path).expect("Could not read file");
    if write {
        write_object("blob", &content)
    } else {
        let header = format!("blob {}\0", content.len());
        let mut full_data = Vec::new();
        full_data.extend_from_slice(header.as_bytes());
        full_data.extend_from_slice(&content);
        let mut hasher = Sha1::new();
        hasher.update(&full_data);
        hex::encode(hasher.finalize())
    }
}
