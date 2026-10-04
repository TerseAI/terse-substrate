use std::convert::Infallible;

use bytes::{BufMut, Bytes, BytesMut};
use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use prost::Message;
use terse_substrate::{
    Actor, CreateActorRequest, ObjectRef, ResourceMetadata, control_client::ControlClient,
};
use tower::service_fn;

#[tokio::test]
async fn creates_an_actor_from_an_immutable_snapshot_tag() -> Result<(), Box<dyn std::error::Error>>
{
    let transport = service_fn(|request: Request<tonic::body::Body>| async move {
        assert_eq!(request.uri().path(), "/ateapi.Control/CreateActor");
        assert_eq!(request.headers()["content-type"], "application/grpc");
        let body = request.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body[0], 0);
        assert_eq!(
            u32::from_be_bytes(body[1..5].try_into().unwrap()) as usize,
            body.len() - 5
        );
        let mut actor = CreateActorRequest::decode(&body[5..])
            .unwrap()
            .actor
            .unwrap();
        assert_eq!(actor.metadata.as_ref().unwrap().atespace, "test");
        assert_eq!(actor.actor_template.as_ref().unwrap().name, "runtime");
        assert_eq!(actor.source_tag.as_ref().unwrap().name, "code-v1");
        actor.metadata.as_mut().unwrap().uid = "allocated-actor-uid".into();
        let encoded = actor.encode_to_vec();
        let mut frame = BytesMut::with_capacity(encoded.len() + 5);
        frame.put_u8(0);
        frame.put_u32(encoded.len().try_into().unwrap());
        frame.extend_from_slice(&encoded);
        Ok::<_, Infallible>(
            Response::builder()
                .header("content-type", "application/grpc")
                .header("grpc-status", "0")
                .body(Full::<Bytes>::new(frame.freeze()))
                .unwrap(),
        )
    });
    let mut client = ControlClient::new(transport);
    let actor = client
        .create_actor(CreateActorRequest {
            actor: Some(Actor {
                metadata: Some(ResourceMetadata {
                    atespace: "test".into(),
                    name: "actor".into(),
                    ..Default::default()
                }),
                actor_template: Some(ObjectRef {
                    atespace: "test".into(),
                    name: "runtime".into(),
                }),
                source_tag: Some(ObjectRef {
                    atespace: "test".into(),
                    name: "code-v1".into(),
                }),
                ..Default::default()
            }),
        })
        .await?
        .into_inner();
    assert_eq!(actor.metadata.unwrap().uid, "allocated-actor-uid");
    Ok(())
}
