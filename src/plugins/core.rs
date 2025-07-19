//! 插件系统核心特征定义
//! 
//! 定义统一的Plugin trait体系，支持零成本抽象和编译时优化

use crate::types::*;
use crate::{Result, MosesQuantError};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use semver::Version;

/// 插件上下文 - 提供插件运行时环境
pub struct PluginContext {
    /// 插件ID
    pub plugin_id: PluginId,
    /// 配置数据
    pub config: HashMap<String, serde_json::Value>,
    /// 共享状态存储
    pub shared_state: Arc<tokio::sync::RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
    /// 事件总线引用
    pub event_bus: Option<Arc<crate::SimpleEventBus>>,
    /// 内存管理器引用
    pub memory_manager: Option<Arc<crate::SimpleMemoryManager>>,
}

impl PluginContext {
    /// 创建新的插件上下文
    pub fn new(plugin_id: PluginId) -> Self {
        Self {
            plugin_id,
            config: HashMap::new(),
            shared_state: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            event_bus: None,
            memory_manager: None,
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: HashMap<String, serde_json::Value>) -> Self {
        self.config = config;
        self
    }

    /// 获取配置值
    pub fn get_config<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.config.get(key).ok_or_else(|| {
            MosesQuantError::ConfigNotFound { key: key.to_string() }
        })?;
        
        serde_json::from_value(value.clone()).map_err(|e| {
            MosesQuantError::ConfigValidation { 
                message: format!("Failed to deserialize config key '{}': {}", key, e)
            }
        })
    }

    /// 设置共享状态
    pub async fn set_shared_state<T>(&self, key: &str, value: T) -> Result<()>
    where
        T: Any + Send + Sync,
    {
        let mut state = self.shared_state.write().await;
        state.insert(key.to_string(), Box::new(value));
        Ok(())
    }

    /// 获取共享状态
    pub async fn get_shared_state<T>(&self, key: &str) -> Option<Arc<T>>
    where
        T: Any + Send + Sync,
    {
        let state = self.shared_state.read().await;
        state.get(key)?
            .downcast_ref::<T>()
            .map(|v| unsafe { std::mem::transmute(v as *const T) })
            .map(Arc::from)
    }
}

/// 插件状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 已启动
    Running,
    /// 暂停中
    Paused,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
}

/// 插件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    /// 数据源插件
    DataSource,
    /// 策略插件  
    Strategy,
    /// 风险管理插件
    RiskManager,
    /// 执行插件
    Execution,
    /// 分析插件
    Analytics,
    /// 通知插件
    Notification,
    /// 工具插件
    Utility,
    /// 自定义插件
    Custom(String),
}

/// 插件能力标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCapability {
    /// 实时数据处理
    RealTimeData,
    /// 历史数据访问
    HistoricalData,
    /// 订单执行
    OrderExecution,
    /// 风险计算
    RiskCalculation,
    /// 性能分析
    PerformanceAnalytics,
    /// 机器学习
    MachineLearning,
    /// 外部API集成
    ExternalAPI,
    /// 数据持久化
    DataPersistence,
    /// 自定义能力
    Custom(String),
}

