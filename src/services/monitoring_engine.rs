//! 可插拔监控引擎
//! 
//! 基于插件系统的高性能监控和指标收集引擎，支持多维度监控和实时告警

use crate::plugins::*;
use crate::types::*;
use crate::{Result, MosesQuantError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, broadcast, mpsc};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 监控引擎配置
#[derive(Debug, Clone)]
pub struct MonitoringEngineConfig {
    /// 最大并发监控插件数量
    pub max_concurrent_monitors: usize,
    /// 指标收集间隔
    pub metrics_collection_interval: Duration,
    /// 是否启用实时监控
    pub enable_realtime_monitoring: bool,
    /// 告警检查间隔
    pub alert_check_interval: Duration,
    /// 指标缓存大小
    pub metrics_cache_size: usize,
    /// 指标保留时间
    pub metrics_retention_period: Duration,
    /// 批量处理大小
    pub batch_size: usize,
    /// 告警队列大小
    pub alert_queue_size: usize,
    /// 是否启用性能监控
    pub enable_performance_monitoring: bool,
    /// 健康检查间隔
    pub health_check_interval: Duration,
}

impl Default for MonitoringEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_monitors: 50,
            metrics_collection_interval: Duration::from_secs(10),
            enable_realtime_monitoring: true,
            alert_check_interval: Duration::from_secs(5),
            metrics_cache_size: 100000,
            metrics_retention_period: Duration::from_secs(86400), // 24小时
            batch_size: 1000,
            alert_queue_size: 10000,
            enable_performance_monitoring: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// 指标类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// 计数器
    Counter,
    /// 仪表盘
    Gauge,
    /// 直方图
    Histogram,
    /// 摘要
    Summary,
    /// 集合
    Set,
    /// 自定义类型
    Custom(String),
}

/// 指标数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标值
    pub value: MetricValue,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 时间戳
    pub timestamp: i64,
    /// 来源
    pub source: String,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 指标值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// 整数值
    Integer(i64),
    /// 浮点值
    Float(f64),
    /// 十进制值
    Decimal(Decimal),
    /// 字符串值
    String(String),
    /// 布尔值
    Boolean(bool),
    /// 分布值（用于直方图）
    Distribution(Vec<f64>),
    /// 百分位数值
    Percentiles(HashMap<String, f64>),
}

/// 告警级别
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// 告警规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 监控指标
    pub metric_name: String,
    /// 告警条件
    pub condition: AlertCondition,
    /// 告警级别
    pub level: AlertLevel,
    /// 评估间隔
    pub evaluation_interval: Duration,
    /// 持续时间阈值
    pub for_duration: Duration,
    /// 告警标签
    pub labels: HashMap<String, String>,
    /// 告警注解
    pub annotations: HashMap<String, String>,
    /// 是否启用
    pub enabled: bool,
}

/// 告警条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// 大于阈值
    GreaterThan(f64),
    /// 小于阈值
    LessThan(f64),
    /// 等于值
    Equals(f64),
    /// 在范围内
    InRange(f64, f64),
    /// 在范围外
    OutOfRange(f64, f64),
    /// 变化率
    ChangeRate(f64),
    /// 自定义条件
    Custom(String),
}

/// 告警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    /// 事件ID
    pub id: String,
    /// 规则ID
    pub rule_id: String,
    /// 告警级别
    pub level: AlertLevel,
    /// 告警状态
    pub state: AlertState,
    /// 触发时间
    pub fired_at: i64,
    /// 解决时间
    pub resolved_at: Option<i64>,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 告警消息
    pub message: String,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 注解
    pub annotations: HashMap<String, String>,
}

/// 告警状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertState {
    /// 触发中
    Firing,
    /// 已解决
    Resolved,
    /// 已确认
    Acknowledged,
    /// 已静默
    Silenced,
}

/// 监控插件运行时信息
#[derive(Debug)]
pub struct MonitorRuntime {
    /// 监控插件
    pub plugin: Arc<Mutex<dyn MonitoringPlugin>>,
    /// 监控状态
    pub state: MonitorState,
    /// 监控统计
    pub stats: MonitorStats,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
    /// 上次收集时间
    pub last_collection_time: Option<Instant>,
    /// 最后错误
    pub last_error: Option<String>,
}

/// 监控状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorState {
    /// 未启动
    Stopped,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 错误状态
    Error,
}

