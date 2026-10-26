use async_trait::async_trait;

use crate::{AgentState, Error, Message, Middleware, Role};

pub struct SlidingWindow {
    keep: usize,
}

impl SlidingWindow {
    /// Creates a sliding window that keeps `keep` complete turns.
    ///
    /// `keep` is clamped to at least 1, so the window never drops the turn
    /// that is currently being generated.
    pub fn new(keep: usize) -> Self {
        Self { keep: keep.max(1) }
    }
}

#[async_trait]
impl Middleware for SlidingWindow {
    async fn before_model(&self, state: &mut AgentState) -> Result<(), Error> {
        // The system prompt, if any, always leads the conversation and is not
        // counted against the window.
        let system_len = usize::from(
            state
                .messages
                .first()
                .is_some_and(|m| m.role == Role::System),
        );
        let start = window_start(&state.messages[system_len..], self.keep);
        if start == 0 {
            return Ok(());
        }

        state.messages.drain(system_len..system_len + start);
        Ok(())
    }
}

/// Returns the index of the first message to keep, counting back `keep`
/// complete turns from the end.
///
/// Returns 0 when the history holds no more than `keep` turns.
fn window_start(history: &[Message], keep: usize) -> usize {
    let mut turns = 0;
    for i in (0..history.len()).rev() {
        if is_turn_start(history, i) {
            turns += 1;
            if turns >= keep {
                return i;
            }
        }
    }
    0
}

/// Returns whether `messages[i]` begins a self-contained turn.
///
/// A turn starts at a user message that is not the continuation of a tool-call
/// round, so a window boundary can never split an `assistant.tool_calls` from
/// its `tool` results.
fn is_turn_start(messages: &[Message], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = &messages[i - 1];
    messages[i].role == Role::User && prev.role != Role::Tool && prev.tool_call_id.is_none()
}
