use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result, ensure};
use tonic::{
    Code, Request,
    codegen::{Body, Bytes, StdError},
    transport::{Certificate, Channel, ClientTlsConfig, Endpoint},
};

use crate::{Credentials, control_client::ControlClient, *};

#[derive(Clone, Copy, Debug)]
pub struct Readiness {
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl Default for Readiness {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            poll_interval: Duration::from_millis(100),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionOptions {
    pub endpoint: String,
    pub trust_bundle: PathBuf,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub readiness: Readiness,
}

impl ConnectionOptions {
    pub fn new(endpoint: impl Into<String>, trust_bundle: impl Into<PathBuf>) -> Self {
        Self {
            endpoint: endpoint.into(),
            trust_bundle: trust_bundle.into(),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(120),
            readiness: Readiness::default(),
        }
    }
}

#[derive(Clone)]
pub struct Client<T = Channel> {
    inner: ControlClient<T>,
    credentials: Arc<dyn Credentials>,
    readiness: Readiness,
}

impl Client<Channel> {
    pub async fn connect(
        options: ConnectionOptions,
        credentials: Arc<dyn Credentials>,
    ) -> Result<Self> {
        let roots = tokio::fs::read(&options.trust_bundle)
            .await
            .context("read Substrate trust bundle")?;
        let endpoint = Endpoint::from_shared(options.endpoint)?;
        ensure!(
            endpoint.uri().scheme_str() == Some("https"),
            "Substrate connection requires HTTPS"
        );
        let channel = endpoint
            .tls_config(ClientTlsConfig::new().ca_certificate(Certificate::from_pem(roots)))?
            .connect_timeout(options.connect_timeout)
            .timeout(options.request_timeout)
            .connect()
            .await?;
        Ok(Self::new(channel, credentials, options.readiness))
    }
}

impl<T> Client<T>
where
    T: tonic::client::GrpcService<tonic::body::Body> + Clone,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    pub fn new(transport: T, credentials: Arc<dyn Credentials>, readiness: Readiness) -> Self {
        Self {
            inner: ControlClient::new(transport),
            credentials,
            readiness,
        }
    }

    /// Creates the atespace if it does not already exist.
    pub async fn ensure_atespace(&self, atespace: Atespace) -> Result<()> {
        let request = self
            .request(CreateAtespaceRequest {
                atespace: Some(atespace),
            })
            .await?;
        match self.inner.clone().create_atespace(request).await {
            Ok(_) => Ok(()),
            Err(status) if status.code() == Code::AlreadyExists => Ok(()),
            Err(status) => Err(status.into()),
        }
    }

    /// Creates an immutable template if absent and waits for its golden snapshot.
    pub async fn ensure_template(&self, template: ActorTemplate) -> Result<ActorTemplate> {
        let meta = template
            .metadata
            .as_ref()
            .context("template identity missing")?;
        let target = ObjectRef {
            atespace: meta.atespace.clone(),
            name: meta.name.clone(),
        };
        let request = self
            .request(CreateActorTemplateRequest {
                actor_template: Some(template),
            })
            .await?;
        match self.inner.clone().create_actor_template(request).await {
            Ok(_) => {}
            Err(status) if status.code() == Code::AlreadyExists => {}
            Err(status) => return Err(status.into()),
        }
        self.wait_template(target).await
    }

    pub async fn create_actor(&self, actor: Actor) -> Result<Actor> {
        let request = self
            .request(CreateActorRequest { actor: Some(actor) })
            .await?;
        Ok(self.inner.clone().create_actor(request).await?.into_inner())
    }

    pub async fn resume_actor(&self, actor: ObjectRef) -> Result<ResumeActorResponse> {
        let request = self
            .request(ResumeActorRequest { actor: Some(actor) })
            .await?;
        Ok(self.inner.clone().resume_actor(request).await?.into_inner())
    }

    pub async fn suspend_actor(&self, actor: ObjectRef) -> Result<SuspendActorResponse> {
        let request = self
            .request(SuspendActorRequest { actor: Some(actor) })
            .await?;
        Ok(self
            .inner
            .clone()
            .suspend_actor(request)
            .await?
            .into_inner())
    }

    pub async fn delete_actor(&self, request: DeleteActorRequest) -> Result<Actor> {
        let request = self.request(request).await?;
        Ok(self.inner.clone().delete_actor(request).await?.into_inner())
    }

    pub async fn create_actor_egress_policy(
        &self,
        request: CreateActorEgressPolicyRequest,
    ) -> Result<EgressPolicy> {
        let request = self.request(request).await?;
        Ok(self
            .inner
            .clone()
            .create_actor_egress_policy(request)
            .await?
            .into_inner())
    }

    pub async fn create_tag(&self, tag: Tag) -> Result<Tag> {
        let request = self.request(CreateTagRequest { tag: Some(tag) }).await?;
        Ok(self.inner.clone().create_tag(request).await?.into_inner())
    }

    pub async fn get_tag(&self, tag: ObjectRef) -> Result<Tag> {
        let request = self.request(GetTagRequest { tag: Some(tag) }).await?;
        Ok(self.inner.clone().get_tag(request).await?.into_inner())
    }

    pub async fn delete_tag(&self, request: DeleteTagRequest) -> Result<Tag> {
        let request = self.request(request).await?;
        Ok(self.inner.clone().delete_tag(request).await?.into_inner())
    }

    pub async fn list_actors(&self, atespace: &str) -> Result<Vec<Actor>> {
        let mut actors = Vec::new();
        let mut page_token = String::new();
        loop {
            let request = self
                .request(ListActorsRequest {
                    atespace: atespace.into(),
                    page_size: 1000,
                    page_token,
                })
                .await?;
            let page = self.inner.clone().list_actors(request).await?.into_inner();
            actors.extend(page.actors);
            page_token = page.next_page_token;
            if page_token.is_empty() {
                return Ok(actors);
            }
        }
    }

    pub async fn list_workers(&self) -> Result<Vec<Worker>> {
        let mut workers = Vec::new();
        let mut page_token = String::new();
        loop {
            let request = self
                .request(ListWorkersRequest {
                    page_size: 1000,
                    page_token,
                })
                .await?;
            let page = self.inner.clone().list_workers(request).await?.into_inner();
            workers.extend(page.workers);
            page_token = page.next_page_token;
            if page_token.is_empty() {
                return Ok(workers);
            }
        }
    }

    pub async fn wait_template(&self, target: ObjectRef) -> Result<ActorTemplate> {
        tokio::time::timeout(self.readiness.timeout, async {
            loop {
                let request = self
                    .request(GetActorTemplateRequest {
                        actor_template: Some(target.clone()),
                    })
                    .await?;
                let template = self
                    .inner
                    .clone()
                    .get_actor_template(request)
                    .await?
                    .into_inner();
                if let Some(golden) = template
                    .status
                    .as_ref()
                    .and_then(|s| s.golden_snapshot_status.as_ref())
                {
                    ensure!(
                        golden.error_message.is_empty(),
                        "golden snapshot failed: {}",
                        golden.error_message
                    );
                    if golden.golden_tag.is_some() {
                        return Ok(template);
                    }
                }
                tokio::time::sleep(self.readiness.poll_interval).await;
            }
        })
        .await
        .context("timed out preparing Substrate template")?
    }

    async fn request<M>(&self, message: M) -> Result<Request<M>> {
        let token = self.credentials.token().await?;
        let token = token.trim();
        ensure!(!token.is_empty(), "Substrate bearer token is empty");
        let mut authorization =
            format!("Bearer {token}").parse::<tonic::metadata::MetadataValue<_>>()?;
        authorization.set_sensitive(true);
        let mut request = Request::new(message);
        request
            .metadata_mut()
            .insert("authorization", authorization);
        Ok(request)
    }
}
