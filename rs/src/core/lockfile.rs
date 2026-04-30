use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct LockFile {
    pub path: PathBuf,
    pub lock_path: PathBuf,
    pub file: Option<File>,
}

impl LockFile {
    pub fn new(path: &Path) -> Self {
        let mut lock_path = path.to_path_buf();
        lock_path.set_extension("lock");
        Self {
            path: path.to_path_buf(),
            lock_path,
            file: None,
        }
    }

    pub fn hold_for_update(&mut self) -> io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock_path)?;
        self.file = Some(file);
        Ok(())
    }

    pub fn commit(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.take() {
            drop(file);
            std::fs::rename(&self.lock_path, &self.path)?;
        }
        Ok(())
    }

    pub fn rollback(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.take() {
            drop(file);
            if self.lock_path.exists() {
                std::fs::remove_file(&self.lock_path)?;
            }
        }
        Ok(())
    }
}

impl Write for LockFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.as_ref().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_ref().unwrap().flush()
    }
}
