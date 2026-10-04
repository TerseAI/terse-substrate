use crate::{ActorTemplate, ObjectRef, Tag};
use anyhow::{Context, Result, ensure};

pub fn golden_tag(template: &ActorTemplate) -> Result<ObjectRef> {
    template
        .status
        .as_ref()
        .and_then(|s| s.golden_snapshot_status.as_ref())
        .and_then(|s| s.golden_tag.clone())
        .context("template golden snapshot is not ready")
}

/// Checks that a tag is ready and can seed actors of the requested template UID.
pub fn validate_tag(tag: &Tag, template_uid: &str) -> Result<()> {
    let status = tag.status.as_ref().context("snapshot tag is not ready")?;
    ensure!(status.snapshot.is_some(), "snapshot tag is not ready");
    ensure!(
        !template_uid.is_empty() && status.actor_template_uid == template_uid,
        "snapshot tag does not match actor template"
    );
    Ok(())
}
