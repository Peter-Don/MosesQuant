//! 可插拔策略引擎
//! 
//! 基于插件系统的高性能策略执行引擎，支持多策略并发执行

use crate::plugins::*;
use crate::types::*;
use crate::{Result, MosesQuantError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};

/// 策略引擎配置
#[derive(Debug, Clone)]
pub struct StrategyEngineConfig {
    /// 最大并发策略数
    pub max_concurrent_strategies: usize,
    /// 策略执行超时时间
    pub strategy_timeout: Duration,
    /// 信号处理超时时间
    pub signal_processing_timeout: Duration,
    /// 是否启用风险检查
    pub enable_risk_checks: bool,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
    /// 执行间隔
    pub execution_interval: Duration,
    /// 最大信号队列大小
    pub max_signal_queue_size: usize,
}

impl Default for StrategyEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_strategies: 10,
            strategy_timeout: Duration::from_secs(30),
            signal_processing_timeout: Duration::from_secs(5),
            enable_risk_checks: true,
            enable_performance_monitoring: true,
            execution_interval: Duration::from_millis(100),
            max_signal_queue_size: 1000,
        }
    }
}

/// 策略执行状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyExecutionState {
    /// 未启动
    Stopped,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 暂停中
    Paused,
    /// 正在停止
    Stopping,
    /// 错误状态
    Error,
}

/// 策略运行时统计
#[derive(Debug, Clone, Default)]
pub struct StrategyRuntimeStats {
    /// 启动时间
    pub start_time: Option<Instant>,
    /// 总运行时间
    pub total_runtime: Duration,
    /// 处理的信号数量
    pub signals_processed: u64,
    /// 生成的订单数量
    pub orders_generated: u64,
    /// 错误次数
    pub error_count: u64,
    /// 平均执行时间
    pub avg_execution_time: Duration,
    /// 最后执行时间
    pub last_execution_time: Option<Instant>,
    /// 性能得分
    pub performance_score: f64,
}

/// 策略运行时信息
#[derive(Debug)]
pub struct StrategyRuntime {
    /// 策略插件
    pub plugin: Arc<Mutex<dyn StrategyPlugin>>,
    /// 执行状态
    pub state: StrategyExecutionState,
    /// 运行时统计
    pub stats: StrategyRuntimeStats,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
    /// 上下文
    pub context: StrategyContext,
    /// 最后错误
    pub last_error: Option<String>,
}

/// 策略引擎
pub struct StrategyEngine {
    /// 注册的策略
    strategies: Arc<RwLock<HashMap<StrategyId, StrategyRuntime>>>,
    /// 插件注册表
    plugin_registry: Arc<PluginRegistry>,
    /// 生命周期管理器
    lifecycle_manager: Arc<PluginLifecycleManager>,
    /// 通信管理器
    communication_manager: Arc<PluginCommunicationManager>,
    /// 引擎配置
    config: StrategyEngineConfig,
    /// 信号队列
    signal_queue: Arc<Mutex<Vec<(StrategyId, Signal)>>>,
    /// 引擎状态
    engine_state: Arc<RwLock<EngineState>>,
    /// 执行任务句柄
    execution_handle: Option<tokio::task::JoinHandle<()>>,
}

/// 引擎状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// 策略执行结果
#[derive(Debug, Clone)]
pub struct StrategyExecutionResult {
    /// 策略ID
    pub strategy_id: StrategyId,
    /// 执行是否成功
    pub success: bool,
    /// 生成的信号
    pub signals: Vec<Signal>,
    /// 执行时间
    pub execution_time: Duration,
    /// 错误信息
    pub error: Option<String>,
    /// 性能指标
    pub performance_metrics: HashMap<String, f64>,
}

