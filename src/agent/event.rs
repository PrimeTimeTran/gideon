use crate::agent::{AgentEvent, SystemEvent};

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    System(SystemEvent),
    Agent(AgentEvent),
}
