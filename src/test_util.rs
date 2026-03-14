#[cfg(test)]
pub(crate) struct TempTestDir {
    dir: tempfile::TempDir,
}

#[cfg(test)]
impl TempTestDir {
    pub fn new(name: &str) -> Self {
        let dir = tempfile::Builder::new().prefix(name).tempdir().unwrap();
        Self { dir }
    }

    pub fn create_file(&self, name: &str, content: &str) {
        std::fs::write(self.dir.path().join(name), content).unwrap();
    }

    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }
}
