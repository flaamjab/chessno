use std::path::PathBuf;

pub struct PathWrangler(String);

impl PathWrangler {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn with_os_convention(mut self) -> Self {
        if cfg!(unix) {
            self.0 = self.0.replace("\\", "/");
        } else if cfg!(windows) {
            self.0 = self.0.replace("/", "\\");
        }

        self
    }

    pub fn finish(self) -> PathBuf {
        self.0.into()
    }
}
