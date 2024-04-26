use std::fmt;

pub(crate) struct Buffer {
    inner: Vec<String>,
}

impl Buffer {
    pub(crate) fn new() -> Buffer {
        Buffer { inner: vec![] }
    }

    pub(crate) fn write(&mut self, v: &str) {
        self.inner.push(v.to_owned());
    }

    pub(crate) fn newline(&mut self) {
        self.inner.push("\n".to_owned());
    }

    pub(crate) fn write_quoted(&mut self, v: &str) {
        self.inner.push(format!("\"{v}\""));
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for chunk in &self.inner {
            write!(f, "{}", chunk)?;
        }

        Ok(())
    }
}
