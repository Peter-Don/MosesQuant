//! MosesQuant - 高性能量化交易框架
//! 
//! 基于WonderTrader架构理念，结合QuantConnect LEAN模块化设计
//! 实现零成本抽象的高性能量化交易系统

pub mod types;
pub mod error;
pub mod events;
pub mod data;
pub mod strategy;
pub mod indicators;  // 新增技术指标计算模块
pub mod python_ffi;  // 新增Python FFI绑定模块
pub mod plugins;     // 新增策略插件系统
pub mod config;
pub mod runner;
pub mod examples;

// 重新导出核心类型
pub use types::*;
pub use error::*;

/// 框架信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FRAMEWORK_NAME: &str = "MosesQuant";

/// 结果类型别名
pub type Result<T> = std::result::Result<T, CzscError>;

/// 快速启动函数
pub async fn initialize() -> Result<()> {
    tracing::info!("🚀 Initializing {} v{}", FRAMEWORK_NAME, VERSION);
    tracing::info!("🏗️  Architecture: WonderTrader + QuantConnect LEAN");
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