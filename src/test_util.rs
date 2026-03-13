#[cfg(test)]
pub(crate) struct TempTestDir {
    pub path: std::path::PathBuf,
}

#[cfg(test)]
impl TempTestDir {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn create_file(&self, name: &str, content: &str) {
        std::fs::write(self.path.join(name), content).unwrap();
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[cfg(test)]
impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
