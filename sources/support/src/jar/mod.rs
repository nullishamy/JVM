use std::io::{Read, Cursor};
use anyhow::{Result, Context};
use std::cell::RefCell;

pub struct JarFile {
    archive: RefCell<zip::ZipArchive<Cursor<Vec<u8>>>>
}

impl JarFile {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<JarFile> {
        let cursor = std::io::Cursor::new(bytes);
        let zip = zip::ZipArchive::new(cursor).context("could not read zipfile")?;

        Ok(Self {
            archive: RefCell::new(zip)
        })
    }

    pub fn locate_class(&self, class_name: &str) -> Result<Vec<u8>> {
        let mut archive = self.archive.borrow_mut();
        let mut classfile = archive.by_name(class_name).context("could not find class")?;
        let mut buf = Vec::new();
        classfile.read_to_end(&mut buf).context("could not read classfile")?;

        Ok(buf)
    }
}

