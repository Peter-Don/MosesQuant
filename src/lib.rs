//! MosesQuant - 高性能量化交易框架
//! 
//! 基于四层架构设计，实现零成本抽象的高性能量化交易系统
//! 
//! # 架构分层
//! 
//! - **用户实现层**: 自定义策略、数据源、网关等插件实现
//! - **可插拔接口层**: 统一的插件接口定义
//! - **框架服务层**: 策略引擎、数据管理器等核心服务
//! - **核心基础层**: 类型系统、事件总线、内存管理等基础设施
//! 
//! # 特性
//! 
//! - **零成本抽象**: 编译时优化，运行时无额外开销
//! - **类型安全**: 高精度Decimal类型，避免浮点误差
//! - **可插拔架构**: 精准的可插拔设计，避免过度工程
//! - **异步优先**: 基于Tokio的高性能异步处理
//! - **内存安全**: Rust所有权系统保证的安全性

pub mod types;
pub mod error;
pub mod core;
pub mod plugins;
pub mod services;
pub mod data;
pub mod strategy;
pub mod config;
pub mod runner;
pub mod examples;

// 重新导出核心类型
pub use types::*;
pub use error::*;
pub use core::*;
pub use plugins::*;
pub use services::*;

/// 框架信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FRAMEWORK_NAME: &str = "MosesQuant";

/// 结果类型别名 - 兼容旧版本
pub type Result<T> = std::result::Result<T, MosesQuantError>;

/// 快速启动函数
pub async fn initialize() -> Result<()> {
    // 初始化日志系统
    tracing_subscriber::fmt::init();
    
    tracing::info!("🚀 Initializing {} v{}", FRAMEWORK_NAME, VERSION);
    tracing::info!("🏗️  Architecture: Four-layer pluggable design");
    tracing::info!("⚡ Features: Zero-cost abstractions, Type safety, Memory safety");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_framework_info() {
        assert_eq!(FRAMEWORK_NAME, "MosesQuant");
        assert!(!VERSION.is_empty());
    }
    
    #[tokio::test]
    async fn test_initialize() {
        let result = initialize().await;
        assert!(result.is_ok());
    }
}