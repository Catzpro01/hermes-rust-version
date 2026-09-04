use futures::StreamExt;
use hermes_core::{
    config::SecretString,
    conversation::{Event, Turn},
    provider::{HttpProvider, Provider},
};
use url::Url;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn http_provider_streams_openai_sse_and_authenticates() {
    let server = MockServer::start().await;
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\ndata: [DONE]\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let provider = HttpProvider::new(
        Url::parse(&(server.uri() + "/")).unwrap(),
        SecretString::from("test-key"),
        "test-model",
    );
    let stream = provider
        .chat(&[Turn::User {
            content: "hi".into(),
        }])
        .await
        .unwrap();
    let events: Vec<_> = stream
        .collect::<Vec<Result<Event, hermes_core::provider::ProviderError>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        events,
        vec![
            Event::Started,
            Event::Chunk("hello".into()),
            Event::Chunk(" world".into()),
            Event::Done
        ]
    );
}
