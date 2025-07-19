//! MosesQuant 错误处理系统
//! 
//! 统一的错误类型和错误处理机制
//! 基于分层架构的错误分类和处理策略

use thiserror::Error;
use crate::types::{PluginId, OrderId, StrategyId, OrderStatus};

/// 框架统一错误类型
#[derive(Error, Debug)]
pub enum MosesQuantError {
    // === 系统级错误 ===
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("YAML serialization error: {0}")]
    YamlSerialization(#[from] serde_yaml::Error),
    
    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
    
    #[error("Decimal parsing error: {0}")]
    DecimalParsing(#[from] rust_decimal::Error),
    
    // === 插件系统错误 ===
    #[error("Plugin not found: {plugin_id}")]
    PluginNotFound { plugin_id: PluginId },
    
    #[error("Plugin already registered: {plugin_id}")]
    PluginAlreadyRegistered { plugin_id: PluginId },
    
    #[error("Plugin initialization failed: {plugin_id} - {reason}")]
    PluginInitializationFailed { plugin_id: PluginId, reason: String },
    
    #[error("Plugin load error: {reason}")]
    PluginLoadError { reason: String },

    #[error("Plugin load error: {message}")]
    PluginLoad { message: String },

    #[error("Dependency injection error: {message}")]
    DependencyInjection { message: String },

    #[error("Version management error: {message}")]
    Version { message: String },
    
    #[error("Plugin dependency not found: {dependency}")]
    DependencyNotFound { dependency: String },
    
    #[error("Circular dependency detected")]
    CircularDependency,
    
    #[error("Plugin type mismatch")]
    TypeMismatch,
    
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: OrderStatus, to: OrderStatus },
    
    // === 事件系统错误 ===
    #[error("Event bus error: {message}")]
    EventBus { message: String },
    
    #[error("Event handler error: {message}")]
    EventHandler { message: String },
    
    #[error("Event queue full")]
    EventQueueFull,
    
    // === 数据相关错误 ===
    #[error("Data source error: {message}")]
    DataSource { message: String },
    
    #[error("Data validation error: {message}")]
    DataValidation { message: String },
    
    #[error("Market data error: {message}")]
    MarketData { message: String },
    
    #[error("Price not found for symbol: {symbol}")]
    PriceNotFound { symbol: String },
    
    // === 交易相关错误 ===
    #[error("Order error: {message}")]
    Order { message: String },
    
    #[error("Order not found: {order_id}")]
    OrderNotFound { order_id: OrderId },
    
    #[error("Trade execution error: {message}")]
    TradeExecution { message: String },
    
    #[error("Position error: {message}")]
    Position { message: String },
    
    #[error("Gateway error: {message}")]
    Gateway { message: String },
    
    // === 策略相关错误 ===
    #[error("Strategy error: {strategy_id} - {message}")]
    Strategy { strategy_id: StrategyId, message: String },
    
    #[error("Strategy not found: {strategy_id}")]
    StrategyNotFound { strategy_id: StrategyId },
    
    #[error("Alpha model error: {message}")]
    AlphaModel { message: String },
    
    #[error("Portfolio construction error: {message}")]
    PortfolioConstruction { message: String },
    
    // === 风险管理错误 ===
    #[error("Risk check failed: {violated_limits:?}")]
    RiskCheckFailed { violated_limits: Vec<String> },
    
    #[error("Risk limit exceeded: {limit_type} - {current_value} > {limit_value}")]
    RiskLimitExceeded { 
        limit_type: String,
        current_value: f64,
        limit_value: f64,
    },
    
    #[error("Risk model error: {message}")]
    RiskModel { message: String },
    
    // === 配置相关错误 ===
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Configuration not found: {key}")]
    ConfigNotFound { key: String },
    
    #[error("Configuration validation error: {message}")]
    ConfigValidation { message: String },
    
    // === 网络相关错误 ===
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("WebSocket error: {message}")]
    WebSocket { message: String },
    
    // === 热更新相关错误 ===
    #[error("Hot update error: {message}")]
    HotUpdate { message: String },
    
    #[error("Version compatibility error: {current} -> {target}")]
    VersionCompatibility { current: String, target: String },
    
    #[error("State migration failed: {errors:?}")]
    StateMigration { errors: Vec<String> },
    
    #[error("Rollback failed: {reason}")]
    RollbackFailed { reason: String },
    
    // === 通用错误 ===
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Not supported: {message}")]
    NotSupported { message: String },
    
    #[error("Operation timeout: {operation}")]
    Timeout { operation: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl MosesQuantError {
    // === 便捷构造函数 ===
    
    /// 创建插件相关错误
    pub fn plugin_not_found(plugin_id: impl Into<PluginId>) -> Self {
        Self::PluginNotFound { plugin_id: plugin_id.into() }
    }
    
    pub fn plugin_init_failed(plugin_id: impl Into<PluginId>, reason: impl Into<String>) -> Self {
        Self::PluginInitializationFailed { 
            plugin_id: plugin_id.into(), 
            reason: reason.into() 
        }
    }
    
    /// 创建数据相关错误
    pub fn data_source(message: impl Into<String>) -> Self {
        Self::DataSource { message: message.into() }
    }
    
    pub fn market_data(message: impl Into<String>) -> Self {
        Self::MarketData { message: message.into() }
    }
    
    /// 创建交易相关错误
    pub fn order_error(message: impl Into<String>) -> Self {
        Self::Order { message: message.into() }
    }
    
    pub fn trade_execution(message: impl Into<String>) -> Self {
        Self::TradeExecution { message: message.into() }
    }
    
    /// 创建策略相关错误
    pub fn strategy_error(strategy_id: impl Into<StrategyId>, message: impl Into<String>) -> Self {
        Self::Strategy { 
            strategy_id: strategy_id.into(), 
            message: message.into() 
        }
    }
    
    /// 创建风险相关错误
    pub fn risk_check_failed(violated_limits: Vec<String>) -> Self {
        Self::RiskCheckFailed { violated_limits }
    }
    
    pub fn risk_limit_exceeded(
        limit_type: impl Into<String>, 
        current_value: f64, 
        limit_value: f64
    ) -> Self {
        Self::RiskLimitExceeded {
            limit_type: limit_type.into(),
            current_value,
            limit_value,
        }
    }
    
    /// 创建配置相关错误
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    pub fn config_not_found(key: impl Into<String>) -> Self {
        Self::ConfigNotFound { key: key.into() }
    }
    
    /// 创建网络相关错误
    pub fn network_error(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// 创建验证相关错误
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::Validation { message: message.into() }
    }
    
    /// 创建内部错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }
    
    // === 错误分类方法 ===
    
    /// 是否为可恢复错误
    pub fn is_recoverable(&self) -> bool {
        match self {
            // 系统级错误通常不可恢复
            Self::Io(_) | Self::Time(_) => false,
            
            // 插件错误可能可恢复
            Self::PluginNotFound { .. } | Self::PluginInitializationFailed { .. } => true,
            
            // 数据错误通常可恢复
            Self::DataSource { .. } | Self::MarketData { .. } => true,
            
            // 交易错误部分可恢复
            Self::OrderNotFound { .. } => false,
            Self::TradeExecution { .. } => true,
            
            // 网络错误通常可恢复
            Self::Network { .. } | Self::Http(_) | Self::WebSocket { .. } => true,
            
            // 配置错误通常不可恢复
            Self::Config { .. } | Self::ConfigValidation { .. } => false,
            
            // 风险错误通常可恢复
            Self::RiskCheckFailed { .. } | Self::RiskLimitExceeded { .. } => true,
            
            // 其他错误根据具体情况
            _ => false,
        }
    }
    
    /// 获取错误严重级别
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // 关键错误
            Self::CircularDependency | Self::RollbackFailed { .. } => ErrorSeverity::Critical,
            
            // 高级错误
            Self::RiskCheckFailed { .. } | Self::RiskLimitExceeded { .. } => ErrorSeverity::High,
            
            // 中级错误
            Self::PluginInitializationFailed { .. } | Self::TradeExecution { .. } => ErrorSeverity::Medium,
            
            // 低级错误
            Self::DataValidation { .. } | Self::ConfigNotFound { .. } => ErrorSeverity::Low,
            
            // 默认为中级
            _ => ErrorSeverity::Medium,
        }
    }
    
    /// 获取错误类别
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Io(_) | Self::Time(_) | Self::Serialization(_) | Self::YamlSerialization(_) => {
                ErrorCategory::System
            }
            
            Self::PluginNotFound { .. } | Self::PluginAlreadyRegistered { .. } | 
            Self::PluginInitializationFailed { .. } | Self::PluginLoadError { .. } => {
                ErrorCategory::Plugin
            }
            
            Self::DataSource { .. } | Self::MarketData { .. } | Self::DataValidation { .. } => {
                ErrorCategory::Data
            }
            
            Self::Order { .. } | Self::TradeExecution { .. } | Self::Position { .. } => {
                ErrorCategory::Trading
            }
            
            Self::Strategy { .. } | Self::AlphaModel { .. } | Self::PortfolioConstruction { .. } => {
                ErrorCategory::Strategy
            }
            
            Self::RiskCheckFailed { .. } | Self::RiskLimitExceeded { .. } | Self::RiskModel { .. } => {
                ErrorCategory::Risk
            }
            
            Self::Network { .. } | Self::Http(_) | Self::WebSocket { .. } => {
                ErrorCategory::Network
            }
            
            _ => ErrorCategory::General,
        }
    }
}

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    System,
    Plugin,
    Data,
    Trading,
    Strategy,
    Risk,
    Network,
    General,
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, MosesQuantError>;

