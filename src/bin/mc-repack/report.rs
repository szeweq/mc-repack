use std::{collections::BTreeMap, fs, io, path};

pub struct Report(Box<path::Path>, Vec<(Box<str>, u64, u64)>);
impl Report {
    pub const fn new(p: Box<path::Path>) -> Self {
        Self(p, Vec::new())
    }
    pub fn push(&mut self, name: &str, old_size: u64, new_size: u64) {
        self.1.push((name.into(), old_size, new_size));
    }
    pub fn save_csv(&self) -> io::Result<()> {
        use io::Write;
        fn write_impl(w: &mut io::BufWriter<fs::File>, v: &[(Box<str>, u64, u64)]) -> io::Result<()> {
            writeln!(w, "name,old_size,new_size")?;
            for (name, old_size, new_size) in v {
                writeln!(w, "{name},{old_size},{new_size}")?;
            }
            Ok(())
        }
        let mut w = io::BufWriter::new(fs::File::create(&self.0)?);
        if let Err(e) = write_impl(&mut w, &self.1) {
            fs::remove_file(&self.0)?;
            return Err(e);
        }
        Ok(())
    }
}

/// A list of well-known file formats that should not be considered unknown.
const WELL_KNOWN: &[&str] = &[
    "class", "txt"
];

/// A catalogue of unknown file formats encountered while repacking.
pub struct Unknowns(Box<path::Path>, BTreeMap<Box<str>, u64>);
impl Unknowns {
    pub const fn new(p: Box<path::Path>) -> Self {
        Self(p, BTreeMap::new())
    }
    pub fn push(&mut self, ext: &str) {
        if WELL_KNOWN.binary_search(&ext).is_ok() { return }
        *self.1.entry(ext.to_ascii_lowercase().into()).or_default() += 1;
    }
    pub fn save(&self) -> io::Result<()> {
        use io::Write;
        let mut w = io::BufWriter::new(fs::File::create(&self.0)?);
        let mut v = self.1.iter().map(|(ext, count)| (ext, *count)).collect::<Vec<_>>();
        v.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        if v.is_empty() {
            writeln!(w, "No unknown formats found.")?;
        }
        for (ext, count) in &v {
            writeln!(w, "{ext}: {count}")?;
        }
        Ok(())
    }
}