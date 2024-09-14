use std::{fs, io, path};

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
        fn write_impl(w: &mut fs::File, v: &[(Box<str>, u64, u64)]) -> io::Result<()> {
            writeln!(w, "name,old_size,new_size")?;
            for (name, old_size, new_size) in v {
                writeln!(w, "{name},{old_size},{new_size}")?;
            }
            Ok(())
        }
        let mut w = fs::File::create(&self.0)?;
        if let Err(e) = write_impl(&mut w, &self.1) {
            fs::remove_file(&self.0)?;
            return Err(e);
        }
        Ok(())
    }
}