impl StrategyEngine {
    /// 创建新的策略引擎
    pub fn new(
        config: StrategyEngineConfig,
        plugin_registry: Arc<PluginRegistry>,
        lifecycle_manager: Arc<PluginLifecycleManager>,
        communication_manager: Arc<PluginCommunicationManager>,
    ) -> Self {
        Self {
            strategies: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry,
            lifecycle_manager,
            communication_manager,
            config,
            signal_queue: Arc::new(Mutex::new(Vec::new())),
            engine_state: Arc::new(RwLock::new(EngineState::Stopped)),
            execution_handle: None,
        }
    }

    /// 注册策略插件
    pub async fn register_strategy(
        &self,
        strategy_id: StrategyId,
        plugin: Arc<Mutex<dyn StrategyPlugin>>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // 检查策略数量限制
        {
            let strategies = self.strategies.read().await;
            if strategies.len() >= self.config.max_concurrent_strategies {
                return Err(MosesQuantError::Internal {
                    message: "Maximum number of strategies reached".to_string()
                });
            }
        }

        // 创建策略上下文
        let context = StrategyContext {
            current_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            positions: Vec::new(),
            available_capital: rust_decimal::Decimal::from(100000), // 默认资金
            market_data: HashMap::new(),
            historical_cache: HashMap::new(),
        };

        // 创建策略运行时
        let runtime = StrategyRuntime {
            plugin,
            state: StrategyExecutionState::Stopped,
            stats: StrategyRuntimeStats::default(),
            config,
            context,
            last_error: None,
        };

        // 注册策略
        {
            let mut strategies = self.strategies.write().await;
            strategies.insert(strategy_id.clone(), runtime);
        }

        info!("Strategy '{}' registered successfully", strategy_id);
        Ok(())
    }

