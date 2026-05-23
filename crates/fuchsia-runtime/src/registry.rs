use fuchsia_actor::{Actor, ActorError};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait ActorFactory: Send + Sync {
  fn instantiate(&self, config: Value) -> Result<Arc<dyn Actor>, ActorError>;
}

struct ClosureFactory<A, Cfg, F> {
  ctor: F,
  _marker: PhantomData<fn(Cfg) -> A>,
}

impl<A, Cfg, F> ActorFactory for ClosureFactory<A, Cfg, F>
where
  A: Actor + 'static,
  Cfg: DeserializeOwned + 'static,
  F: Fn(Cfg) -> A + Send + Sync + 'static,
{
  fn instantiate(&self, config: Value) -> Result<Arc<dyn Actor>, ActorError> {
    let cfg: Cfg = serde_json::from_value(config)?;
    Ok(Arc::new((self.ctor)(cfg)))
  }
}

#[derive(Default)]
pub struct ActorRegistry {
  factories: HashMap<String, Arc<dyn ActorFactory>>,
}

impl ActorRegistry {
  pub fn new() -> Self {
    Self {
      factories: HashMap::new(),
    }
  }

  pub fn register<A, Cfg, F>(&mut self, name: impl Into<String>, ctor: F)
  where
    A: Actor + 'static,
    Cfg: DeserializeOwned + 'static,
    F: Fn(Cfg) -> A + Send + Sync + 'static,
  {
    let name = name.into();
    tracing::debug!(actor = %name, "registry.register");
    let factory = ClosureFactory::<A, Cfg, F> {
      ctor,
      _marker: PhantomData,
    };
    self.factories.insert(name, Arc::new(factory));
  }

  pub fn instantiate(&self, name: &str, config: Value) -> Result<Arc<dyn Actor>, ActorError> {
    tracing::trace!(actor = %name, "registry.instantiate");
    let factory = self
      .factories
      .get(name)
      .ok_or_else(|| ActorError::UnknownActor(name.into()))?;
    factory.instantiate(config)
  }
}