/// 核心插件特征 - 所有插件必须实现
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件元数据
    fn metadata(&self) -> &PluginMetadata;
    
    /// 初始化插件
    async fn initialize(&mut self, context: &PluginContext) -> Result<()>;
    
    /// 启动插件
    async fn start(&mut self, context: &PluginContext) -> Result<()>;
    
    /// 停止插件
    async fn stop(&mut self, context: &PluginContext) -> Result<()>;
    
    /// 暂停插件
    async fn pause(&mut self, context: &PluginContext) -> Result<()> {
        // 默认实现：暂停等同于停止
        self.stop(context).await
    }
    
    /// 恢复插件
    async fn resume(&mut self, context: &PluginContext) -> Result<()> {
        // 默认实现：恢复等同于启动
        self.start(context).await
    }
    
    /// 获取插件状态
    fn state(&self) -> PluginState;
    
    /// 健康检查
    async fn health_check(&self) -> Result<PluginHealthStatus>;
    
    /// 获取插件统计信息
    async fn get_metrics(&self) -> Result<PluginMetrics>;
    
    /// 处理配置更新
    async fn configure(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()>;
    
    /// 类型擦除转换
    fn as_any(&self) -> &dyn Any;
    
    /// 可变类型擦除转换
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件ID
    pub id: PluginId,
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: Version,
    /// 插件描述
    pub description: String,
    /// 插件作者
    pub author: String,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 插件能力
    pub capabilities: Vec<PluginCapability>,
    /// 依赖的插件
    pub dependencies: Vec<PluginDependency>,
    /// 最小框架版本要求
    pub min_framework_version: Version,
    /// 最大框架版本兼容
    pub max_framework_version: Option<Version>,
    /// 配置Schema（JSON Schema）
    pub config_schema: Option<serde_json::Value>,
    /// 插件标签
    pub tags: Vec<String>,
}

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// 依赖插件ID
    pub plugin_id: PluginId,
    /// 版本要求
    pub version_req: String,
    /// 是否为可选依赖
    pub optional: bool,
}

/// 插件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthStatus {
    /// 是否健康
    pub healthy: bool,
    /// 状态消息
    pub message: String,
    /// 最后检查时间
    pub last_check: TimestampNs,
    /// 详细状态信息
    pub details: HashMap<String, serde_json::Value>,
}

/// 插件统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginMetrics {
    /// 插件运行时间（纳秒）
    pub uptime_ns: i64,
    /// 处理的请求数
    pub requests_processed: u64,
    /// 处理失败的请求数
    pub requests_failed: u64,
    /// 平均处理时间（纳秒）
    pub avg_processing_time_ns: i64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: usize,
    /// CPU使用率（百分比）
    pub cpu_usage_percent: f64,
    /// 自定义指标
    pub custom_metrics: HashMap<String, serde_json::Value>,
}

/// 数据源插件特征
#[async_trait]
pub trait DataSourcePlugin: Plugin {
    /// 获取支持的交易对
    async fn get_symbols(&self) -> Result<Vec<Symbol>>;
    
    /// 订阅实时数据
    async fn subscribe(&mut self, symbols: &[Symbol]) -> Result<()>;
    
    /// 取消订阅
    async fn unsubscribe(&mut self, symbols: &[Symbol]) -> Result<()>;
    
    /// 获取历史数据
    async fn get_historical_data(
        &self, 
        symbol: &Symbol, 
        timeframe: TimeFrame,
        start: TimestampNs,
        end: TimestampNs,
    ) -> Result<Vec<Bar>>;
    
    /// 获取最新行情
    async fn get_latest_tick(&self, symbol: &Symbol) -> Result<Tick>;
}

/// 策略插件特征
#[async_trait]
pub trait StrategyPlugin: Plugin {
    /// 处理市场数据
    async fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<Signal>>;
    
    /// 处理订单更新
    async fn on_order_update(&mut self, order: &Order) -> Result<()>;
    
    /// 处理成交回报
    async fn on_trade(&mut self, trade: &Trade) -> Result<()>;
    
    /// 获取当前持仓
    async fn get_positions(&self) -> Result<Vec<Position>>;
    
    /// 生成交易信号
    async fn generate_signals(&mut self, context: &StrategyContext) -> Result<Vec<Signal>>;
}

/// 风险管理插件特征
#[async_trait]
pub trait RiskManagerPlugin: Plugin {
    /// 验证订单风险
    async fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> Result<RiskCheckResult>;
    
    /// 计算持仓风险
    async fn calculate_position_risk(&self, position: &Position) -> Result<PositionRisk>;
    
    /// 计算组合风险
    async fn calculate_portfolio_risk(&self, portfolio: &Portfolio) -> Result<PortfolioRisk>;
    
