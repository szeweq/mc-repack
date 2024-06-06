use std::fs;

pub struct Report(Box<std::path::Path>, Vec<(Box<str>, u64, u64)>);
impl Report {
    pub fn new(p: Box<std::path::Path>) -> Self {
        Self(p, Vec::new())
    }
    pub fn push(&mut self, name: &str, old_size: u64, new_size: u64) {
        self.1.push((name.into(), old_size, new_size));
    }
    pub fn save_csv(&self) -> std::io::Result<()> {
        use std::io::Write;
        fn write_impl(w: &mut fs::File, v: &[(Box<str>, u64, u64)]) -> std::io::Result<()> {
            writeln!(w, "name,old_size,new_size")?;
            for (name, old_size, new_size) in v {
                writeln!(w, "{},{},{}", name, old_size, new_size)?;
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