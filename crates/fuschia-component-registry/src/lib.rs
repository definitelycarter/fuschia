mod error;
mod fs_registry;
mod in_memory;
mod manifest;
mod registry;

pub use error::RegistryError;
pub use fs_registry::FsComponentRegistry;
pub use in_memory::InMemoryComponentRegistry;
pub use fuschia_config::TriggerType;
pub use manifest::{ComponentCapabilities, ComponentManifest, TaskExport, TriggerExport};
pub use registry::{ComponentRegistry, InstalledComponent};
