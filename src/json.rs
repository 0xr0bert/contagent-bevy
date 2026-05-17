use std::fs;
use std::path::Path;
use crate::components::behaviour::Behaviours;

pub fn load_behaviours_from_json<P: AsRef<Path>>(path: P) -> Behaviours {
    let json = fs::read_to_string(path).expect("Unable to read behaviours file");
    serde_json::from_str(&json).expect("JSON was not well-formatted")
}
