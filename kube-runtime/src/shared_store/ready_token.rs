use parking_lot::RwLock;
use std::sync::Arc;

/// A token that can be used to track readiness of multiple components.
#[derive(Clone, Default)]
pub struct ReadyToken(Arc<RwLock<Vec<ReadyState>>>);

#[derive(Clone)]
pub struct ReadyState(Arc<RwLock<bool>>);

impl ReadyToken {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(vec![])))
    }

    pub fn is_ready(&self) -> bool {
        self.0.read().iter().all(|state| *state.0.read())
    }

    pub fn child(&self) -> ReadyState {
        let state = ReadyState::new();
        self.0.write().push(state.clone());
        state
    }
}

impl ReadyState {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(false)))
    }

    pub fn ready(&self) {
        *(self.0.write()) = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_is_ready_if_it_has_no_ready_states_to_track() {
        assert!(ReadyToken::new().is_ready());
    }

    #[test]
    fn it_is_not_ready_if_it_has_a_state_that_is_not_ready() {
        let rt = ReadyToken::new();
        let _ = rt.child();
        assert!(!rt.is_ready());
    }

    #[test]
    fn it_is_ready_when_the_child_state_is_ready() {
        let rt = ReadyToken::new();
        let mut child = rt.child();
        child.ready();
        assert!(rt.is_ready());
    }

    #[test]
    fn it_works_for_multiple_child_states() {
        let rt = ReadyToken::new();
        let mut child1 = rt.child();
        let mut child2 = rt.child();
        child1.ready();
        assert!(!rt.is_ready());
        child2.ready();
        assert!(rt.is_ready());
    }
}
