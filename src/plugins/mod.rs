//! 可插拔接口层模块
//! 
//! 实现统一的Plugin trait体系，支持零成本抽象的插件系统

pub mod core;
pub mod lifecycle;
pub mod communication;
pub mod registry;
pub mod metadata;

// 重新导出核心组件
pub use core::*;
pub use lifecycle::*;
pub use communication::*;
pub use registry::*;
pub use metadata::*;