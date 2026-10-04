# terse-substrate

Rust gRPC client bindings for [Agent Substrate](https://github.com/agent-substrate/substrate), maintained by TerseAI. This is an independent package, not an official Google SDK.

The crate contains the upstream `ateapi` message types and generated `ControlClient`. Authentication, TLS, retries, and sandbox lifecycle policy stay in the consuming application. It adds no intermediary service.

## Use

```toml
[dependencies]
terse-substrate = { git = "https://github.com/TerseAI/terse-substrate", tag = "v0.1.0" }
```

For deployment reproducibility, pin `rev` to the release commit and commit your `Cargo.lock`. This crate is distributed through Git; it has not been published to crates.io.

```rust,no_run
use terse_substrate::{CreateActorRequest, control_client::ControlClient};

async fn create_actor(
    channel: tonic::transport::Channel,
    request: CreateActorRequest,
) -> Result<terse_substrate::Actor, tonic::Status> {
    let mut client = ControlClient::new(channel);
    Ok(client.create_actor(request).await?.into_inner())
}
```

Configure the channel's TLS roots and attach the credentials required by your Substrate installation. The example above shows the binding, not a complete authenticated connection. Consumers configuring TLS should enable the appropriate features on their own `tonic` dependency.

## Compatibility and provenance

Release `0.1.0` uses the schema shipped with GKE Agent Substrate `v0.2.0-gke.0`, upstream commit `23863bea16cb14df8a34deb635346d40cac38785`. It uses `tonic` / `prost` 0.14 and requires Rust 1.89 or newer.

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

The integration test exercises a generated client call against an in-process mock gRPC transport. It requires no cloud account or cluster. CI also verifies upstream provenance and tests Rust 1.89 and stable.

## License

Apache-2.0. The vendored schema retains its upstream copyright header; see [LICENSE](LICENSE) and [NOTICE](NOTICE).