// === 向后兼容性 ===

/// 兼容旧版本的错误类型别名
pub type CzscError = MosesQuantError;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let error = MosesQuantError::data_source("Test data error");
        assert!(matches!(error, MosesQuantError::DataSource { .. }));
        assert_eq!(error.to_string(), "Data source error: Test data error");
    }
    
    #[test]
    fn test_plugin_errors() {
        let plugin_id = "test_plugin".to_string();
        let error = MosesQuantError::plugin_not_found(&plugin_id);
        assert!(matches!(error, MosesQuantError::PluginNotFound { .. }));
        
        let init_error = MosesQuantError::plugin_init_failed(&plugin_id, "Init failed");
        assert!(matches!(init_error, MosesQuantError::PluginInitializationFailed { .. }));
    }
    
    #[test]
    fn test_error_classification() {
        let network_error = MosesQuantError::network_error("Connection failed");
        assert!(network_error.is_recoverable());
        assert_eq!(network_error.category(), ErrorCategory::Network);
        assert_eq!(network_error.severity(), ErrorSeverity::Medium);
        
        let risk_error = MosesQuantError::risk_limit_exceeded("position", 1000.0, 500.0);
        assert!(risk_error.is_recoverable());
        assert_eq!(risk_error.category(), ErrorCategory::Risk);
        assert_eq!(risk_error.severity(), ErrorSeverity::High);
        
        let critical_error = MosesQuantError::CircularDependency;
        assert!(!critical_error.is_recoverable());
        assert_eq!(critical_error.severity(), ErrorSeverity::Critical);
    }
    
    #[test]
    fn test_error_from_external() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let moses_error = MosesQuantError::from(io_error);
        assert!(matches!(moses_error, MosesQuantError::Io(_)));
        assert_eq!(moses_error.category(), ErrorCategory::System);
    }
    
    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        let failure: Result<i32> = Err(MosesQuantError::internal_error("Test error"));
        
        assert!(success.is_ok());
        assert!(failure.is_err());
    }
    
    #[test]
    fn test_backward_compatibility() {
        let _old_error: CzscError = MosesQuantError::internal_error("Compatible error");
        // 编译通过即说明类型别名有效
    }
}