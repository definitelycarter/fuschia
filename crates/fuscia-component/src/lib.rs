mod error;
mod fs_registry;
mod manifest;
mod registry;

pub use error::RegistryError;
pub use fs_registry::FsComponentRegistry;
pub use manifest::{
  ComponentCapabilities, ComponentManifest, TaskExport, TriggerExport, TriggerType,
};
pub use registry::{ComponentRegistry, InstalledComponent};
