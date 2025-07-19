//! 可插拔接口层模块
//! 
//! 实现统一的Plugin trait体系，支持零成本抽象的插件系统

pub mod core;
pub mod lifecycle;
pub mod communication;
pub mod registry;
pub mod metadata;
pub mod dependency_injection;
pub mod dynamic_loader;
pub mod version_management;
pub mod quality_assurance;
pub mod dev_toolkit;
pub mod cicd_integration;
pub mod marketplace;

// 重新导出核心组件
pub use core::*;
pub use lifecycle::*;
pub use communication::*;
pub use registry::*;
pub use metadata::*;
pub use dependency_injection::*;
pub use dynamic_loader::*;
pub use version_management::*;
pub use quality_assurance::*;
pub use dev_toolkit::*;
pub use cicd_integration::*;
pub use marketplace::*;