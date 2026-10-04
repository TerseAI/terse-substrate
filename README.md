# terse-substrate

Rust client for [Agent Substrate](https://github.com/agent-substrate/substrate), maintained by TerseAI. This is an independent package, not an official Google SDK.

The crate contains the upstream `ateapi` message types, generated `ControlClient`, and an authenticated `Client` with TLS, rotating credentials, template readiness, and paginated actor listing. It also provides snapshot compatibility checks and Substrate ingress addressing. It adds no intermediary service.

## Use

```toml
[dependencies]
terse-substrate = { git = "https://github.com/TerseAI/terse-substrate", tag = "v0.2.0" }
```

For deployment reproducibility, pin `rev` to the release commit and commit your `Cargo.lock`. This crate is distributed through Git; it has not been published to crates.io.

```rust,no_run
use std::sync::Arc;
use terse_substrate::{Client, ConnectionOptions, FileCredentials};

async fn connect() -> anyhow::Result<Client> {
    Client::connect(
        ConnectionOptions::new("https://api.ate-system.svc", "/var/run/substrate/ca.crt"),
        Arc::new(FileCredentials::new("/var/run/substrate/token")),
    ).await
}
```

Supply the endpoint, trust bundle, and credentials from your installation. `FileCredentials` rereads the token for each RPC so projected Kubernetes tokens can rotate. Implement `Credentials` to inject another token source. Connection, request, and template-readiness timeouts are configurable through `ConnectionOptions`. `Client::new` accepts an existing gRPC transport.

`ensure_atespace` accepts an existing atespace. `ensure_template` accepts an existing template and waits for its golden snapshot; it does not reconcile a changed template specification. Use distinct template identities when configuration changes. `list_actors` follows all response pages.

`golden_tag` extracts a prepared template's snapshot reference; `validate_tag` checks that a tag is ready and belongs to a specified template UID. `target_actor_header` builds the `ate-target-actor` HTTP header value from an `ObjectRef`, validating Substrate's resource-name rules. It does not change your request URL or path.

Other operations preserve upstream failures; callers can inspect `tonic::Status` through `anyhow::Error::downcast_ref`. Calls are not automatically retried. Deletion accepts the full protobuf request, including UID/version preconditions and `any_state`. Kubernetes secrets, customer code, application assignment, snapshot retention, and cleanup policy belong to the consumer. The generated `control_client::ControlClient` remains available for the complete upstream API.

## Compatibility and provenance

Release `0.2.0` uses the schema shipped with GKE Agent Substrate `v0.2.0-gke.0`, upstream commit `23863bea16cb14df8a34deb635346d40cac38785`. It uses `tonic` / `prost` 0.14 and requires Rust 1.89 or newer.

[`upstream.json`](upstream.json) records the source revision and SHA-256 checksums. [`proto/ateapi.proto`](proto/ateapi.proto) is copied without modification. Rust bindings are generated during the build using a bundled `protoc`; no system `protoc` installation or upstream network fetch is required during generation. Generated Rust files remain in Cargo's build directory.

Use bindings matched to your deployed Substrate version. Crate versions are independent of Substrate versions.

## Update the bindings

Verify the checked-in files against the pinned upstream source:

```sh
python3 scripts/sync-upstream.py --check
```

To move to a reviewed upstream commit and its corresponding runtime release:

```sh
python3 scripts/sync-upstream.py --revision <40-character-commit-sha> --release <runtime-release>
cargo test --locked
```

Review the schema and checksum changes, update this compatibility section, increment the crate version, and create a matching Git release tag. Do not edit the vendored schema by hand. The update script fetches public upstream source; ordinary crate builds use the committed copy.

## Development

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Tests exercise generated and authenticated clients against an in-process gRPC transport, including credential rotation, readiness deadlines, pagination, snapshot compatibility, and ingress addressing. They require no cloud account or cluster. CI also verifies upstream provenance and tests Rust 1.89 and stable.

## License

Apache-2.0. The vendored schema retains its upstream copyright header; see [LICENSE](LICENSE) and [NOTICE](NOTICE).
