use terse_substrate::{
    ActorTemplate, ActorTemplateStatus, ExternalSnapshot, GoldenSnapshotStatus, ObjectRef,
    TARGET_ACTOR_HEADER, Tag, TagStatus, golden_tag, target_actor_header, validate_tag,
};

#[test]
fn ingress_address_matches_the_substrate_router_contract() -> anyhow::Result<()> {
    assert_eq!(TARGET_ACTOR_HEADER, "ate-target-actor");
    assert_eq!(target_actor_header(&target("actor-1"))?, "team-a/actor-1");
    assert!(target_actor_header(&target(&"a".repeat(63))).is_ok());
    for name in [
        "",
        "a/b",
        "UPPER",
        "actor.name",
        "-actor",
        "actor-",
        "a\r\nb",
        &"a".repeat(64),
    ] {
        assert!(target_actor_header(&target(name)).is_err(), "{name:?}");
        assert!(
            target_actor_header(&ObjectRef {
                atespace: name.into(),
                name: "actor".into()
            })
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn snapshots_must_be_ready_and_belong_to_the_requested_template() -> anyhow::Result<()> {
    assert!(golden_tag(&ActorTemplate::default()).is_err());
    let template = ActorTemplate {
        status: Some(ActorTemplateStatus {
            golden_snapshot_status: Some(GoldenSnapshotStatus {
                golden_tag: Some(target("golden")),
                ..Default::default()
            }),
        }),
        ..Default::default()
    };
    assert_eq!(golden_tag(&template)?, target("golden"));
    assert!(validate_tag(&Tag::default(), "template-uid").is_err());
    let mut tag = Tag {
        status: Some(TagStatus {
            actor_template_uid: "template-uid".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(validate_tag(&tag, "template-uid").is_err());
    tag.status.as_mut().unwrap().snapshot = Some(ExternalSnapshot::default());
    validate_tag(&tag, "template-uid")?;
    assert!(validate_tag(&tag, "different-template").is_err());
    assert!(validate_tag(&tag, "").is_err());
    Ok(())
}

fn target(name: &str) -> ObjectRef {
    ObjectRef {
        atespace: "team-a".into(),
        name: name.into(),
    }
}
