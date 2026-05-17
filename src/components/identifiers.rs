use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct Uuid(pub uuid::Uuid);

impl Default for Uuid {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
