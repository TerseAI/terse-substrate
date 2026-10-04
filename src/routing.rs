use crate::ObjectRef;
use anyhow::{Result, ensure};
use http::HeaderValue;

pub const TARGET_ACTOR_HEADER: &str = "ate-target-actor";

pub fn target_actor_header(actor: &ObjectRef) -> Result<HeaderValue> {
    ensure!(
        valid_resource_name(&actor.atespace) && valid_resource_name(&actor.name),
        "invalid Substrate actor identity"
    );
    Ok(format!("{}/{}", actor.atespace, actor.name).parse()?)
}

/// Substrate resource names follow the DNS-1123 label rules (up to 63 bytes).
pub fn valid_resource_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}
