use crate::{
    Agent, AgentEvent, AgentState, ChatEvent, ChatModel, ChatRequest, ChatResponse, ChatStream,
    CheckpointMessage, Checkpointer, Context, DynamicPrompt, Error, InMemoryCheckpointer, Message,
    Role, ThreadId,
};
use async_trait::async_trait;
use futures_util::TryStreamExt;

struct EchoSystemPrompt;

#[async_trait]
impl ChatModel for EchoSystemPrompt {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        let system = &request.messages[0];
        assert_eq!(system.role, Role::System);
        Ok(ChatResponse {
            id: "mock".into(),
            model: "mock".into(),
            message: Message::assistant(system.text()),
            finish_reason: None,
            usage: None,
            inference_speed: None,
        })
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let system = &request.messages[0];
        assert_eq!(system.role, Role::System);
        let event = ChatEvent::Text(system.text().to_owned());
        Box::pin(futures_util::stream::iter([Ok(event)]))
    }
}

#[tokio::test]
async fn default_prompt_preserves_an_existing_leading_system_message() {
    let agent = Agent::builder()
        .model(EchoSystemPrompt)
        .request(ChatRequest::new("mock"))
        .build()
        .unwrap();

    let result = agent.invoke(Message::user("hello")).await.unwrap();
    assert_eq!(result.text(), "You are a helpful assistant.");

    let result = agent
        .invoke_messages([Message::system("custom"), Message::user("hello")])
        .await
        .unwrap();
    assert_eq!(result.text(), "custom");
}

#[tokio::test]
async fn dynamic_prompt_replaces_the_default_without_changing_checkpoint_history() {
    let checkpointer = InMemoryCheckpointer::new();
    let history = [
        Message::user("old question"),
        Message::assistant("old answer"),
    ];
    checkpointer
        .append(
            "thread",
            history
                .iter()
                .map(|message| CheckpointMessage::from_message(message, "thread", "old-run"))
                .collect(),
        )
        .await
        .unwrap();

    let agent = Agent::builder()
        .model(EchoSystemPrompt)
        .request(ChatRequest::new("mock"))
        .checkpointer(checkpointer)
        .middleware(DynamicPrompt::new(|state: &AgentState| {
            let system = &state.messages[0];
            assert_eq!(system.role, Role::System);
            assert_eq!(system.text(), "You are a helpful assistant.");
            format!("dynamic for {}", state.last().unwrap().text())
        }))
        .build()
        .unwrap();
    let mut context = Context::new();
    context.insert(ThreadId("thread".to_owned()));

    let events: Vec<_> = agent
        .stream_with_context(Message::user("stream question"), context.clone())
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        events,
        [
            AgentEvent::Text("dynamic for stream question".into()),
            AgentEvent::Done,
        ]
    );

    let result = agent
        .invoke_with_context(Message::user("invoke question"), context.clone())
        .await
        .unwrap();
    assert_eq!(result.text(), "dynamic for invoke question");

    let resumed = agent
        .invoke_with_context(Message::user("next question"), context)
        .await
        .unwrap();
    let texts: Vec<_> = resumed.messages().iter().map(Message::text).collect();
    assert_eq!(
        texts,
        [
            "dynamic for next question",
            "old question",
            "old answer",
            "stream question",
            "dynamic for stream question",
            "invoke question",
            "dynamic for invoke question",
            "next question",
            "dynamic for next question",
        ]
    );
}
