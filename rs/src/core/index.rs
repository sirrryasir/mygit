use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub ctime_s: u32,
    pub ctime_n: u32,
    pub mtime_s: u32,
    pub mtime_n: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub sha1: [u8; 20],
    pub flags: u16,
    pub path: String,
}

impl IndexEntry {
    pub fn matches_stat(&self, metadata: &fs::Metadata) -> bool {
        self.mtime_s == metadata.mtime() as u32
            && self.mtime_n == metadata.mtime_nsec() as u32
            && self.size == metadata.size() as u32
            && self.ino == metadata.ino() as u32
            && self.dev == metadata.dev() as u32
    }
}

pub struct Index {
    pub entries: BTreeMap<String, IndexEntry>,
}

impl Index {
    pub fn load() -> Self {
        let path = Path::new(".git/index");
        if !path.exists() {
            return Index {
                entries: BTreeMap::new(),
            };
        }

        let data = fs::read(path).expect("Could not read index");
        if data.len() < 12 {
            return Index {
                entries: BTreeMap::new(),
            };
        }

        let num_entries = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let mut offset = 12;
        let mut entries = BTreeMap::new();

        for _ in 0..num_entries {
            let entry_start = offset;
            let ctime_s = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
            let ctime_n = u32::from_be_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            let mtime_s = u32::from_be_bytes(data[offset + 8..offset + 12].try_into().unwrap());
            let mtime_n = u32::from_be_bytes(data[offset + 12..offset + 16].try_into().unwrap());
            let dev = u32::from_be_bytes(data[offset + 16..offset + 20].try_into().unwrap());
            let ino = u32::from_be_bytes(data[offset + 20..offset + 24].try_into().unwrap());
            let mode = u32::from_be_bytes(data[offset + 24..offset + 28].try_into().unwrap());
            let uid = u32::from_be_bytes(data[offset + 28..offset + 32].try_into().unwrap());
            let gid = u32::from_be_bytes(data[offset + 32..offset + 36].try_into().unwrap());
            let size = u32::from_be_bytes(data[offset + 36..offset + 40].try_into().unwrap());
            let mut sha1 = [0u8; 20];
            sha1.copy_from_slice(&data[offset + 40..offset + 60]);
            let flags = u16::from_be_bytes(data[offset + 60..offset + 62].try_into().unwrap());
            offset += 62;

            let path_len = (flags & 0xFFF) as usize;
            let path_str = String::from_utf8_lossy(&data[offset..offset + path_len]).to_string();
            offset += path_len;

            let entry_len = offset - entry_start;
            let padding = (8 - (entry_len % 8)) % 8;
            offset += padding;

            entries.insert(
                path_str.clone(),
                IndexEntry {
                    ctime_s,
                    ctime_n,
                    mtime_s,
                    mtime_n,
                    dev,
                    ino,
                    mode,
                    uid,
                    gid,
                    size,
                    sha1,
                    flags,
                    path: path_str,
                },
            );
        }

        Index { entries }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"DIRC");
        data.extend_from_slice(&2u32.to_be_bytes());
        data.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());

        for entry in self.entries.values() {
            let start = data.len();
            data.extend_from_slice(&entry.ctime_s.to_be_bytes());
            data.extend_from_slice(&entry.ctime_n.to_be_bytes());
            data.extend_from_slice(&entry.mtime_s.to_be_bytes());
            data.extend_from_slice(&entry.mtime_n.to_be_bytes());
            data.extend_from_slice(&entry.dev.to_be_bytes());
            data.extend_from_slice(&entry.ino.to_be_bytes());
            data.extend_from_slice(&entry.mode.to_be_bytes());
            data.extend_from_slice(&entry.uid.to_be_bytes());
            data.extend_from_slice(&entry.gid.to_be_bytes());
            data.extend_from_slice(&entry.size.to_be_bytes());
            data.extend_from_slice(&entry.sha1);
            data.extend_from_slice(&entry.flags.to_be_bytes());
            data.extend_from_slice(entry.path.as_bytes());

            let entry_len = data.len() - start;
            let padding = (8 - (entry_len % 8)) % 8;
            data.extend_from_slice(&vec![0u8; padding]);
        }

        let mut hasher = Sha1::new();
        hasher.update(&data);
        data.extend_from_slice(&hasher.finalize());
        data
    }

    pub fn write(&self) -> std::io::Result<()> {
        let mut lock = crate::core::lockfile::LockFile::new(Path::new(".git/index"));
        lock.hold_for_update()?;
        lock.write_all(&self.serialize())?;
        lock.commit()
    }

    pub fn add_entry(&mut self, path: String, sha: String, metadata: fs::Metadata) {
        let sha_bytes = hex::decode(sha).expect("Invalid SHA");
        let mut sha1 = [0u8; 20];
        sha1.copy_from_slice(&sha_bytes);

        let entry = IndexEntry {
            ctime_s: metadata.ctime() as u32,
            ctime_n: metadata.ctime_nsec() as u32,
            mtime_s: metadata.mtime() as u32,
            mtime_n: metadata.mtime_nsec() as u32,
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size() as u32,
            sha1,
            flags: (path.len() as u16) & 0xFFF,
            path: path.clone(),
        };
        self.entries.insert(path, entry);
    }
}