/// 监控统计信息
#[derive(Debug, Clone, Default)]
pub struct MonitorStats {
    /// 收集的指标数量
    pub metrics_collected: u64,
    /// 生成的告警数量
    pub alerts_generated: u64,
    /// 错误次数
    pub error_count: u64,
    /// 平均收集时间
    pub avg_collection_time: Duration,
    /// 最后收集时间
    pub last_collection_time: Option<Instant>,
    /// 数据质量分数
    pub data_quality_score: f64,
}

/// 监控引擎
pub struct MonitoringEngine {
    /// 注册的监控插件
    monitors: Arc<RwLock<HashMap<String, MonitorRuntime>>>,
    /// 插件注册表
    plugin_registry: Arc<PluginRegistry>,
    /// 生命周期管理器
    lifecycle_manager: Arc<PluginLifecycleManager>,
    /// 通信管理器
    communication_manager: Arc<PluginCommunicationManager>,
    /// 监控引擎配置
    config: MonitoringEngineConfig,
    /// 指标缓存
    metrics_cache: Arc<RwLock<Vec<MetricDataPoint>>>,
    /// 告警规则
    alert_rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    /// 活跃告警
    active_alerts: Arc<RwLock<HashMap<String, AlertEvent>>>,
    /// 指标数据广播器
    metrics_sender: broadcast::Sender<MetricDataPoint>,
    /// 告警事件广播器
    alert_sender: broadcast::Sender<AlertEvent>,
    /// 监控任务句柄
    monitoring_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// 引擎状态
    engine_state: Arc<RwLock<MonitoringEngineState>>,
}

/// 监控引擎状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitoringEngineState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// 监控插件接口
#[async_trait]
pub trait MonitoringPlugin: Plugin {
    /// 收集指标
    async fn collect_metrics(&mut self) -> Result<Vec<MetricDataPoint>>;
    
    /// 获取健康状态
    async fn get_health_status(&self) -> Result<HealthStatus>;
    
    /// 获取支持的指标类型
    fn get_supported_metrics(&self) -> Vec<String>;
    
    /// 配置监控参数
    async fn configure_monitoring(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()>;
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// 是否健康
    pub healthy: bool,
    /// 健康分数 (0.0 - 1.0)
    pub score: f64,
    /// 状态消息
    pub message: String,
    /// 检查时间
    pub checked_at: i64,
    /// 详细信息
    pub details: HashMap<String, serde_json::Value>,
}

impl MonitoringEngine {
    /// 创建新的监控引擎
    pub fn new(
        config: MonitoringEngineConfig,
        plugin_registry: Arc<PluginRegistry>,
        lifecycle_manager: Arc<PluginLifecycleManager>,
        communication_manager: Arc<PluginCommunicationManager>,
    ) -> Self {
        let (metrics_sender, _) = broadcast::channel(10000);
        let (alert_sender, _) = broadcast::channel(1000);
        
        Self {
            monitors: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry,
            lifecycle_manager,
            communication_manager,
            config,
            metrics_cache: Arc::new(RwLock::new(Vec::new())),
            alert_rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            metrics_sender,
            alert_sender,
            monitoring_handles: Arc::new(RwLock::new(Vec::new())),
            engine_state: Arc::new(RwLock::new(MonitoringEngineState::Stopped)),
        }
    }

    /// 注册监控插件
    pub async fn register_monitor(
        &self,
        monitor_id: String,
        plugin: Arc<Mutex<dyn MonitoringPlugin>>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // 检查监控插件数量限制
        {
            let monitors = self.monitors.read().await;
            if monitors.len() >= self.config.max_concurrent_monitors {
                return Err(MosesQuantError::Internal {
                    message: "Maximum number of monitors reached".to_string()
                });
            }
        }

        // 创建监控运行时
        let runtime = MonitorRuntime {
            plugin,
            state: MonitorState::Stopped,
            stats: MonitorStats::default(),
            config,
            last_collection_time: None,
            last_error: None,
        };

        // 注册监控插件
        {
            let mut monitors = self.monitors.write().await;
            monitors.insert(monitor_id.clone(), runtime);
        }

        info!("Monitor '{}' registered successfully", monitor_id);
        Ok(())
    }