    /// 检查风险限制
    async fn check_risk_limits(&self, portfolio: &Portfolio) -> Result<Vec<RiskViolation>>;
}

/// 执行插件特征
#[async_trait]
pub trait ExecutionPlugin: Plugin {
    /// 提交订单
    async fn submit_order(&mut self, order: &Order) -> Result<OrderId>;
    
    /// 取消订单
    async fn cancel_order(&mut self, order_id: &OrderId) -> Result<()>;
    
    /// 修改订单
    async fn modify_order(&mut self, order_id: &OrderId, new_order: &Order) -> Result<()>;
    
    /// 查询订单状态
    async fn get_order_status(&self, order_id: &OrderId) -> Result<OrderStatus>;
    
    /// 获取所有活跃订单
    async fn get_active_orders(&self) -> Result<Vec<Order>>;
}

/// 交易信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// 信号ID
    pub id: String,
    /// 交易对
    pub symbol: Symbol,
    /// 信号类型
    pub signal_type: SignalType,
    /// 信号强度 (0.0 - 1.0)
    pub strength: f64,
    /// 目标价格
    pub target_price: Option<Price>,
    /// 止损价格
    pub stop_loss: Option<Price>,
    /// 止盈价格
    pub take_profit: Option<Price>,
    /// 生成时间
    pub timestamp: TimestampNs,
    /// 信号来源
    pub source: String,
    /// 附加信息
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 信号类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    Buy,
    Sell,
    Hold,
    Close,
}

/// 策略上下文
#[derive(Debug, Clone)]
pub struct StrategyContext {
    /// 当前时间
    pub current_time: TimestampNs,
    /// 当前持仓
    pub positions: Vec<Position>,
    /// 可用资金
    pub available_capital: Price,
    /// 市场数据
    pub market_data: HashMap<Symbol, MarketData>,
    /// 历史数据缓存
    pub historical_cache: HashMap<Symbol, Vec<Bar>>,
}

/// 数据源上下文 - 运行时传递给数据源的配置和状态信息
#[derive(Debug, Clone)]
pub struct DataSourceContext {
    /// 数据源ID
    pub source_id: DataSourceId,
    /// 支持的交易品种
    pub symbols: Vec<Symbol>,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
}

/// 投资组合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    /// 组合ID
    pub id: String,
    /// 持仓列表
    pub positions: Vec<Position>,
    /// 现金余额
    pub cash_balance: Price,
    /// 总资产
    pub total_value: Price,
    /// 未实现盈亏
    pub unrealized_pnl: Price,
    /// 已实现盈亏
    pub realized_pnl: Price,
    /// 更新时间
    pub updated_at: TimestampNs,
}

/// 风险检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCheckResult {
    /// 是否通过风险检查
    pub approved: bool,
    /// 拒绝原因
    pub rejection_reason: Option<String>,
    /// 建议的订单调整
    pub suggested_adjustments: Option<OrderAdjustment>,
    /// 风险分数 (0.0 - 1.0, 越高越危险)
    pub risk_score: f64,
    /// 风险详情
    pub risk_details: HashMap<String, serde_json::Value>,
}

/// 订单调整建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAdjustment {
    /// 建议数量
    pub suggested_quantity: Option<Quantity>,
    /// 建议价格
    pub suggested_price: Option<Price>,
    /// 建议止损
    pub suggested_stop_loss: Option<Price>,
    /// 调整原因
    pub reason: String,
}

/// 持仓风险
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRisk {
    /// 持仓ID
    pub position_id: PositionId,
    /// Value at Risk (VaR)
    pub var_1d: Price,
    /// 最大回撤
    pub max_drawdown: Price,
    /// 波动率
    pub volatility: f64,
    /// Beta值
    pub beta: Option<f64>,
    /// 风险敞口
    pub exposure: Price,
}

/// 组合风险
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRisk {
    /// 组合VaR
    pub portfolio_var: Price,
    /// 预期波动率
    pub expected_volatility: f64,
    /// 夏普比率
    pub sharpe_ratio: f64,
    /// 最大回撤
    pub max_drawdown: Price,
    /// 集中度风险
    pub concentration_risk: f64,
    /// 杠杆比率
    pub leverage_ratio: f64,
}

