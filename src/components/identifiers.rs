use bevy::prelude::*;

#[derive(Component)]
pub struct UUID(uuid::Uuid);

impl Default for UUID {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
