use async_trait::async_trait;
use crate::{Router, RouterError, RoutingContext};

/// Chains multiple routers with fallback logic.
pub struct FallbackRouter {
    routers: Vec<Box<dyn Router>>,
    fallback_provider: String,
}

impl FallbackRouter {
    pub fn new(routers: Vec<Box<dyn Router>>, fallback_provider: String) -> Self {
        Self { routers, fallback_provider }
    }
}

#[async_trait]
impl Router for FallbackRouter {
    async fn select_provider(
        &self,
        context: &RoutingContext,
        available: &[String],
    ) -> Result<String, RouterError> {
        for router in &self.routers {
            match router.select_provider(context, available).await {
                Ok(provider) => return Ok(provider),
                Err(_) => continue,
            }
        }
        Ok(self.fallback_provider.clone())
    }

    async fn should_handoff(
        &self,
        current: &str,
        context: &RoutingContext,
    ) -> Result<Option<String>, RouterError> {
        for router in &self.routers {
            match router.should_handoff(current, context).await {
                Ok(Some(provider)) => return Ok(Some(provider)),
                Ok(None) => continue,
                Err(_) => continue,
            }
        }
        Ok(None)
    }
}
