//! MosesQuant 配置管理系统
//! 
//! 支持YAML配置文件驱动的策略运行

use crate::types::*;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 框架配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    /// 框架基础设置
    pub framework: FrameworkSettings,
    /// 数据源配置
    pub data_sources: Vec<DataSourceConfig>,
    /// 策略配置
    pub strategies: Vec<StrategyConfig>,
    /// 风险管理配置
    pub risk_management: RiskManagementConfig,
    /// 执行配置
    pub execution: ExecutionConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

/// 框架基础设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkSettings {
    /// 框架名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 运行模式
    pub mode: RunMode,
    /// 时区
    pub timezone: String,
    /// 基础货币
    pub base_currency: String,
    /// 初始资金
    pub initial_capital: f64,
}

/// 运行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunMode {
    /// 回测模式
    Backtest,
    /// 实盘模拟
    PaperTrading,
    /// 实盘交易
    Live,
}

/// 数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    /// 数据源名称
    pub name: String,
    /// 数据源类型
    pub source_type: DataSourceType,
    /// 连接参数
    pub connection: ConnectionConfig,
    /// 支持的标的
    pub symbols: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 数据源类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSourceType {
    /// CSV文件
    Csv,
    /// Binance
    Binance,
    /// 自定义
    Custom(String),
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// 连接参数
    pub params: HashMap<String, serde_json::Value>,
}

/// 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略类型
    pub strategy_type: StrategyType,
    /// 标的选择器配置
    pub universe_selector: ComponentConfig,
    /// Alpha模型配置
    pub alpha_model: ComponentConfig,
    /// 投资组合构建器配置
    pub portfolio_constructor: ComponentConfig,
    /// 参数配置
    pub parameters: HashMap<String, serde_json::Value>,
    /// 是否启用
    pub enabled: bool,
}

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    /// 趋势跟踪
    Momentum,
    /// 均值回归
    MeanReversion,
    /// 套利
    Arbitrage,
    /// 自定义
    Custom(String),
}

/// 组件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// 组件类型
    pub component_type: String,
    /// 组件参数
    pub parameters: HashMap<String, serde_json::Value>,
}

/// 风险管理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementConfig {
    /// 最大单个持仓比例
    pub max_position_size: f64,
    /// 最大总持仓比例
    pub max_total_position: f64,
    /// 最大回撤限制
    pub max_drawdown: f64,
    /// 止损比例
    pub stop_loss: Option<f64>,
    /// 止盈比例
    pub take_profit: Option<f64>,
    /// 风险度量方法
    pub risk_measures: Vec<RiskMeasure>,
}

/// 风险度量方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskMeasure {
    /// VaR
    VaR { confidence: f64, horizon: i32 },
    /// 最大回撤
    MaxDrawdown,
    /// 波动率
    Volatility { window: i32 },
}

/// 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// 执行频率
    pub frequency: ExecutionFrequency,
    /// 订单类型
    pub order_type: OrderType,
    /// 滑点容忍度
    pub slippage_tolerance: f64,
    /// 最小订单大小
    pub min_order_size: f64,
    /// 最大订单大小
    pub max_order_size: Option<f64>,
}

/// 执行频率
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionFrequency {
    /// 每个Bar
    EveryBar,
    /// 每分钟
    EveryMinute,
    /// 每小时
    Hourly,
    /// 每日
    Daily,
    /// 自定义间隔（秒）
    Custom(u64),
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    pub level: LogLevel,
    /// 日志输出目标
    pub targets: Vec<LogTarget>,
    /// 是否启用详细日志
    pub verbose: bool,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// 日志输出目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogTarget {
    Console,
    File(String),
    Database,
}

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            framework: FrameworkSettings {
                name: "MosesQuant".to_string(),
                version: "1.0.0".to_string(),
                mode: RunMode::Backtest,
                timezone: "UTC".to_string(),
                base_currency: "USDT".to_string(),
                initial_capital: 100000.0,
            },
            data_sources: vec![
                DataSourceConfig {
                    name: "csv_historical".to_string(),
                    source_type: DataSourceType::Csv,
                    connection: ConnectionConfig {
                        params: {
                            let mut params = HashMap::new();
                            params.insert("file_path".to_string(), serde_json::Value::String("data/BTCUSDT_1m.csv".to_string()));
                            params
                        },
                    },
                    symbols: vec!["BTCUSDT".to_string()],
                    enabled: true,
                },
                DataSourceConfig {
                    name: "binance_live".to_string(),
                    source_type: DataSourceType::Binance,
                    connection: ConnectionConfig {
                        params: {
                            let mut params = HashMap::new();
                            params.insert("testnet".to_string(), serde_json::Value::Bool(true));
                            params
                        },
                    },
                    symbols: vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
                    enabled: false,
                },
            ],
            strategies: vec![
                StrategyConfig {
                    id: "momentum_strategy_1".to_string(),
                    name: "BTC Momentum Strategy".to_string(),
                    strategy_type: StrategyType::Momentum,
                    universe_selector: ComponentConfig {
                        component_type: "SimpleUniverseSelector".to_string(),
                        parameters: {
                            let mut params = HashMap::new();
                            params.insert("symbols".to_string(), serde_json::Value::Array(vec![
                                serde_json::Value::String("BTCUSDT".to_string())
                            ]));
                            params
                        },
                    },
                    alpha_model: ComponentConfig {
                        component_type: "SimpleAlphaModel".to_string(),
                        parameters: HashMap::new(),
                    },
                    portfolio_constructor: ComponentConfig {
                        component_type: "SimplePortfolioConstructor".to_string(),
                        parameters: HashMap::new(),
                    },
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("lookback_period".to_string(), serde_json::Value::Number(serde_json::Number::from(20)));
                        params.insert("signal_threshold".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.02).unwrap()));
                        params
                    },
                    enabled: true,
                },
            ],
            risk_management: RiskManagementConfig {
                max_position_size: 0.15,
                max_total_position: 0.95,
                max_drawdown: 0.20,
                stop_loss: Some(0.05),
                take_profit: Some(0.10),
                risk_measures: vec![
                    RiskMeasure::VaR { confidence: 0.95, horizon: 1 },
                    RiskMeasure::MaxDrawdown,
                    RiskMeasure::Volatility { window: 30 },
                ],
            },
            execution: ExecutionConfig {
                frequency: ExecutionFrequency::EveryBar,
                order_type: OrderType::Market,
                slippage_tolerance: 0.001,
                min_order_size: 0.001,
                max_order_size: None,
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                targets: vec![LogTarget::Console, LogTarget::File("moses_quant.log".to_string())],
                verbose: false,
            },
        }
    }
}

