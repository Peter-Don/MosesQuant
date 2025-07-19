//! 框架服务层模块
//! 
//! 提供基于可插拔架构的核心服务，包括策略引擎、数据管理器、风险管理器等

pub mod strategy_engine;
pub mod data_manager;
pub mod risk_manager;
pub mod order_manager;
pub mod config_manager;
pub mod monitoring_engine;

// 重新导出核心服务
pub use strategy_engine::*;
pub use data_manager::*;
pub use risk_manager::*;
pub use order_manager::*;
pub use config_manager::*;
pub use monitoring_engine::*;