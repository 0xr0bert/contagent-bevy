use std::fs;
use std::path::Path;
use std::io::{Read, Write};
use crate::components::behaviour::Behaviours;
use crate::components::belief::Beliefs;
use crate::components::agent::Agents;

use serde::{Deserialize, Serialize};

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

pub fn save_agents_to_zstd_json<P: AsRef<Path>>(path: P, agents: &Agents) {
    let json = serde_json::to_string(agents).expect("Failed to serialize agents");
    let file = fs::File::create(path).expect("Unable to create output file");
    let mut encoder = zstd::Encoder::new(file, 3).expect("Unable to create zstd encoder");
    encoder.write_all(json.as_bytes()).expect("Failed to write to zstd encoder");
    encoder.finish().expect("Failed to finish zstd compression");
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SummarySpec {
    pub mean_activations: std::collections::HashMap<String, f64>,
    pub sd_activations: std::collections::HashMap<String, f64>,
    pub median_activations: std::collections::HashMap<String, f64>,
    pub nonzero_activations: std::collections::HashMap<String, i32>,
    pub n_performers: std::collections::HashMap<String, i32>,
}

pub fn save_summaries_to_zstd_json<P: AsRef<Path>>(path: P, summaries: &Vec<SummarySpec>) {
    let json = serde_json::to_string(summaries).expect("Failed to serialize summaries");
    let file = fs::File::create(path).expect("Unable to create output file");
    let mut encoder = zstd::Encoder::new(file, 3).expect("Unable to create zstd encoder");
    encoder.write_all(json.as_bytes()).expect("Failed to write to zstd encoder");
    encoder.finish().expect("Failed to finish zstd compression");
}
