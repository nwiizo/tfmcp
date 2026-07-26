mod error;
mod module_versions;
mod modules;
mod provider;

pub use error::RegistryError;
pub use module_versions::{
    ModuleVersionDetail, ModuleVersionInfo, ModuleVersionProvider, ModuleVersionRoot,
    ModuleVersionSubmodule, ModuleVersionsResponse,
};
pub use modules::{
    ModuleDependency, ModuleDetails, ModuleInfo, ModuleInput, ModuleOutput,
    ModuleProviderDependency, ModuleResource, ModuleRoot, ModuleSearchMeta, ModuleSearchResponse,
    ModuleSubmodule,
};
pub use provider::{
    DocIdResult, ProviderDocsResponse, ProviderInfo, ProviderVersions, RegistrySearchResponse,
    VersionInfo,
};
