use std::error::Error;

pub trait ErrorCollector {
    fn collect(&mut self, name: String, e: Box<dyn Error>);
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

pub struct SilentCollector;
impl ErrorCollector for SilentCollector {
    fn collect(&mut self, _name: String, _e: Box<dyn Error>) {
        
    }
    fn get_results(&mut self) -> Vec<(String, String)> {
        vec![]
    }
}