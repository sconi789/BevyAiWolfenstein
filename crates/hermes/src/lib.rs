use bevy_app::{App, Plugin};
use bevy_ecs::event::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Producer,
    GameDesigner,
    LevelDesigner,
    GameplayProgrammer,
    ArtDirector,
    QaTester,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HermesTopic {
    ProducerGate,
    LevelPlan,
    Gameplay,
    Qa,
}

#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct HermesEvent {
    pub topic: HermesTopic,
    pub from: AgentRole,
    pub message: String,
}

pub struct HermesPlugin;

impl Plugin for HermesPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HermesEvent>();
    }
}

