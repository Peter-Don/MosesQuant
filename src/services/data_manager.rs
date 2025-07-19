//! 可插拔数据管理器
//! 
//! 基于插件系统的高性能数据管理引擎，支持多数据源并发处理和实时数据流

use crate::plugins::*;
use crate::types::*;
use crate::{Result, MosesQuantError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, broadcast};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};

/// 数据管理器配置
#[derive(Debug, Clone)]
pub struct DataManagerConfig {
    /// 最大并发数据源数量
    pub max_concurrent_sources: usize,
    /// 数据缓存大小
    pub cache_size: usize,
    /// 数据过期时间
    pub data_expiry: Duration,
    /// 是否启用数据验证
    pub enable_data_validation: bool,
    /// 是否启用缓存
    pub enable_caching: bool,
    /// 最大批量大小
    pub max_batch_size: usize,
    /// 数据获取超时时间
    pub fetch_timeout: Duration,
    /// 重试次数
    pub max_retries: u32,
    /// 重试间隔
    pub retry_interval: Duration,
}

impl Default for DataManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sources: 20,
            cache_size: 10000,
            data_expiry: Duration::from_secs(300), // 5 minutes
            enable_data_validation: true,
            enable_caching: true,
            max_batch_size: 1000,
            fetch_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_interval: Duration::from_millis(500),
        }
    }
}

/// 数据源状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSourceState {
    /// 已断开
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
    /// 正在重连
    Reconnecting,
    /// 错误状态
    Error,
}

/// 数据源运行时信息
#[derive(Debug)]
pub struct DataSourceRuntime {
    /// 数据源插件
    pub plugin: Arc<Mutex<dyn DataSourcePlugin>>,
    /// 连接状态
    pub state: DataSourceState,
    /// 支持的资产列表
    pub supported_symbols: Vec<Symbol>,
    /// 连接统计
    pub stats: DataSourceStats,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
    /// 上下文
    pub context: DataSourceContext,
    /// 最后错误
    pub last_error: Option<String>,
    /// 重连尝试次数
    pub reconnect_attempts: u32,
}

/// 数据源统计信息
#[derive(Debug, Clone, Default)]
pub struct DataSourceStats {
    /// 连接时间
    pub connect_time: Option<Instant>,
    /// 总连接时长
    pub total_uptime: Duration,
    /// 接收的消息数
    pub messages_received: u64,
    /// 发送的请求数
    pub requests_sent: u64,
    /// 错误次数
    pub error_count: u64,
    /// 平均延迟
    pub avg_latency: Duration,
    /// 数据质量分数
    pub data_quality_score: f64,
    /// 最后活动时间
    pub last_activity: Option<Instant>,
}

/// 数据管理器
pub struct DataManager {
    /// 注册的数据源
    data_sources: Arc<RwLock<HashMap<DataSourceId, DataSourceRuntime>>>,
    /// 插件注册表
    plugin_registry: Arc<PluginRegistry>,
    /// 生命周期管理器
    lifecycle_manager: Arc<PluginLifecycleManager>,
    /// 通信管理器
    communication_manager: Arc<PluginCommunicationManager>,
    /// 数据管理器配置
    config: DataManagerConfig,
    /// 数据缓存
    data_cache: Arc<RwLock<HashMap<String, CachedMarketData>>>,
    /// 市场数据广播器
    market_data_sender: broadcast::Sender<MarketData>,
    /// 订阅管理
    subscriptions: Arc<RwLock<HashMap<Symbol, Vec<DataSourceId>>>>,
    /// 管理器状态
    manager_state: Arc<RwLock<ManagerState>>,
}

/// 管理器状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// 缓存的市场数据
#[derive(Debug, Clone)]
pub struct CachedMarketData {
    /// 市场数据
    pub data: MarketData,
    /// 缓存时间
    pub cached_at: Instant,
    /// 数据源ID
    pub source_id: DataSourceId,
    /// 访问次数
    pub access_count: u64,
}

