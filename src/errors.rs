use std::error::Error;

/// This trait helps with collecting errors and returning the results.
pub trait ErrorCollector {
    /// Collects errors for files based on their name (path).
    fn collect(&mut self, name: String, e: Box<dyn Error>);

    /// Returns all currently gathered results.
    fn get_results(&mut self) -> Vec<(String, String)>;
}

impl ErrorCollector for Vec<(String, String)> {
    fn collect(&mut self, name: String, e: Box<dyn Error>) {
        self.push((name, e.to_string()))
    }
    fn get_results(&mut self) -> Vec<(String, String)> {
        let nv = self.clone();
        self.clear();
        nv
    }
}

/// A silent version of ErrorCollector that does nothing and returns no results.
pub struct SilentCollector;
impl ErrorCollector for SilentCollector {
    fn collect(&mut self, _name: String, _e: Box<dyn Error>) {
        
    }
    fn get_results(&mut self) -> Vec<(String, String)> {
        vec![]
    }
}