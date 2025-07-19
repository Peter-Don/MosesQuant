//! 核心基础层模块
//! 
//! 提供框架的核心基础设施，包括事件总线、内存管理、类型系统等

pub mod simple_event_bus;
pub mod simple_memory;
pub mod scheduling;

// 重新导出核心组件
pub use simple_event_bus::*;
pub use simple_memory::*;
pub use scheduling::*;

// 类型别名用于向后兼容
pub type EventBus = SimpleEventBus;