/// 数据查询请求
#[derive(Debug, Clone)]
pub struct DataQuery {
    /// 资产符号
    pub symbol: Symbol,
    /// 查询类型
    pub query_type: DataQueryType,
    /// 时间范围
    pub time_range: Option<TimeRange>,
    /// 数据源偏好
    pub preferred_sources: Vec<DataSourceId>,
    /// 是否允许缓存数据
    pub allow_cached: bool,
    /// 超时时间
    pub timeout: Option<Duration>,
}

/// 数据查询类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataQueryType {
    /// 最新行情
    LatestTick,
    /// 最新K线
    LatestBar,
    /// 历史K线
    HistoricalBars,
    /// 历史Tick
    HistoricalTicks,
    /// 订单簿
    OrderBook,
    /// 成交记录
    Trades,
}

/// 时间范围
#[derive(Debug, Clone)]
pub struct TimeRange {
    /// 开始时间
    pub start: i64,
    /// 结束时间
    pub end: i64,
}

/// 数据查询结果
#[derive(Debug, Clone)]
pub struct DataQueryResult {
    /// 查询请求
    pub query: DataQuery,
    /// 查询结果
    pub data: Vec<MarketData>,
    /// 数据源
    pub source_id: DataSourceId,
    /// 查询耗时
    pub elapsed: Duration,
    /// 是否来自缓存
    pub from_cache: bool,
    /// 数据质量评分
    pub quality_score: f64,
}

impl DataManager {
    /// 创建新的数据管理器
    pub fn new(
        config: DataManagerConfig,
        plugin_registry: Arc<PluginRegistry>,
        lifecycle_manager: Arc<PluginLifecycleManager>,
        communication_manager: Arc<PluginCommunicationManager>,
    ) -> Self {
        let (market_data_sender, _) = broadcast::channel(1000);
        
        Self {
            data_sources: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry,
            lifecycle_manager,
            communication_manager,
            config,
            data_cache: Arc::new(RwLock::new(HashMap::new())),
            market_data_sender,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            manager_state: Arc::new(RwLock::new(ManagerState::Stopped)),
        }
    }

    /// 注册数据源插件
    pub async fn register_data_source(
        &self,
        source_id: DataSourceId,
        plugin: Arc<Mutex<dyn DataSourcePlugin>>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // 检查数据源数量限制
        {
            let sources = self.data_sources.read().await;
            if sources.len() >= self.config.max_concurrent_sources {
                return Err(MosesQuantError::Internal {
                    message: "Maximum number of data sources reached".to_string()
                });
            }
        }

        // 获取支持的资产列表
        let supported_symbols = {
            let plugin_guard = plugin.lock().await;
            plugin_guard.get_supported_symbols().await.unwrap_or_default()
        };

        // 创建数据源上下文
        let context = DataSourceContext {
            source_id: source_id.clone(),
            symbols: supported_symbols.clone(),
            config: config.clone(),
        };

        // 创建数据源运行时
        let runtime = DataSourceRuntime {
            plugin,
            state: DataSourceState::Disconnected,
            supported_symbols,
            stats: DataSourceStats::default(),
            config,
            context,
            last_error: None,
            reconnect_attempts: 0,
        };

        // 注册数据源
        {
            let mut sources = self.data_sources.write().await;
            sources.insert(source_id.clone(), runtime);
        }

        info!("Data source '{}' registered successfully", source_id);
        Ok(())
    }