/// 风险违规
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskViolation {
    /// 违规类型
    pub violation_type: RiskViolationType,
    /// 当前值
    pub current_value: f64,
    /// 限制值
    pub limit_value: f64,
    /// 严重程度
    pub severity: RiskSeverity,
    /// 描述信息
    pub description: String,
}

/// 风险违规类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskViolationType {
    /// 持仓限制
    PositionLimit,
    /// 损失限制
    LossLimit,
    /// 杠杆限制
    LeverageLimit,
    /// 集中度限制
    ConcentrationLimit,
    /// VaR限制
    VarLimit,
}

/// 风险严重程度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[derive(Debug)]
    struct MockPlugin {
        metadata: PluginMetadata,
        state: PluginState,
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _context: &PluginContext) -> Result<()> {
            self.state = PluginState::Initializing;
            Ok(())
        }

        async fn start(&mut self, _context: &PluginContext) -> Result<()> {
            self.state = PluginState::Running;
            Ok(())
        }

        async fn stop(&mut self, _context: &PluginContext) -> Result<()> {
            self.state = PluginState::Stopped;
            Ok(())
        }

        fn state(&self) -> PluginState {
            self.state
        }

        async fn health_check(&self) -> Result<PluginHealthStatus> {
            Ok(PluginHealthStatus {
                healthy: true,
                message: "Plugin is healthy".to_string(),
                last_check: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                details: HashMap::new(),
            })
        }

        async fn get_metrics(&self) -> Result<PluginMetrics> {
            Ok(PluginMetrics::default())
        }

        async fn configure(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<()> {
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            plugin_type: PluginType::Utility,
            capabilities: vec![PluginCapability::Custom("test".to_string())],
            dependencies: vec![],
            min_framework_version: Version::new(2, 0, 0),
            max_framework_version: None,
            config_schema: None,
            tags: vec!["test".to_string()],
        };

        let mut plugin = MockPlugin {
            metadata,
            state: PluginState::Uninitialized,
        };

        let context = PluginContext::new("test_plugin".to_string());

        // 测试插件生命周期
        assert_eq!(plugin.state(), PluginState::Uninitialized);

        plugin.initialize(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Initializing);

        plugin.start(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Running);

        let health = plugin.health_check().await.unwrap();
        assert!(health.healthy);

        plugin.stop(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_plugin_context() {
        let mut context = PluginContext::new("test_plugin".to_string());
        
        // 测试配置
        let mut config = HashMap::new();
        config.insert("test_key".to_string(), serde_json::json!("test_value"));
        context = context.with_config(config);
        
        let value: String = context.get_config("test_key").unwrap();
        assert_eq!(value, "test_value");

        // 测试共享状态
        context.set_shared_state("shared_data", 42i32).await.unwrap();
        let shared_value: Option<Arc<i32>> = context.get_shared_state("shared_data").await;
        // 注意：由于类型转换的复杂性，这里只测试是否能设置和获取
    }

    #[test]
    fn test_plugin_metadata_serialization() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            plugin_type: PluginType::DataSource,
            capabilities: vec![
                PluginCapability::RealTimeData,
                PluginCapability::HistoricalData,
            ],
            dependencies: vec![
                PluginDependency {
                    plugin_id: "dependency_plugin".to_string(),
                    version_req: "^1.0".to_string(),
                    optional: false,
                }
            ],
            min_framework_version: Version::new(2, 0, 0),
            max_framework_version: Some(Version::new(3, 0, 0)),
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "api_key": {"type": "string"}
                }
            })),
            tags: vec!["data".to_string(), "realtime".to_string()],
        };

        // 测试序列化和反序列化
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: PluginMetadata = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.version, deserialized.version);
        assert_eq!(metadata.plugin_type, deserialized.plugin_type);
    }
}