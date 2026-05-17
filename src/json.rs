use std::fs;
use std::path::Path;
use std::io::Read;
use crate::components::behaviour::Behaviours;
use crate::components::belief::Beliefs;
use crate::components::agent::Agents;

pub fn load_behaviours_from_json<P: AsRef<Path>>(path: P) -> Behaviours {
    let json = fs::read_to_string(path).expect("Unable to read behaviours file");
    serde_json::from_str(&json).expect("JSON was not well-formatted")
}

pub fn load_beliefs_from_json<P: AsRef<Path>>(path: P) -> Beliefs {
    let json = fs::read_to_string(path).expect("Unable to read beliefs file");
    serde_json::from_str(&json).expect("JSON was not well-formatted")
}

pub fn load_agents_from_zstd_json<P: AsRef<Path>>(path: P) -> Agents {
    let file = fs::File::open(path).expect("Unable to open agents file");
    let mut decoder = zstd::Decoder::new(file).expect("Unable to create zstd decoder");
    let mut json = String::new();
    decoder.read_to_string(&mut json).expect("Unable to decompress agents file");
    serde_json::from_str(&json).expect("JSON was not well-formatted")
}