    /// 连接数据源
    pub async fn connect_data_source(&self, source_id: &DataSourceId) -> Result<()> {
        let mut sources = self.data_sources.write().await;
        
        if let Some(runtime) = sources.get_mut(source_id) {
            match runtime.state {
                DataSourceState::Disconnected => {
                    runtime.state = DataSourceState::Connecting;
                    runtime.stats.connect_time = Some(Instant::now());
                    
                    // 创建插件上下文
                    let plugin_context = PluginContext::new(source_id.clone())
                        .with_config(runtime.config.clone());
                    
                    // 连接数据源
                    {
                        let mut plugin = runtime.plugin.lock().await;
                        plugin.initialize(&plugin_context).await?;
                        plugin.start(&plugin_context).await?;
                        plugin.connect(&runtime.context).await?;
                    }
                    
                    runtime.state = DataSourceState::Connected;
                    runtime.reconnect_attempts = 0;
                    info!("Data source '{}' connected successfully", source_id);
                    Ok(())
                }
                _ => {
                    Err(MosesQuantError::Internal {
                        message: format!("Data source '{}' is not in disconnected state", source_id)
                    })
                }
            }
        } else {
            Err(MosesQuantError::DataSourceNotFound { source_id: source_id.clone() })
        }
    }

    /// 断开数据源
    pub async fn disconnect_data_source(&self, source_id: &DataSourceId) -> Result<()> {
        let mut sources = self.data_sources.write().await;
        
        if let Some(runtime) = sources.get_mut(source_id) {
            if runtime.state == DataSourceState::Connected {
                let plugin_context = PluginContext::new(source_id.clone());
                
                {
                    let mut plugin = runtime.plugin.lock().await;
                    plugin.disconnect(&runtime.context).await?;
                    plugin.stop(&plugin_context).await?;
                }
                
                runtime.state = DataSourceState::Disconnected;
                
                // 更新连接统计
                if let Some(connect_time) = runtime.stats.connect_time {
                    runtime.stats.total_uptime += connect_time.elapsed();
                    runtime.stats.connect_time = None;
                }
                
                info!("Data source '{}' disconnected successfully", source_id);
                Ok(())
            } else {
                Err(MosesQuantError::Internal {
                    message: format!("Data source '{}' is not connected", source_id)
                })
            }
        } else {
            Err(MosesQuantError::DataSourceNotFound { source_id: source_id.clone() })
        }
    }

    /// 订阅市场数据
    pub async fn subscribe_market_data(&self, symbol: Symbol, source_id: DataSourceId) -> Result<()> {
        // 检查数据源是否已连接
        let is_connected = {
            let sources = self.data_sources.read().await;
            sources.get(&source_id)
                .map(|runtime| runtime.state == DataSourceState::Connected)
                .unwrap_or(false)
        };

        if !is_connected {
            return Err(MosesQuantError::Internal {
                message: format!("Data source '{}' is not connected", source_id)
            });
        }

        // 添加订阅
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.entry(symbol.clone())
                .or_insert_with(Vec::new)
                .push(source_id.clone());
        }

        // 调用数据源订阅
        {
            let sources = self.data_sources.read().await;
            if let Some(runtime) = sources.get(&source_id) {
                let mut plugin = runtime.plugin.lock().await;
                plugin.subscribe_market_data(&symbol).await?;
            }
        }

