use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct UUID(pub uuid::Uuid);

impl Default for UUID {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