    /// 启动监控插件
    pub async fn start_monitor(&self, monitor_id: &str) -> Result<()> {
        let mut monitors = self.monitors.write().await;
        
        if let Some(runtime) = monitors.get_mut(monitor_id) {
            match runtime.state {
                MonitorState::Stopped => {
                    // 创建插件上下文
                    let plugin_context = PluginContext::new(monitor_id.to_string())
                        .with_config(runtime.config.clone());
                    
                    // 启动插件
                    {
                        let mut plugin = runtime.plugin.lock().await;
                        plugin.initialize(&plugin_context).await?;
                        plugin.start(&plugin_context).await?;
                        plugin.configure_monitoring(runtime.config.clone()).await?;
                    }
                    
                    runtime.state = MonitorState::Running;
                    runtime.last_collection_time = Some(Instant::now());
                    
                    info!("Monitor '{}' started successfully", monitor_id);
                    Ok(())
                }
                _ => {
                    Err(MosesQuantError::Internal {
                        message: format!("Monitor '{}' is not in stopped state", monitor_id)
                    })
                }
            }
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Monitor '{}' not found", monitor_id)
            })
        }
    }

    /// 停止监控插件
    pub async fn stop_monitor(&self, monitor_id: &str) -> Result<()> {
        let mut monitors = self.monitors.write().await;
        
        if let Some(runtime) = monitors.get_mut(monitor_id) {
            if runtime.state == MonitorState::Running {
                let plugin_context = PluginContext::new(monitor_id.to_string());
                
                {
                    let mut plugin = runtime.plugin.lock().await;
                    plugin.stop(&plugin_context).await?;
                }
                
                runtime.state = MonitorState::Stopped;
                info!("Monitor '{}' stopped successfully", monitor_id);
                Ok(())
            } else {
                Err(MosesQuantError::Internal {
                    message: format!("Monitor '{}' is not running", monitor_id)
                })
            }
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Monitor '{}' not found", monitor_id)
            })
        }
    }

    /// 添加告警规则
    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<()> {
        let mut rules = self.alert_rules.write().await;
        rules.insert(rule.id.clone(), rule.clone());
        
        info!("Alert rule '{}' added: {}", rule.id, rule.name);
        Ok(())
    }

    /// 删除告警规则
    pub async fn remove_alert_rule(&self, rule_id: &str) -> Result<()> {
        let mut rules = self.alert_rules.write().await;
        
        if rules.remove(rule_id).is_some() {
            info!("Alert rule '{}' removed", rule_id);
            Ok(())
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Alert rule '{}' not found", rule_id)
            })
        }
    }

    /// 获取指标数据订阅器
    pub fn subscribe_metrics(&self) -> broadcast::Receiver<MetricDataPoint> {
        self.metrics_sender.subscribe()
    }

    /// 获取告警事件订阅器
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<AlertEvent> {
        self.alert_sender.subscribe()
    }

    /// 查询历史指标
    pub async fn query_metrics(
        &self,
        metric_name: &str,
        start_time: i64,
        end_time: i64,
        labels: Option<HashMap<String, String>>,
    ) -> Result<Vec<MetricDataPoint>> {
        let cache = self.metrics_cache.read().await;
        
        let filtered_metrics: Vec<MetricDataPoint> = cache.iter()
            .filter(|metric| {
                metric.name == metric_name &&
                metric.timestamp >= start_time &&
                metric.timestamp <= end_time &&
                labels.as_ref().map_or(true, |required_labels| {
                    required_labels.iter().all(|(key, value)| {
                        metric.labels.get(key) == Some(value)
                    })
                })
            })
            .cloned()
            .collect();

        Ok(filtered_metrics)
    }

    /// 获取活跃告警
    pub async fn get_active_alerts(&self) -> Vec<AlertEvent> {
        let alerts = self.active_alerts.read().await;
        alerts.values()
            .filter(|alert| alert.state == AlertState::Firing)
            .cloned()
            .collect()
    }

    /// 确认告警
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.active_alerts.write().await;
        
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.state = AlertState::Acknowledged;
            info!("Alert '{}' acknowledged", alert_id);
            Ok(())
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Alert '{}' not found", alert_id)
            })
        }
    }

    /// 启动监控引擎
    pub async fn start_engine(&self) -> Result<()> {
        {
            let mut state = self.engine_state.write().await;
            if *state != MonitoringEngineState::Stopped {
                return Err(MosesQuantError::Internal {
                    message: "Monitoring engine is not in stopped state".to_string()
                });
            }
            *state = MonitoringEngineState::Starting;
        }

        // 启动所有监控插件
        let monitor_ids: Vec<String> = {
            let monitors = self.monitors.read().await;
            monitors.keys().cloned().collect()
        };

        for monitor_id in monitor_ids {
            if let Err(e) = self.start_monitor(&monitor_id).await {
                warn!("Failed to start monitor '{}': {:?}", monitor_id, e);
            }
        }

        // 启动指标收集循环
        self.start_metrics_collection_loop().await;

        // 启动告警检查循环
        self.start_alert_checking_loop().await;

        // 启动健康检查循环
        if self.config.enable_performance_monitoring {
            self.start_health_check_loop().await;
        }

        {
            let mut state = self.engine_state.write().await;
            *state = MonitoringEngineState::Running;
        }

        info!("Monitoring engine started successfully");
        Ok(())
    }

    /// 停止监控引擎
    pub async fn stop_engine(&self) -> Result<()> {
        {
            let mut state = self.engine_state.write().await;
            if *state != MonitoringEngineState::Running {
                return Err(MosesQuantError::Internal {
                    message: "Monitoring engine is not in running state".to_string()
                });
            }
            *state = MonitoringEngineState::Stopping;
        }

        // 停止所有监控任务
        {
            let mut handles = self.monitoring_handles.write().await;
            for handle in handles.drain(..) {
                handle.abort();
            }
        }

        // 停止所有监控插件
        let monitor_ids: Vec<String> = {
            let monitors = self.monitors.read().await;
            monitors.keys().cloned().collect()
        };

        for monitor_id in monitor_ids {
            if let Err(e) = self.stop_monitor(&monitor_id).await {
                warn!("Failed to stop monitor '{}': {:?}", monitor_id, e);
            }
        }

        {
            let mut state = self.engine_state.write().await;
            *state = MonitoringEngineState::Stopped;
        }

        info!("Monitoring engine stopped successfully");
        Ok(())
    }

    /// 获取监控引擎统计信息
    pub async fn get_engine_stats(&self) -> MonitoringEngineStats {
        let monitors = self.monitors.read().await;
        let alerts = self.active_alerts.read().await;
        let rules = self.alert_rules.read().await;
        let cache = self.metrics_cache.read().await;
        let state = self.engine_state.read().await;

        let mut total_metrics_collected = 0;
        let mut total_alerts_generated = 0;
        let mut total_errors = 0;
        let mut running_monitors = 0;

        for runtime in monitors.values() {
            if runtime.state == MonitorState::Running {
                running_monitors += 1;
            }
            total_metrics_collected += runtime.stats.metrics_collected;
            total_alerts_generated += runtime.stats.alerts_generated;
            total_errors += runtime.stats.error_count;
        }

        let active_alerts_count = alerts.values()
            .filter(|alert| alert.state == AlertState::Firing)
            .count();

        MonitoringEngineStats {
            engine_state: state.clone(),
            total_monitors: monitors.len(),
            running_monitors,
            total_alert_rules: rules.len(),
            active_alerts: active_alerts_count,
            total_metrics_collected,
            total_alerts_generated,
            total_errors,
            metrics_cache_size: cache.len(),
            realtime_monitoring_enabled: self.config.enable_realtime_monitoring,
        }
    }

    // 私有方法

    /// 启动指标收集循环
    async fn start_metrics_collection_loop(&self) {
        let monitors = self.monitors.clone();
        let metrics_sender = self.metrics_sender.clone();
        let metrics_cache = self.metrics_cache.clone();
        let config = self.config.clone();
        let engine_state = self.engine_state.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.metrics_collection_interval);
            
            loop {
                interval.tick().await;
                
                // 检查引擎状态
                {
                    let state = engine_state.read().await;
                    if *state != MonitoringEngineState::Running {
                        break;
                    }
                }

                // 收集所有监控插件的指标
                Self::collect_all_metrics(&monitors, &metrics_sender, &metrics_cache, &config).await;
            }
        });

        {
            let mut handles = self.monitoring_handles.write().await;
            handles.push(handle);
        }
    }

    /// 启动告警检查循环
    async fn start_alert_checking_loop(&self) {
        let alert_rules = self.alert_rules.clone();
        let active_alerts = self.active_alerts.clone();
        let alert_sender = self.alert_sender.clone();
        let metrics_cache = self.metrics_cache.clone();
        let config = self.config.clone();
        let engine_state = self.engine_state.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.alert_check_interval);
            
            loop {
                interval.tick().await;
                
                // 检查引擎状态
                {
                    let state = engine_state.read().await;
                    if *state != MonitoringEngineState::Running {
                        break;
                    }
                }

                // 检查所有告警规则
                Self::check_alert_rules(&alert_rules, &active_alerts, &alert_sender, &metrics_cache).await;
            }
        });

        {
            let mut handles = self.monitoring_handles.write().await;
            handles.push(handle);
        }
    }

    /// 启动健康检查循环
    async fn start_health_check_loop(&self) {
        let monitors = self.monitors.clone();
        let config = self.config.clone();
        let engine_state = self.engine_state.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                // 检查引擎状态
                {
                    let state = engine_state.read().await;
                    if *state != MonitoringEngineState::Running {
                        break;
                    }
                }

                // 执行健康检查
                Self::perform_health_checks(&monitors).await;
            }
        });

        {
            let mut handles = self.monitoring_handles.write().await;
            handles.push(handle);
        }
    }

    /// 收集所有监控插件的指标
    async fn collect_all_metrics(
        monitors: &Arc<RwLock<HashMap<String, MonitorRuntime>>>,
        metrics_sender: &broadcast::Sender<MetricDataPoint>,
        metrics_cache: &Arc<RwLock<Vec<MetricDataPoint>>>,
        config: &MonitoringEngineConfig,
    ) {
        let monitor_list = {
            let monitors_guard = monitors.read().await;
            monitors_guard.iter()
                .filter(|(_, runtime)| runtime.state == MonitorState::Running)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };

        for monitor_id in monitor_list {
            let monitors_clone = monitors.clone();
            let sender_clone = metrics_sender.clone();
            let cache_clone = metrics_cache.clone();
            let config_clone = config.clone();
            
            tokio::spawn(async move {
                Self::collect_monitor_metrics(&monitors_clone, &monitor_id, &sender_clone, &cache_clone, &config_clone).await;
            });
        }
    }

    /// 收集单个监控插件的指标
    async fn collect_monitor_metrics(
        monitors: &Arc<RwLock<HashMap<String, MonitorRuntime>>>,
        monitor_id: &str,
        metrics_sender: &broadcast::Sender<MetricDataPoint>,
        metrics_cache: &Arc<RwLock<Vec<MetricDataPoint>>>,
        config: &MonitoringEngineConfig,
    ) {
        let collection_start = Instant::now();
        
        let result = {
            let monitors_guard = monitors.read().await;
            if let Some(runtime) = monitors_guard.get(monitor_id) {
                if runtime.state != MonitorState::Running {
                    return;
                }
                
                let plugin = runtime.plugin.clone();
                drop(monitors_guard);

                // 收集指标
                let mut plugin_guard = plugin.lock().await;
                plugin_guard.collect_metrics().await
            } else {
                return;
            }
        };

        let collection_time = collection_start.elapsed();

        // 更新监控统计信息
        {
            let mut monitors_guard = monitors.write().await;
            if let Some(runtime) = monitors_guard.get_mut(monitor_id) {
                runtime.stats.last_collection_time = Some(Instant::now());
                
                match result {
                    Ok(metrics) => {
                        runtime.stats.metrics_collected += metrics.len() as u64;
                        runtime.stats.avg_collection_time = 
                            (runtime.stats.avg_collection_time + collection_time) / 2;
                        
                        // 广播指标数据
                        for metric in &metrics {
                            if let Err(_) = metrics_sender.send(metric.clone()) {
                                debug!("No subscribers for metrics data");
                            }
                        }
                        
                        // 缓存指标数据
                        {
                            let mut cache = metrics_cache.write().await;
                            cache.extend(metrics);
                            
                            // 保持缓存大小限制
                            if cache.len() > config.metrics_cache_size {
                                let remove_count = cache.len() - config.metrics_cache_size;
                                cache.drain(0..remove_count);
                            }
                        }
                        
                        debug!("Monitor '{}' collected {} metrics", monitor_id, metrics.len());
                    }
                    Err(e) => {
                        runtime.stats.error_count += 1;
                        runtime.last_error = Some(e.to_string());
                        runtime.state = MonitorState::Error;
                        
                        error!("Monitor '{}' metrics collection failed: {:?}", monitor_id, e);
                    }
                }
            }
        }
    }

    /// 检查告警规则
    async fn check_alert_rules(
        alert_rules: &Arc<RwLock<HashMap<String, AlertRule>>>,
        active_alerts: &Arc<RwLock<HashMap<String, AlertEvent>>>,
        alert_sender: &broadcast::Sender<AlertEvent>,
        metrics_cache: &Arc<RwLock<Vec<MetricDataPoint>>>,
    ) {
        let rules = alert_rules.read().await;
        let cache = metrics_cache.read().await;
        
        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }
            
            // 查找相关指标
            let relevant_metrics: Vec<&MetricDataPoint> = cache.iter()
                .filter(|metric| metric.name == rule.metric_name)
                .collect();
            
            if relevant_metrics.is_empty() {
                continue;
            }
            
            // 获取最新指标值
            if let Some(latest_metric) = relevant_metrics.last() {
                let current_value = Self::extract_numeric_value(&latest_metric.value);
                let should_fire = Self::evaluate_alert_condition(&rule.condition, current_value);
                
                if should_fire {
                    Self::fire_alert(rule, current_value, active_alerts, alert_sender).await;
                } else {
                    Self::resolve_alert(rule, active_alerts, alert_sender).await;
                }
            }
        }
    }

    /// 执行健康检查
    async fn perform_health_checks(monitors: &Arc<RwLock<HashMap<String, MonitorRuntime>>>) {
        let monitor_list = {
            let monitors_guard = monitors.read().await;
            monitors_guard.iter()
                .filter(|(_, runtime)| runtime.state == MonitorState::Running)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };

        for monitor_id in monitor_list {
            let monitors_clone = monitors.clone();
            
            tokio::spawn(async move {
                Self::check_monitor_health(&monitors_clone, &monitor_id).await;
            });
        }
    }

    /// 检查单个监控插件的健康状态
    async fn check_monitor_health(
        monitors: &Arc<RwLock<HashMap<String, MonitorRuntime>>>,
        monitor_id: &str,
    ) {
        let result = {
            let monitors_guard = monitors.read().await;
            if let Some(runtime) = monitors_guard.get(monitor_id) {
                let plugin = runtime.plugin.clone();
                drop(monitors_guard);

                let plugin_guard = plugin.lock().await;
                plugin_guard.get_health_status().await
            } else {
                return;
            }
        };

        match result {
            Ok(health_status) => {
                let mut monitors_guard = monitors.write().await;
                if let Some(runtime) = monitors_guard.get_mut(monitor_id) {
                    runtime.stats.data_quality_score = health_status.score;
                    
                    if !health_status.healthy {
                        warn!("Monitor '{}' health check failed: {}", monitor_id, health_status.message);
                        runtime.state = MonitorState::Error;
                    }
                }
            }
            Err(e) => {
                error!("Monitor '{}' health check error: {:?}", monitor_id, e);
                
                let mut monitors_guard = monitors.write().await;
                if let Some(runtime) = monitors_guard.get_mut(monitor_id) {
                    runtime.state = MonitorState::Error;
                    runtime.last_error = Some(e.to_string());
                }
            }
        }
    }

    /// 提取数值
    fn extract_numeric_value(value: &MetricValue) -> f64 {
        match value {
            MetricValue::Integer(i) => *i as f64,
            MetricValue::Float(f) => *f,
            MetricValue::Decimal(d) => d.to_f64().unwrap_or(0.0),
            MetricValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }

    /// 评估告警条件
    fn evaluate_alert_condition(condition: &AlertCondition, value: f64) -> bool {
        match condition {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::Equals(target) => (value - target).abs() < f64::EPSILON,
            AlertCondition::InRange(min, max) => value >= *min && value <= *max,
            AlertCondition::OutOfRange(min, max) => value < *min || value > *max,
            AlertCondition::ChangeRate(_rate) => {
                // 简化实现，实际需要计算变化率
                false
            }
            AlertCondition::Custom(_expr) => {
                // 自定义条件需要表达式解析器
                false
            }
        }
    }

    /// 触发告警
    async fn fire_alert(
        rule: &AlertRule,
        current_value: f64,
        active_alerts: &Arc<RwLock<HashMap<String, AlertEvent>>>,
        alert_sender: &broadcast::Sender<AlertEvent>,
    ) {
        let alert_id = format!("{}_{}", rule.id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
        
        let alert_event = AlertEvent {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            level: rule.level.clone(),
            state: AlertState::Firing,
            fired_at: chrono::Utc::now().timestamp(),
            resolved_at: None,
            current_value,
            threshold: Self::get_threshold_from_condition(&rule.condition),
            message: format!("Alert {} fired: {} = {}", rule.name, rule.metric_name, current_value),
            labels: rule.labels.clone(),
            annotations: rule.annotations.clone(),
        };

        {
            let mut alerts = active_alerts.write().await;
            alerts.insert(alert_id, alert_event.clone());
        }

        if let Err(_) = alert_sender.send(alert_event) {
            debug!("No subscribers for alert events");
        }
    }

    /// 解决告警
    async fn resolve_alert(
        rule: &AlertRule,
        active_alerts: &Arc<RwLock<HashMap<String, AlertEvent>>>,
        alert_sender: &broadcast::Sender<AlertEvent>,
    ) {
        let mut alerts = active_alerts.write().await;
        
        // 查找并解决相关告警
        let alert_ids_to_resolve: Vec<String> = alerts.iter()
            .filter(|(_, alert)| alert.rule_id == rule.id && alert.state == AlertState::Firing)
            .map(|(id, _)| id.clone())
            .collect();

        for alert_id in alert_ids_to_resolve {
            if let Some(alert) = alerts.get_mut(&alert_id) {
                alert.state = AlertState::Resolved;
                alert.resolved_at = Some(chrono::Utc::now().timestamp());
                
                if let Err(_) = alert_sender.send(alert.clone()) {
                    debug!("No subscribers for alert events");
                }
            }
        }
    }

    /// 从条件中获取阈值
    fn get_threshold_from_condition(condition: &AlertCondition) -> f64 {
        match condition {
            AlertCondition::GreaterThan(threshold) => *threshold,
            AlertCondition::LessThan(threshold) => *threshold,
            AlertCondition::Equals(target) => *target,
            AlertCondition::InRange(min, _) => *min,
            AlertCondition::OutOfRange(min, _) => *min,
            AlertCondition::ChangeRate(rate) => *rate,
            AlertCondition::Custom(_) => 0.0,
        }
    }
}

/// 监控引擎统计信息
#[derive(Debug, Clone)]
pub struct MonitoringEngineStats {
    /// 引擎状态
    pub engine_state: MonitoringEngineState,
    /// 总监控插件数量
    pub total_monitors: usize,
    /// 运行中的监控插件数量
    pub running_monitors: usize,
    /// 告警规则总数
    pub total_alert_rules: usize,
    /// 活跃告警数量
    pub active_alerts: usize,
    /// 总收集指标数
    pub total_metrics_collected: u64,
    /// 总生成告警数
    pub total_alerts_generated: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 指标缓存大小
    pub metrics_cache_size: usize,
    /// 是否启用实时监控
    pub realtime_monitoring_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct MockMonitoringPlugin {
        metadata: PluginMetadata,
        metrics_count: Arc<AtomicU64>,
        should_fail: bool,
    }

    impl MockMonitoringPlugin {
        fn new(monitor_id: &str, should_fail: bool) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: monitor_id.to_string(),
                    name: format!("Mock Monitor {}", monitor_id),
                    version: semver::Version::new(1, 0, 0),
                    description: "Mock monitoring plugin for testing".to_string(),
                    author: "Test".to_string(),
                    plugin_type: PluginType::Analytics,
                    capabilities: vec![PluginCapability::MetricsCollection],
                    dependencies: vec![],
                    min_framework_version: semver::Version::new(2, 0, 0),
                    max_framework_version: None,
                    config_schema: None,
                    tags: vec![],
                },
                metrics_count: Arc::new(AtomicU64::new(0)),
                should_fail,
            }
        }
    }

    #[async_trait]
    impl Plugin for MockMonitoringPlugin {
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
                healthy: !self.should_fail,
                message: if self.should_fail { "Mock failure" } else { "Healthy" }.to_string(),
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
    impl MonitoringPlugin for MockMonitoringPlugin {
        async fn collect_metrics(&mut self) -> Result<Vec<MetricDataPoint>> {
            if self.should_fail {
                return Err(MosesQuantError::Internal {
                    message: "Mock collection failure".to_string()
                });
            }

            let count = self.metrics_count.fetch_add(1, Ordering::Relaxed);
            
            Ok(vec![
                MetricDataPoint {
                    name: "test_counter".to_string(),
                    metric_type: MetricType::Counter,
                    value: MetricValue::Integer(count as i64),
                    labels: [("source".to_string(), self.metadata.id.clone())].iter().cloned().collect(),
                    timestamp: chrono::Utc::now().timestamp(),
                    source: self.metadata.id.clone(),
                    metadata: HashMap::new(),
                },
                MetricDataPoint {
                    name: "test_gauge".to_string(),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Float(42.5),
                    labels: [("source".to_string(), self.metadata.id.clone())].iter().cloned().collect(),
                    timestamp: chrono::Utc::now().timestamp(),
                    source: self.metadata.id.clone(),
                    metadata: HashMap::new(),
                }
            ])
        }

        async fn get_health_status(&self) -> Result<HealthStatus> {
            Ok(HealthStatus {
                healthy: !self.should_fail,
                score: if self.should_fail { 0.0 } else { 1.0 },
                message: if self.should_fail { "Mock failure" } else { "Healthy" }.to_string(),
                checked_at: chrono::Utc::now().timestamp(),
                details: HashMap::new(),
            })
        }

        fn get_supported_metrics(&self) -> Vec<String> {
            vec!["test_counter".to_string(), "test_gauge".to_string()]
        }

        async fn configure_monitoring(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_monitoring_engine_creation() {
        let config = MonitoringEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = MonitoringEngine::new(config, registry, lifecycle, communication);
        
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.engine_state, MonitoringEngineState::Stopped);
        assert_eq!(stats.total_monitors, 0);
    }

    #[tokio::test]
    async fn test_monitor_registration_and_lifecycle() {
        let config = MonitoringEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = MonitoringEngine::new(config, registry, lifecycle, communication);

        // 注册监控插件
        let monitor_plugin = Arc::new(Mutex::new(MockMonitoringPlugin::new("test_monitor", false)));
        let config = HashMap::new();
        
        engine.register_monitor("test_monitor".to_string(), monitor_plugin, config).await.unwrap();

        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.total_monitors, 1);
        assert_eq!(stats.running_monitors, 0);

        // 启动监控插件
        engine.start_monitor("test_monitor").await.unwrap();
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.running_monitors, 1);

        // 停止监控插件
        engine.stop_monitor("test_monitor").await.unwrap();
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.running_monitors, 0);
    }

    #[tokio::test]
    async fn test_alert_rule_management() {
        let config = MonitoringEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = MonitoringEngine::new(config, registry, lifecycle, communication);

        // 添加告警规则
        let alert_rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Alert".to_string(),
            description: "Test alert rule".to_string(),
            metric_name: "test_metric".to_string(),
            condition: AlertCondition::GreaterThan(100.0),
            level: AlertLevel::Warning,
            evaluation_interval: Duration::from_secs(60),
            for_duration: Duration::from_secs(120),
            labels: HashMap::new(),
            annotations: HashMap::new(),
            enabled: true,
        };

        engine.add_alert_rule(alert_rule).await.unwrap();
        
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.total_alert_rules, 1);

        // 删除告警规则
        engine.remove_alert_rule("test_rule").await.unwrap();
        let stats = engine.get_engine_stats().await;
        assert_eq!(stats.total_alert_rules, 0);
    }

    #[tokio::test]
    async fn test_metrics_subscription() {
        let config = MonitoringEngineConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let engine = MonitoringEngine::new(config, registry, lifecycle, communication);

        // 订阅指标数据
        let mut metrics_receiver = engine.subscribe_metrics();
        
        // 订阅告警事件
        let mut alerts_receiver = engine.subscribe_alerts();

        // 验证订阅器创建成功
        assert!(tokio::time::timeout(Duration::from_millis(10), metrics_receiver.recv()).await.is_err());
        assert!(tokio::time::timeout(Duration::from_millis(10), alerts_receiver.recv()).await.is_err());
    }
}