        info!("Subscribed to market data for {} from {}", symbol.symbol, source_id);
        Ok(())
    }

    /// 取消订阅市场数据
    pub async fn unsubscribe_market_data(&self, symbol: &Symbol, source_id: &DataSourceId) -> Result<()> {
        // 移除订阅
        {
            let mut subscriptions = self.subscriptions.write().await;
            if let Some(sources) = subscriptions.get_mut(symbol) {
                sources.retain(|id| id != source_id);
                if sources.is_empty() {
                    subscriptions.remove(symbol);
                }
            }
        }

        // 调用数据源取消订阅
        {
            let sources = self.data_sources.read().await;
            if let Some(runtime) = sources.get(source_id) {
                let mut plugin = runtime.plugin.lock().await;
                plugin.unsubscribe_market_data(symbol).await?;
            }
        }

        info!("Unsubscribed from market data for {} from {}", symbol.symbol, source_id);
        Ok(())
    }

    /// 查询市场数据
    pub async fn query_market_data(&self, query: DataQuery) -> Result<DataQueryResult> {
        let start_time = Instant::now();

        // 优先检查缓存
        if query.allow_cached && self.config.enable_caching {
            if let Some(cached_result) = self.get_cached_data(&query).await {
                return Ok(DataQueryResult {
                    query,
                    data: vec![cached_result.data],
                    source_id: cached_result.source_id,
                    elapsed: start_time.elapsed(),
                    from_cache: true,
                    quality_score: 1.0,
                });
            }
        }

        // 选择最佳数据源
        let source_id = self.select_best_data_source(&query).await?;

        // 从数据源获取数据
        let data = self.fetch_data_from_source(&source_id, &query).await?;

        // 缓存数据
        if self.config.enable_caching && !data.is_empty() {
            self.cache_data(&query.symbol, &data[0], &source_id).await;
        }

        // 计算质量评分
        let quality_score = self.calculate_data_quality(&data, &source_id).await;

        Ok(DataQueryResult {
            query,
            data,
            source_id,
            elapsed: start_time.elapsed(),
            from_cache: false,
            quality_score,
        })
    }

    /// 获取市场数据订阅器
    pub fn subscribe_market_data_stream(&self) -> broadcast::Receiver<MarketData> {
        self.market_data_sender.subscribe()
    }

    /// 处理接收到的市场数据
    pub async fn on_market_data(&self, data: MarketData, source_id: DataSourceId) -> Result<()> {
        // 更新数据源统计
        {
            let mut sources = self.data_sources.write().await;
            if let Some(runtime) = sources.get_mut(&source_id) {
                runtime.stats.messages_received += 1;
                runtime.stats.last_activity = Some(Instant::now());
            }
        }

        // 验证数据
        if self.config.enable_data_validation {
            self.validate_market_data(&data)?;
        }

        // 缓存数据
        if self.config.enable_caching {
            let symbol = data.symbol();
            self.cache_data(&symbol, &data, &source_id).await;
        }

        // 广播数据
        if let Err(_) = self.market_data_sender.send(data) {
            warn!("No subscribers for market data broadcast");
        }

        Ok(())
    }

    /// 获取数据源状态
    pub async fn get_data_source_state(&self, source_id: &DataSourceId) -> Option<DataSourceState> {
        let sources = self.data_sources.read().await;
        sources.get(source_id).map(|runtime| runtime.state.clone())
    }

    /// 获取数据源统计信息
    pub async fn get_data_source_stats(&self, source_id: &DataSourceId) -> Option<DataSourceStats> {
        let sources = self.data_sources.read().await;
        sources.get(source_id).map(|runtime| runtime.stats.clone())
    }

    /// 获取所有数据源状态
    pub async fn get_all_data_sources_status(&self) -> HashMap<DataSourceId, DataSourceState> {
        let sources = self.data_sources.read().await;
        sources.iter()
            .map(|(id, runtime)| (id.clone(), runtime.state.clone()))
            .collect()
    }

    /// 获取管理器统计信息
    pub async fn get_manager_stats(&self) -> DataManagerStats {
        let sources = self.data_sources.read().await;
        let manager_state = self.manager_state.read().await;
        let cache = self.data_cache.read().await;
        let subscriptions = self.subscriptions.read().await;

        let mut stats = DataManagerStats {
            manager_state: manager_state.clone(),
            total_sources: sources.len(),
            connected_sources: 0,
            disconnected_sources: 0,
            error_sources: 0,
            total_messages_received: 0,
            total_requests_sent: 0,
            total_errors: 0,
            cache_size: cache.len(),
            total_subscriptions: subscriptions.len(),
            average_latency: Duration::ZERO,
        };

        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0;

        for runtime in sources.values() {
            match runtime.state {
                DataSourceState::Connected => stats.connected_sources += 1,
                DataSourceState::Disconnected => stats.disconnected_sources += 1,
                DataSourceState::Error => stats.error_sources += 1,
                _ => {}
            }

            stats.total_messages_received += runtime.stats.messages_received;
            stats.total_requests_sent += runtime.stats.requests_sent;
            stats.total_errors += runtime.stats.error_count;
            
            total_latency += runtime.stats.avg_latency;
            latency_count += 1;
        }

        if latency_count > 0 {
            stats.average_latency = total_latency / latency_count as u32;
        }

        stats
    }

    /// 启动数据管理器
    pub async fn start_manager(&self) -> Result<()> {
        {
            let mut state = self.manager_state.write().await;
            if *state != ManagerState::Stopped {
                return Err(MosesQuantError::Internal {
                    message: "Data manager is not in stopped state".to_string()
                });
            }
            *state = ManagerState::Starting;
        }

        // 启动所有数据源
        let source_ids: Vec<DataSourceId> = {
            let sources = self.data_sources.read().await;
            sources.keys().cloned().collect()
        };

        for source_id in source_ids {
            if let Err(e) = self.connect_data_source(&source_id).await {
                warn!("Failed to connect data source '{}': {:?}", source_id, e);
            }
        }

        {
            let mut state = self.manager_state.write().await;
            *state = ManagerState::Running;
        }

        info!("Data manager started successfully");
        Ok(())
    }

    /// 停止数据管理器
    pub async fn stop_manager(&self) -> Result<()> {
        {
            let mut state = self.manager_state.write().await;
            if *state != ManagerState::Running {
                return Err(MosesQuantError::Internal {
                    message: "Data manager is not in running state".to_string()
                });
            }
            *state = ManagerState::Stopping;
        }

        // 停止所有数据源
        let source_ids: Vec<DataSourceId> = {
            let sources = self.data_sources.read().await;
            sources.keys().cloned().collect()
        };

        for source_id in source_ids {
            if let Err(e) = self.disconnect_data_source(&source_id).await {
                warn!("Failed to disconnect data source '{}': {:?}", source_id, e);
            }
        }

        // 清理缓存
        {
            let mut cache = self.data_cache.write().await;
            cache.clear();
        }

        {
            let mut state = self.manager_state.write().await;
            *state = ManagerState::Stopped;
        }

        info!("Data manager stopped successfully");
        Ok(())
    }

    // 私有方法

    /// 获取缓存数据
    async fn get_cached_data(&self, query: &DataQuery) -> Option<CachedMarketData> {
        let cache = self.data_cache.read().await;
        let cache_key = self.generate_cache_key(&query.symbol, &query.query_type);
        
        if let Some(cached) = cache.get(&cache_key) {
            // 检查数据是否过期
            if cached.cached_at.elapsed() < self.config.data_expiry {
                return Some(cached.clone());
            }
        }
        
        None
    }

    /// 选择最佳数据源
    async fn select_best_data_source(&self, query: &DataQuery) -> Result<DataSourceId> {
        let sources = self.data_sources.read().await;

        // 优先使用指定的数据源
        for preferred_source in &query.preferred_sources {
            if let Some(runtime) = sources.get(preferred_source) {
                if runtime.state == DataSourceState::Connected &&
                   runtime.supported_symbols.contains(&query.symbol) {
                    return Ok(preferred_source.clone());
                }
            }
        }

        // 选择最佳数据源
        let mut best_source = None;
        let mut best_score = 0.0;

        for (source_id, runtime) in sources.iter() {
            if runtime.state == DataSourceState::Connected &&
               runtime.supported_symbols.contains(&query.symbol) {
                
                let score = self.calculate_source_score(runtime);
                if score > best_score {
                    best_score = score;
                    best_source = Some(source_id.clone());
                }
            }
        }

        best_source.ok_or_else(|| MosesQuantError::Internal {
            message: format!("No available data source for symbol {}", query.symbol.symbol)
        })
    }

    /// 计算数据源评分
    fn calculate_source_score(&self, runtime: &DataSourceRuntime) -> f64 {
        let mut score = runtime.stats.data_quality_score;
        
        // 根据延迟调整评分
        let latency_penalty = runtime.stats.avg_latency.as_millis() as f64 / 1000.0;
        score -= latency_penalty * 0.1;
        
        // 根据错误率调整评分
        if runtime.stats.messages_received > 0 {
            let error_rate = runtime.stats.error_count as f64 / runtime.stats.messages_received as f64;
            score -= error_rate * 0.5;
        }
        
        score.max(0.0).min(1.0)
    }

    /// 从数据源获取数据
    async fn fetch_data_from_source(&self, source_id: &DataSourceId, query: &DataQuery) -> Result<Vec<MarketData>> {
        let sources = self.data_sources.read().await;
        let runtime = sources.get(source_id)
            .ok_or_else(|| MosesQuantError::DataSourceNotFound { source_id: source_id.clone() })?;

        let timeout = query.timeout.unwrap_or(self.config.fetch_timeout);
        
        let result = tokio::time::timeout(timeout, async {
            let mut plugin = runtime.plugin.lock().await;
            match query.query_type {
                DataQueryType::LatestTick => {
                    plugin.get_latest_tick(&query.symbol).await.map(|tick| vec![MarketData::Tick(tick)])
                }
                DataQueryType::LatestBar => {
                    plugin.get_latest_bar(&query.symbol, None).await.map(|bar| vec![MarketData::Bar(bar)])
                }
                DataQueryType::HistoricalBars => {
                    if let Some(ref time_range) = query.time_range {
                        plugin.get_historical_bars(&query.symbol, time_range.start, time_range.end, None).await
                            .map(|bars| bars.into_iter().map(MarketData::Bar).collect())
                    } else {
                        Ok(vec![])
                    }
                }
                _ => Ok(vec![])
            }
        }).await;

        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(MosesQuantError::Internal {
                message: format!("Data fetch timeout from source {}", source_id)
            })
        }
    }

    /// 缓存数据
    async fn cache_data(&self, symbol: &Symbol, data: &MarketData, source_id: &DataSourceId) {
        let cache_key = self.generate_cache_key(symbol, &DataQueryType::LatestTick);
        let cached_data = CachedMarketData {
            data: data.clone(),
            cached_at: Instant::now(),
            source_id: source_id.clone(),
            access_count: 0,
        };

        let mut cache = self.data_cache.write().await;
        
        // 检查缓存大小限制
        if cache.len() >= self.config.cache_size {
            // 移除最旧的数据
            let oldest_key = cache.iter()
                .min_by_key(|(_, cached)| cached.cached_at)
                .map(|(key, _)| key.clone());
            
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        cache.insert(cache_key, cached_data);
    }

    /// 生成缓存键
    fn generate_cache_key(&self, symbol: &Symbol, query_type: &DataQueryType) -> String {
        format!("{}:{}:{:?}", symbol.symbol, symbol.exchange, query_type)
    }

    /// 验证市场数据
    fn validate_market_data(&self, data: &MarketData) -> Result<()> {
        match data {
            MarketData::Tick(tick) => {
                if tick.bid_price <= rust_decimal::Decimal::ZERO || tick.ask_price <= rust_decimal::Decimal::ZERO {
                    return Err(MosesQuantError::DataValidation {
                        message: "Invalid tick prices".to_string()
                    });
                }
                if tick.bid_price >= tick.ask_price {
                    return Err(MosesQuantError::DataValidation {
                        message: "Bid price should be less than ask price".to_string()
                    });
                }
            }
            MarketData::Bar(bar) => {
                if bar.open <= rust_decimal::Decimal::ZERO || bar.high <= rust_decimal::Decimal::ZERO ||
                   bar.low <= rust_decimal::Decimal::ZERO || bar.close <= rust_decimal::Decimal::ZERO {
                    return Err(MosesQuantError::DataValidation {
                        message: "Invalid bar prices".to_string()
                    });
                }
                if bar.low > bar.high {
                    return Err(MosesQuantError::DataValidation {
                        message: "Low price should not be greater than high price".to_string()
                    });
                }
            }
        }
        Ok(())
    }

    /// 计算数据质量
    async fn calculate_data_quality(&self, data: &[MarketData], source_id: &DataSourceId) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let sources = self.data_sources.read().await;
        if let Some(runtime) = sources.get(source_id) {
            runtime.stats.data_quality_score
        } else {
            0.5 // 默认质量分数
        }
    }
}