    /// 启动策略
    pub async fn start_strategy(&self, strategy_id: &StrategyId) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        if let Some(runtime) = strategies.get_mut(strategy_id) {
            match runtime.state {
                StrategyExecutionState::Stopped => {
                    runtime.state = StrategyExecutionState::Starting;
                    runtime.stats.start_time = Some(Instant::now());
                    
                    // 初始化策略插件
                    let plugin_context = PluginContext::new(strategy_id.clone())
                        .with_config(runtime.config.clone());
                    
                    {
                        let mut plugin = runtime.plugin.lock().await;
                        plugin.initialize(&plugin_context).await?;
                        plugin.start(&plugin_context).await?;
                    }
                    
                    runtime.state = StrategyExecutionState::Running;
                    info!("Strategy '{}' started successfully", strategy_id);
                    Ok(())
                }
                _ => {
                    Err(MosesQuantError::Internal {
                        message: format!("Strategy '{}' is not in stopped state", strategy_id)
                    })
                }
            }
        } else {
            Err(MosesQuantError::StrategyNotFound { strategy_id: strategy_id.clone() })
        }
    }

    /// 停止策略
    pub async fn stop_strategy(&self, strategy_id: &StrategyId) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        if let Some(runtime) = strategies.get_mut(strategy_id) {
            match runtime.state {
                StrategyExecutionState::Running | StrategyExecutionState::Paused => {
                    runtime.state = StrategyExecutionState::Stopping;
                    
                    let plugin_context = PluginContext::new(strategy_id.clone());
                    
                    {
                        let mut plugin = runtime.plugin.lock().await;
                        plugin.stop(&plugin_context).await?;
                    }
                    
                    runtime.state = StrategyExecutionState::Stopped;
                    
                    // 更新运行时统计
                    if let Some(start_time) = runtime.stats.start_time {
                        runtime.stats.total_runtime += start_time.elapsed();
                        runtime.stats.start_time = None;
                    }
                    
                    info!("Strategy '{}' stopped successfully", strategy_id);
                    Ok(())
                }
                _ => {
                    Err(MosesQuantError::Internal {
                        message: format!("Strategy '{}' is not in running state", strategy_id)
                    })
                }
            }
        } else {
            Err(MosesQuantError::StrategyNotFound { strategy_id: strategy_id.clone() })
        }
    }

    /// 暂停策略
    pub async fn pause_strategy(&self, strategy_id: &StrategyId) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        if let Some(runtime) = strategies.get_mut(strategy_id) {
            if runtime.state == StrategyExecutionState::Running {
                let plugin_context = PluginContext::new(strategy_id.clone());
                
                {
                    let mut plugin = runtime.plugin.lock().await;
                    plugin.pause(&plugin_context).await?;
                }
                
                runtime.state = StrategyExecutionState::Paused;
                info!("Strategy '{}' paused successfully", strategy_id);
                Ok(())
            } else {
                Err(MosesQuantError::Internal {
                    message: format!("Strategy '{}' is not in running state", strategy_id)
                })
            }
        } else {
            Err(MosesQuantError::StrategyNotFound { strategy_id: strategy_id.clone() })
        }
    }

    /// 恢复策略
    pub async fn resume_strategy(&self, strategy_id: &StrategyId) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        if let Some(runtime) = strategies.get_mut(strategy_id) {
            if runtime.state == StrategyExecutionState::Paused {
                let plugin_context = PluginContext::new(strategy_id.clone());
                
                {
                    let mut plugin = runtime.plugin.lock().await;
                    plugin.resume(&plugin_context).await?;
                }
                
                runtime.state = StrategyExecutionState::Running;
                info!("Strategy '{}' resumed successfully", strategy_id);
                Ok(())
            } else {
                Err(MosesQuantError::Internal {
                    message: format!("Strategy '{}' is not in paused state", strategy_id)
                })
            }
        } else {
            Err(MosesQuantError::StrategyNotFound { strategy_id: strategy_id.clone() })
        }
    }

    /// 启动引擎
    pub async fn start_engine(&mut self) -> Result<()> {
        {
            let mut state = self.engine_state.write().await;
            if *state != EngineState::Stopped {
                return Err(MosesQuantError::Internal {
                    message: "Engine is not in stopped state".to_string()
                });
            }
            *state = EngineState::Starting;
        }

        // 启动执行循环
        self.start_execution_loop().await;

        {
            let mut state = self.engine_state.write().await;
            *state = EngineState::Running;
        }

        info!("Strategy engine started successfully");
        Ok(())
    }

    /// 停止引擎
    pub async fn stop_engine(&mut self) -> Result<()> {
        {
            let mut state = self.engine_state.write().await;
            if *state != EngineState::Running {
                return Err(MosesQuantError::Internal {
                    message: "Engine is not in running state".to_string()
                });
            }
            *state = EngineState::Stopping;
        }

        // 停止所有策略
        let strategy_ids: Vec<StrategyId> = {
            let strategies = self.strategies.read().await;
            strategies.keys().cloned().collect()
        };

        for strategy_id in strategy_ids {
            if let Err(e) = self.stop_strategy(&strategy_id).await {
                warn!("Failed to stop strategy '{}': {:?}", strategy_id, e);
            }
        }

        // 停止执行循环
        if let Some(handle) = self.execution_handle.take() {
            handle.abort();
        }

        {
            let mut state = self.engine_state.write().await;
            *state = EngineState::Stopped;
        }

        info!("Strategy engine stopped successfully");
        Ok(())
    }

    /// 处理市场数据
    pub async fn on_market_data(&self, data: &MarketData) -> Result<()> {
        let strategies = self.strategies.read().await;
        
        for (strategy_id, runtime) in strategies.iter() {
            if runtime.state == StrategyExecutionState::Running {
                let plugin = runtime.plugin.clone();
                let strategy_id_clone = strategy_id.clone();
                
                // 异步处理市场数据
                tokio::spawn(async move {
                    let result = tokio::time::timeout(
                        Duration::from_secs(5),
                        async {
                            let mut plugin_guard = plugin.lock().await;
                            plugin_guard.on_market_data(data).await
                        }
                    ).await;

                    match result {
                        Ok(Ok(signals)) => {
                            debug!("Strategy '{}' generated {} signals", strategy_id_clone, signals.len());
                            // 处理生成的信号
                            // TODO: 添加信号到信号队列
                        }
                        Ok(Err(e)) => {
                            error!("Strategy '{}' error processing market data: {:?}", strategy_id_clone, e);
                        }
                        Err(_) => {
                            error!("Strategy '{}' timeout processing market data", strategy_id_clone);
                        }
                    }
                });
            }
        }
        
        Ok(())
    }

    /// 获取策略状态
    pub async fn get_strategy_state(&self, strategy_id: &StrategyId) -> Option<StrategyExecutionState> {
        let strategies = self.strategies.read().await;
        strategies.get(strategy_id).map(|runtime| runtime.state.clone())
    }

    /// 获取策略统计信息
    pub async fn get_strategy_stats(&self, strategy_id: &StrategyId) -> Option<StrategyRuntimeStats> {
        let strategies = self.strategies.read().await;
        strategies.get(strategy_id).map(|runtime| runtime.stats.clone())
    }

    /// 获取所有策略状态
    pub async fn get_all_strategies_status(&self) -> HashMap<StrategyId, StrategyExecutionState> {
        let strategies = self.strategies.read().await;
        strategies.iter()
            .map(|(id, runtime)| (id.clone(), runtime.state.clone()))
            .collect()
    }

    /// 获取引擎统计信息
    pub async fn get_engine_stats(&self) -> EngineStats {
        let strategies = self.strategies.read().await;
        let engine_state = self.engine_state.read().await;
        
        let mut stats = EngineStats {
            engine_state: engine_state.clone(),
            total_strategies: strategies.len(),
            running_strategies: 0,
            paused_strategies: 0,
            stopped_strategies: 0,
            error_strategies: 0,
            total_signals_processed: 0,
            total_orders_generated: 0,
            total_errors: 0,
            average_execution_time: Duration::ZERO,
        };

        let mut total_execution_time = Duration::ZERO;
        let mut execution_count = 0;

        for runtime in strategies.values() {
            match runtime.state {
                StrategyExecutionState::Running => stats.running_strategies += 1,
                StrategyExecutionState::Paused => stats.paused_strategies += 1,
                StrategyExecutionState::Stopped => stats.stopped_strategies += 1,
                StrategyExecutionState::Error => stats.error_strategies += 1,
                _ => {}
            }

            stats.total_signals_processed += runtime.stats.signals_processed;
            stats.total_orders_generated += runtime.stats.orders_generated;
            stats.total_errors += runtime.stats.error_count;
            
            total_execution_time += runtime.stats.avg_execution_time;
            execution_count += 1;
        }

        if execution_count > 0 {
            stats.average_execution_time = total_execution_time / execution_count as u32;
        }

        stats
    }

    /// 启动执行循环
    async fn start_execution_loop(&mut self) {
        let strategies = self.strategies.clone();
        let signal_queue = self.signal_queue.clone();
        let engine_state = self.engine_state.clone();
        let execution_interval = self.config.execution_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(execution_interval);
            
            loop {
                interval.tick().await;
                
                // 检查引擎状态
                {
                    let state = engine_state.read().await;
                    if *state != EngineState::Running {
                        break;
                    }
                }

                // 处理信号队列
                Self::process_signal_queue(&signal_queue).await;

                // 执行策略
                Self::execute_strategies(&strategies).await;
            }
        });

        self.execution_handle = Some(handle);
    }

    /// 处理信号队列
    async fn process_signal_queue(signal_queue: &Arc<Mutex<Vec<(StrategyId, Signal)>>>) {
        let mut queue = signal_queue.lock().await;
        
        if !queue.is_empty() {
            let signals: Vec<(StrategyId, Signal)> = queue.drain(..).collect();
            drop(queue);

            for (strategy_id, signal) in signals {
                debug!("Processing signal from strategy '{}': {:?}", strategy_id, signal);
                // TODO: 实现信号处理逻辑
            }
        }
    }

    /// 执行所有运行中的策略
    async fn execute_strategies(strategies: &Arc<RwLock<HashMap<StrategyId, StrategyRuntime>>>) {
        let strategy_list = {
            let strategies_guard = strategies.read().await;
            strategies_guard.iter()
                .filter(|(_, runtime)| runtime.state == StrategyExecutionState::Running)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };

        for strategy_id in strategy_list {
            // 为每个策略创建独立的执行任务
            let strategies_clone = strategies.clone();
            let strategy_id_clone = strategy_id.clone();
            
            tokio::spawn(async move {
                Self::execute_single_strategy(&strategies_clone, &strategy_id_clone).await;
            });
        }
    }

    /// 执行单个策略
    async fn execute_single_strategy(
        strategies: &Arc<RwLock<HashMap<StrategyId, StrategyRuntime>>>,
        strategy_id: &StrategyId,
    ) {
        let execution_start = Instant::now();
        
        let result = {
            let strategies_guard = strategies.read().await;
            if let Some(runtime) = strategies_guard.get(strategy_id) {
                if runtime.state != StrategyExecutionState::Running {
                    return;
                }
                
                let plugin = runtime.plugin.clone();
                let context = runtime.context.clone();
                
                drop(strategies_guard);

                // 执行策略
                let plugin_guard = plugin.lock().await;
                plugin_guard.generate_signals(&context).await
            } else {
                return;
            }
        };

        let execution_time = execution_start.elapsed();

        // 更新策略统计信息
        {
            let mut strategies_guard = strategies.write().await;
            if let Some(runtime) = strategies_guard.get_mut(strategy_id) {
                runtime.stats.last_execution_time = Some(Instant::now());
                
                match result {
                    Ok(signals) => {
                        runtime.stats.signals_processed += signals.len() as u64;
                        runtime.stats.avg_execution_time = 
                            (runtime.stats.avg_execution_time + execution_time) / 2;
                        
                        debug!("Strategy '{}' executed successfully, generated {} signals", 
                               strategy_id, signals.len());
                    }
                    Err(e) => {
                        runtime.stats.error_count += 1;
                        runtime.last_error = Some(e.to_string());
                        runtime.state = StrategyExecutionState::Error;
                        
                        error!("Strategy '{}' execution failed: {:?}", strategy_id, e);
                    }
                }
            }
        }
    }
}