/// 配置管理器
#[derive(Debug)]
pub struct ConfigManager {
    config: FrameworkConfig,
}

impl ConfigManager {
    /// 从文件加载配置
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| crate::CzscError::config(&format!("Failed to read config file: {}", e)))?;
        
        let config: FrameworkConfig = serde_yaml::from_str(&content)
            .map_err(|e| crate::CzscError::config(&format!("Failed to parse config file: {}", e)))?;
        
        Ok(Self { config })
    }
    
    /// 创建默认配置
    pub fn new_default() -> Self {
        Self {
            config: FrameworkConfig::default(),
        }
    }
    
    /// 保存配置到文件
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(&self.config)
            .map_err(|e| crate::CzscError::config(&format!("Failed to serialize config: {}", e)))?;
        
        tokio::fs::write(path, content).await
            .map_err(|e| crate::CzscError::config(&format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取配置
    pub fn get_config(&self) -> &FrameworkConfig {
        &self.config
    }
    
    /// 获取可变配置
    pub fn get_config_mut(&mut self) -> &mut FrameworkConfig {
        &mut self.config
    }
    
    /// 获取启用的数据源
    pub fn get_enabled_data_sources(&self) -> Vec<&DataSourceConfig> {
        self.config.data_sources.iter().filter(|ds| ds.enabled).collect()
    }
    
    /// 获取启用的策略
    pub fn get_enabled_strategies(&self) -> Vec<&StrategyConfig> {
        self.config.strategies.iter().filter(|s| s.enabled).collect()
    }
    
    /// 验证配置有效性
    pub fn validate(&self) -> Result<()> {
        // 检查必要字段
        if self.config.framework.name.is_empty() {
            return Err(crate::CzscError::config("Framework name cannot be empty"));
        }
        
        if self.config.framework.initial_capital <= 0.0 {
            return Err(crate::CzscError::config("Initial capital must be positive"));
        }
        
        // 检查数据源配置
        let enabled_sources = self.get_enabled_data_sources();
        if enabled_sources.is_empty() {
            return Err(crate::CzscError::config("At least one data source must be enabled"));
        }
        
        // 检查策略配置
        let enabled_strategies = self.get_enabled_strategies();
        if enabled_strategies.is_empty() {
            return Err(crate::CzscError::config("At least one strategy must be enabled"));
        }
        
        // 检查风险管理参数
        if self.config.risk_management.max_position_size <= 0.0 || self.config.risk_management.max_position_size > 1.0 {
            return Err(crate::CzscError::config("Max position size must be between 0 and 1"));
        }
        
        tracing::info!("Configuration validation passed");
        Ok(())
    }
}

/// 生成默认配置文件
pub async fn generate_default_config_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let config_manager = ConfigManager::new_default();
    config_manager.save_to_file(path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_config_manager_default() {
        let config_manager = ConfigManager::new_default();
        let config = config_manager.get_config();
        
        assert_eq!(config.framework.name, "MosesQuant");
        assert_eq!(config.framework.initial_capital, 100000.0);
        assert!(!config.data_sources.is_empty());
        assert!(!config.strategies.is_empty());
        
        // 验证配置
        assert!(config_manager.validate().is_ok());
    }
    
    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_path = "test_config.yaml";
        
        // 创建并保存配置
        let config_manager = ConfigManager::new_default();
        config_manager.save_to_file(temp_path).await.unwrap();
        
        // 加载配置
        let loaded_config = ConfigManager::load_from_file(temp_path).await.unwrap();
        
        assert_eq!(loaded_config.get_config().framework.name, "MosesQuant");
        
        // 清理测试文件
        let _ = fs::remove_file(temp_path).await;
    }
    
    #[tokio::test]
    async fn test_enabled_sources_and_strategies() {
        let config_manager = ConfigManager::new_default();
        
        let enabled_sources = config_manager.get_enabled_data_sources();
        assert!(!enabled_sources.is_empty());
        
        let enabled_strategies = config_manager.get_enabled_strategies();
        assert!(!enabled_strategies.is_empty());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = FrameworkConfig::default();
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        
        assert!(yaml_str.contains("MosesQuant"));
        assert!(yaml_str.contains("data_sources"));
        assert!(yaml_str.contains("strategies"));
        
        // 测试反序列化
        let deserialized: FrameworkConfig = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(deserialized.framework.name, "MosesQuant");
    }
}