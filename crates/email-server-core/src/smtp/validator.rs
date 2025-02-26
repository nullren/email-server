use async_trait::async_trait;
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    TokioAsyncResolver,
};

#[async_trait]
pub trait HeloValidator: Send + Sync + std::fmt::Debug {
    async fn valid(&self, domain: &str) -> bool;
}

#[derive(Debug, Default)]
pub struct NoopValidator;
#[async_trait]
impl HeloValidator for NoopValidator {
    async fn valid(&self, _domain: &str) -> bool {
        true
    }
}

#[derive(Debug, Default)]
pub struct DomainNameValidator;
#[async_trait]
impl HeloValidator for DomainNameValidator {
    async fn valid(&self, domain: &str) -> bool {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        // valid domain
        if let Ok(response) = resolver.lookup_ip(domain).await {
            if response.iter().next().is_some() {
                return true;
            }
        }
        false
    }
}