/// 引擎统计信息
#[derive(Debug, Clone)]
pub struct EngineStats {
    /// 引擎状态
    pub engine_state: EngineState,
    /// 总策略数
    pub total_strategies: usize,
    /// 运行中的策略数
    pub running_strategies: usize,
    /// 暂停的策略数
    pub paused_strategies: usize,
    /// 停止的策略数
    pub stopped_strategies: usize,
    /// 错误的策略数
    pub error_strategies: usize,
    /// 总处理信号数
    pub total_signals_processed: u64,
    /// 总生成订单数
    pub total_orders_generated: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 平均执行时间
    pub average_execution_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct MockStrategyPlugin {
        metadata: PluginMetadata,
        signal_count: Arc<AtomicU64>,
    }

    impl MockStrategyPlugin {
        fn new(strategy_id: &str) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: strategy_id.to_string(),
                    name: format!("Mock Strategy {}", strategy_id),
                    version: semver::Version::new(1, 0, 0),
                    description: "Mock strategy for testing".to_string(),
                    author: "Test".to_string(),
                    plugin_type: PluginType::Strategy,
                    capabilities: vec![PluginCapability::RealTimeData],
                    dependencies: vec![],
                    min_framework_version: semver::Version::new(2, 0, 0),
                    max_framework_version: None,
                    config_schema: None,
                    tags: vec![],
                },
                signal_count: Arc::new(AtomicU64::new(0)),
            }
        }
    }

    #[async_trait]
    impl Plugin for MockStrategyPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _context: &PluginContext) -> Result<()> {
            Ok(())
        }

        async fn start(&mut self, _context: &PluginContext) -> Result<()> {
            Ok(())
        }

        async fn stop(&mut self, _context: &PluginContext) -> Result<()> {
            Ok(())
        }

        fn state(&self) -> PluginState {
            PluginState::Running
        }

        async fn health_check(&self) -> Result<PluginHealthStatus> {
            Ok(PluginHealthStatus {
                healthy: true,
                message: "Mock strategy is healthy".to_string(),
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

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[async_trait]
    impl StrategyPlugin for MockStrategyPlugin {
        async fn on_market_data(&mut self, _data: &MarketData) -> Result<Vec<Signal>> {
            Ok(vec![])
        }

        async fn on_order_update(&mut self, _order: &Order) -> Result<()> {
            Ok(())
        }

        async fn on_trade(&mut self, _trade: &Trade) -> Result<()> {
            Ok(())
        }

        async fn get_positions(&self) -> Result<Vec<Position>> {
            Ok(vec![])
        }

        async fn generate_signals(&mut self, _context: &StrategyContext) -> Result<Vec<Signal>> {
            self.signal_count.fetch_add(1, Ordering::Relaxed);
            
            Ok(vec![Signal {
                id: "test_signal".to_string(),
                symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
                signal_type: SignalType::Buy,
                strength: 0.8,
                target_price: Some(rust_decimal::Decimal::from(50000)),
                stop_loss: None,
                take_profit: None,
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                source: self.metadata.id.clone(),
                metadata: HashMap::new(),
            }])
        }
    }

    #[tokio::test]
    async fn test_strategy_engine_creation() {
        let config = StrategyEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = StrategyEngine::new(config, registry, lifecycle, communication);
        
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.engine_state, EngineState::Stopped);
        assert_eq!(stats.total_strategies, 0);
    }

    #[tokio::test]
    async fn test_strategy_registration_and_lifecycle() {
        let config = StrategyEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = StrategyEngine::new(config, registry, lifecycle, communication);

        // 注册策略
        let strategy_plugin = Arc::new(Mutex::new(MockStrategyPlugin::new("test_strategy")));
        let config = HashMap::new();
        
        engine.register_strategy("test_strategy".to_string(), strategy_plugin, config).await.unwrap();

        // 检查策略状态
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Stopped));

        // 启动策略
        engine.start_strategy("test_strategy").await.unwrap();
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Running));

        // 暂停策略
        engine.pause_strategy("test_strategy").await.unwrap();
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Paused));

        // 恢复策略
        engine.resume_strategy("test_strategy").await.unwrap();
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Running));

        // 停止策略
        engine.stop_strategy("test_strategy").await.unwrap();
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Stopped));
    }

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let config = StrategyEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let mut engine = StrategyEngine::new(config, registry, lifecycle, communication);

        // 启动引擎
        engine.start_engine().await.unwrap();
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.engine_state, EngineState::Running);

        // 停止引擎
        engine.stop_engine().await.unwrap();
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.engine_state, EngineState::Stopped);
    }

    #[tokio::test]
    async fn test_market_data_processing() {
        let config = StrategyEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = StrategyEngine::new(config, registry, lifecycle, communication);

        // 注册并启动策略
        let strategy_plugin = Arc::new(Mutex::new(MockStrategyPlugin::new("test_strategy")));
        engine.register_strategy("test_strategy".to_string(), strategy_plugin.clone(), HashMap::new()).await.unwrap();
        engine.start_strategy("test_strategy").await.unwrap();

        // 创建测试市场数据
        let market_data = MarketData::Tick(Tick {
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            bid_price: rust_decimal::Decimal::from(50000),
            ask_price: rust_decimal::Decimal::from(50001),
            bid_size: rust_decimal::Decimal::from(10),
            ask_size: rust_decimal::Decimal::from(5),
            last_price: None,
            last_size: None,
        });

        // 处理市场数据
        engine.on_market_data(&market_data).await.unwrap();

        // 给异步处理一些时间
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 验证策略仍在运行
        let state = engine.get_strategy_state("test_strategy").await;
        assert_eq!(state, Some(StrategyExecutionState::Running));
    }
}