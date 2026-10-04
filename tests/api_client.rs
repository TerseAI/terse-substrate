use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::{BufMut, Bytes, BytesMut};
use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use prost::Message;
use terse_substrate::{Client, FileCredentials, Readiness, *};
use tower::service_fn;

type Reply = Response<Full<Bytes>>;

#[tokio::test]
async fn rereads_rotating_credentials_for_each_request() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("token");
    std::fs::write(&path, "first-token\n")?;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let tokens = observed.clone();
    let transport = service_fn(move |request: Request<tonic::body::Body>| {
        tokens.lock().unwrap().push(
            request.headers()["authorization"]
                .to_str()
                .unwrap()
                .to_owned(),
        );
        async { Ok::<_, Infallible>(reply(Actor::default())) }
    });
    let client = Client::new(
        transport,
        Arc::new(FileCredentials::new(&path)),
        Readiness::default(),
    );
    client.create_actor(Actor::default()).await?;
    std::fs::write(&path, "second-token")?;
    client.create_actor(Actor::default()).await?;
    assert_eq!(
        *observed.lock().unwrap(),
        ["Bearer first-token", "Bearer second-token"]
    );
    std::fs::write(&path, " \n")?;
    assert!(client.create_actor(Actor::default()).await.is_err());
    assert_eq!(observed.lock().unwrap().len(), 2);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn existing_template_waits_until_its_golden_snapshot_is_ready()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("token");
    std::fs::write(&path, "test-token")?;
    let calls = Arc::new(Mutex::new(0));
    let observed = calls.clone();
    let transport = service_fn(move |request: Request<tonic::body::Body>| {
        let path = request.uri().path().to_owned();
        let observed = observed.clone();
        async move {
            if path.ends_with("/CreateActorTemplate") {
                return Ok::<_, Infallible>(status(6));
            }
            assert!(path.ends_with("/GetActorTemplate"));
            let mut calls = observed.lock().unwrap();
            *calls += 1;
            Ok(reply(ActorTemplate {
                status: Some(ActorTemplateStatus {
                    golden_snapshot_status: Some(GoldenSnapshotStatus {
                        golden_tag: (*calls == 2).then(|| ObjectRef {
                            atespace: "test".into(),
                            name: "ready".into(),
                        }),
                        ..Default::default()
                    }),
                }),
                ..Default::default()
            }))
        }
    });
    let client = Client::new(
        transport,
        Arc::new(FileCredentials::new(path)),
        Readiness::default(),
    );
    let ready = client.ensure_template(template()).await?;
    assert!(
        ready
            .status
            .unwrap()
            .golden_snapshot_status
            .unwrap()
            .golden_tag
            .is_some()
    );
    assert_eq!(*calls.lock().unwrap(), 2);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn template_readiness_has_a_caller_controlled_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("token");
    std::fs::write(&path, "test-token")?;
    let transport = service_fn(|_: Request<tonic::body::Body>| async {
        Ok::<_, Infallible>(reply(ActorTemplate::default()))
    });
    let client = Client::new(
        transport,
        Arc::new(FileCredentials::new(path)),
        Readiness {
            timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        },
    );
    assert!(
        client
            .ensure_template(template())
            .await
            .unwrap_err()
            .to_string()
            .contains("timed out")
    );
    Ok(())
}

#[tokio::test]
async fn actor_listing_follows_page_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("token");
    std::fs::write(&path, "test-token")?;
    let transport = service_fn(|request: Request<tonic::body::Body>| async move {
        let body = request.into_body().collect().await.unwrap().to_bytes();
        let request = ListActorsRequest::decode(&body[5..]).unwrap();
        assert_eq!(request.atespace, "test");
        let first = request.page_token.is_empty();
        if !first {
            assert_eq!(request.page_token, "next");
        }
        Ok::<_, Infallible>(reply(ListActorsResponse {
            actors: vec![Actor {
                metadata: Some(ResourceMetadata {
                    name: if first { "one" } else { "two" }.into(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            next_page_token: if first { "next" } else { "" }.into(),
        }))
    });
    let client = Client::new(
        transport,
        Arc::new(FileCredentials::new(path)),
        Readiness::default(),
    );
    let actors = client.list_actors("test").await?;
    assert_eq!(
        actors
            .iter()
            .map(|actor| actor.metadata.as_ref().unwrap().name.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    Ok(())
}

#[tokio::test]
async fn deletion_preserves_identity_preconditions_and_upstream_conflicts() -> anyhow::Result<()> {
    let transport = service_fn(|request: Request<tonic::body::Body>| async move {
        assert!(request.uri().path().ends_with("/DeleteActor"));
        let body = request.into_body().collect().await.unwrap().to_bytes();
        let request = DeleteActorRequest::decode(&body[5..]).unwrap();
        assert!(request.any_state);
        assert_eq!(
            request.actor.unwrap(),
            ObjectRef {
                atespace: "test".into(),
                name: "actor".into()
            }
        );
        let options = request.options.unwrap();
        assert_eq!(options.uid, "actor-uid");
        assert_eq!(options.version, 42);
        Ok::<_, Infallible>(status(10))
    });
    let client = Client::new(transport, Arc::new(TestCredentials), Readiness::default());
    let error = client
        .delete_actor(DeleteActorRequest {
            actor: Some(ObjectRef {
                atespace: "test".into(),
                name: "actor".into(),
            }),
            any_state: true,
            options: Some(DeleteOptions {
                uid: "actor-uid".into(),
                version: 42,
            }),
        })
        .await
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<tonic::Status>().unwrap().code(),
        tonic::Code::Aborted
    );
    Ok(())
}

#[tokio::test]
async fn failed_golden_snapshot_returns_its_diagnostic_without_polling() -> anyhow::Result<()> {
    let calls = Arc::new(Mutex::new(0));
    let observed = calls.clone();
    let transport = service_fn(move |_: Request<tonic::body::Body>| {
        *observed.lock().unwrap() += 1;
        async {
            Ok::<_, Infallible>(reply(ActorTemplate {
                status: Some(ActorTemplateStatus {
                    golden_snapshot_status: Some(GoldenSnapshotStatus {
                        error_message: "image preparation failed".into(),
                        ..Default::default()
                    }),
                }),
                ..Default::default()
            }))
        }
    });
    let client = Client::new(transport, Arc::new(TestCredentials), Readiness::default());
    let error = client
        .wait_template(ObjectRef {
            atespace: "test".into(),
            name: "template".into(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("image preparation failed"));
    assert_eq!(*calls.lock().unwrap(), 1);
    Ok(())
}

struct TestCredentials;

#[async_trait::async_trait]
impl Credentials for TestCredentials {
    async fn token(&self) -> anyhow::Result<String> {
        Ok("test-token".into())
    }
}

fn template() -> ActorTemplate {
    ActorTemplate {
        metadata: Some(ResourceMetadata {
            atespace: "test".into(),
            name: "runtime".into(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn reply(message: impl Message) -> Reply {
    let bytes = message.encode_to_vec();
    let mut frame = BytesMut::with_capacity(bytes.len() + 5);
    frame.put_u8(0);
    frame.put_u32(bytes.len().try_into().unwrap());
    frame.extend_from_slice(&bytes);
    Response::builder()
        .header("content-type", "application/grpc")
        .header("grpc-status", "0")
        .body(Full::new(frame.freeze()))
        .unwrap()
}

fn status(code: u8) -> Reply {
    Response::builder()
        .header("content-type", "application/grpc")
        .header("grpc-status", code.to_string())
        .body(Full::new(Bytes::new()))
        .unwrap()
}
