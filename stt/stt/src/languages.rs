#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
    pub native_name: &'static str,
}

impl Language {
    pub const fn new(code: &'static str, name: &'static str, native_name: &'static str) -> Self {
        Self {
            code,
            name,
            native_name,
        }
    }
}