/// 数据管理器统计信息
#[derive(Debug, Clone)]
pub struct DataManagerStats {
    /// 管理器状态
    pub manager_state: ManagerState,
    /// 总数据源数量
    pub total_sources: usize,
    /// 已连接数据源数量
    pub connected_sources: usize,
    /// 已断开数据源数量
    pub disconnected_sources: usize,
    /// 错误数据源数量
    pub error_sources: usize,
    /// 总接收消息数
    pub total_messages_received: u64,
    /// 总发送请求数
    pub total_requests_sent: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 缓存大小
    pub cache_size: usize,
    /// 总订阅数
    pub total_subscriptions: usize,
    /// 平均延迟
    pub average_latency: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct MockDataSourcePlugin {
        metadata: PluginMetadata,
        message_count: Arc<AtomicU64>,
        symbols: Vec<Symbol>,
    }

    impl MockDataSourcePlugin {
        fn new(source_id: &str) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: source_id.to_string(),
                    name: format!("Mock Data Source {}", source_id),
                    version: semver::Version::new(1, 0, 0),
                    description: "Mock data source for testing".to_string(),
                    author: "Test".to_string(),
                    plugin_type: PluginType::DataSource,
                    capabilities: vec![PluginCapability::RealTimeData, PluginCapability::HistoricalData],
                    dependencies: vec![],
                    min_framework_version: semver::Version::new(2, 0, 0),
                    max_framework_version: None,
                    config_schema: None,
                    tags: vec![],
                },
                message_count: Arc::new(AtomicU64::new(0)),
                symbols: vec![
                    Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
                    Symbol::new("ETHUSDT", "BINANCE", AssetType::Crypto),
                ],
            }
        }
    }

    #[async_trait]
    impl Plugin for MockDataSourcePlugin {
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
                message: "Mock data source is healthy".to_string(),
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
    impl DataSourcePlugin for MockDataSourcePlugin {
        async fn connect(&mut self, _context: &DataSourceContext) -> Result<()> {
            Ok(())
        }

        async fn disconnect(&mut self, _context: &DataSourceContext) -> Result<()> {
            Ok(())
        }

        async fn get_supported_symbols(&self) -> Result<Vec<Symbol>> {
            Ok(self.symbols.clone())
        }

        async fn subscribe_market_data(&mut self, _symbol: &Symbol) -> Result<()> {
            Ok(())
        }

        async fn unsubscribe_market_data(&mut self, _symbol: &Symbol) -> Result<()> {
            Ok(())
        }

        async fn get_latest_tick(&mut self, symbol: &Symbol) -> Result<Tick> {
            self.message_count.fetch_add(1, Ordering::Relaxed);
            
            Ok(Tick {
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                bid_price: rust_decimal::Decimal::from(50000),
                ask_price: rust_decimal::Decimal::from(50001),
                bid_size: rust_decimal::Decimal::from(10),
                ask_size: rust_decimal::Decimal::from(5),
                last_price: Some(rust_decimal::Decimal::from(50000)),
                last_size: Some(rust_decimal::Decimal::from(1)),
            })
        }

        async fn get_latest_bar(&mut self, symbol: &Symbol, _timeframe: Option<String>) -> Result<Bar> {
            self.message_count.fetch_add(1, Ordering::Relaxed);
            
            Ok(Bar {
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                open: rust_decimal::Decimal::from(49000),
                high: rust_decimal::Decimal::from(51000),
                low: rust_decimal::Decimal::from(48000),
                close: rust_decimal::Decimal::from(50000),
                volume: rust_decimal::Decimal::from(1000),
            })
        }

        async fn get_historical_bars(
            &mut self, 
            symbol: &Symbol, 
            _start: i64, 
            _end: i64, 
            _timeframe: Option<String>
        ) -> Result<Vec<Bar>> {
            self.message_count.fetch_add(1, Ordering::Relaxed);
            
            Ok(vec![Bar {
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                open: rust_decimal::Decimal::from(49000),
                high: rust_decimal::Decimal::from(51000),
                low: rust_decimal::Decimal::from(48000),
                close: rust_decimal::Decimal::from(50000),
                volume: rust_decimal::Decimal::from(1000),
            }])
        }
    }

    #[tokio::test]
    async fn test_data_manager_creation() {
        let config = DataManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = DataManager::new(config, registry, lifecycle, communication);
        
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.manager_state, ManagerState::Stopped);
        assert_eq!(stats.total_sources, 0);
    }

    #[tokio::test]
    async fn test_data_source_registration_and_connection() {
        let config = DataManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = DataManager::new(config, registry, lifecycle, communication);

        // 注册数据源
        let data_source_plugin = Arc::new(Mutex::new(MockDataSourcePlugin::new("test_source")));
        let config = HashMap::new();
        
        manager.register_data_source("test_source".to_string(), data_source_plugin, config).await.unwrap();

        // 检查数据源状态
        let state = manager.get_data_source_state("test_source").await;
        assert_eq!(state, Some(DataSourceState::Disconnected));

        // 连接数据源
        manager.connect_data_source("test_source").await.unwrap();
        let state = manager.get_data_source_state("test_source").await;
        assert_eq!(state, Some(DataSourceState::Connected));

        // 断开数据源
        manager.disconnect_data_source("test_source").await.unwrap();
        let state = manager.get_data_source_state("test_source").await;
        assert_eq!(state, Some(DataSourceState::Disconnected));
    }

    #[tokio::test]
    async fn test_market_data_subscription() {
        let config = DataManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = DataManager::new(config, registry, lifecycle, communication);

        // 注册并连接数据源
        let data_source_plugin = Arc::new(Mutex::new(MockDataSourcePlugin::new("test_source")));
        manager.register_data_source("test_source".to_string(), data_source_plugin, HashMap::new()).await.unwrap();
        manager.connect_data_source("test_source").await.unwrap();

        // 订阅市场数据
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        manager.subscribe_market_data(symbol.clone(), "test_source".to_string()).await.unwrap();

        // 取消订阅
        manager.unsubscribe_market_data(&symbol, "test_source").await.unwrap();
    }

    #[tokio::test]
    async fn test_data_query() {
        let config = DataManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = DataManager::new(config, registry, lifecycle, communication);

        // 注册并连接数据源
        let data_source_plugin = Arc::new(Mutex::new(MockDataSourcePlugin::new("test_source")));
        manager.register_data_source("test_source".to_string(), data_source_plugin, HashMap::new()).await.unwrap();
        manager.connect_data_source("test_source").await.unwrap();

        // 查询最新Tick数据
        let query = DataQuery {
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            query_type: DataQueryType::LatestTick,
            time_range: None,
            preferred_sources: vec!["test_source".to_string()],
            allow_cached: true,
            timeout: None,
        };

        let result = manager.query_market_data(query).await.unwrap();
        assert!(!result.data.is_empty());
        assert_eq!(result.source_id, "test_source");
        assert!(!result.from_cache);
    }

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let config = DataManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = DataManager::new(config, registry, lifecycle, communication);

        // 启动管理器
        manager.start_manager().await.unwrap();
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.manager_state, ManagerState::Running);

        // 停止管理器
        manager.stop_manager().await.unwrap();
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.manager_state, ManagerState::Stopped);
    }
}