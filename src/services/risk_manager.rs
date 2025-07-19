//! 可插拔风险管理器
//! 
//! 基于插件系统的高性能风险管理引擎，支持多种风险模型和实时风险监控

use crate::plugins::*;
use crate::types::*;
use crate::{Result, MosesQuantError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};
use rust_decimal::Decimal;

/// 风险管理器配置
#[derive(Debug, Clone)]
pub struct RiskManagerConfig {
    /// 最大并发风险模型数量
    pub max_concurrent_models: usize,
    /// 风险检查超时时间
    pub risk_check_timeout: Duration,
    /// 是否启用实时风险监控
    pub enable_realtime_monitoring: bool,
    /// 风险报告间隔
    pub risk_report_interval: Duration,
    /// 最大单笔订单金额
    pub max_single_order_amount: Decimal,
    /// 最大总持仓价值
    pub max_total_position_value: Decimal,
    /// 日内最大亏损限额
    pub daily_loss_limit: Decimal,
    /// 最大杠杆倍数
    pub max_leverage: Decimal,
    /// 风险预警阈值
    pub risk_warning_threshold: f64,
    /// 强制平仓阈值
    pub force_liquidation_threshold: f64,
}

impl Default for RiskManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_models: 10,
            risk_check_timeout: Duration::from_secs(5),
            enable_realtime_monitoring: true,
            risk_report_interval: Duration::from_secs(60),
            max_single_order_amount: Decimal::from(1000000), // 100万
            max_total_position_value: Decimal::from(10000000), // 1000万
            daily_loss_limit: Decimal::from(500000), // 50万
            max_leverage: Decimal::from(10), // 10倍杠杆
            risk_warning_threshold: 0.8, // 80%
            force_liquidation_threshold: 0.95, // 95%
        }
    }
}

/// 风险检查结果
#[derive(Debug, Clone)]
pub struct RiskCheckResult {
    /// 检查是否通过
    pub passed: bool,
    /// 风险分数 (0.0 - 1.0, 越高越危险)
    pub risk_score: f64,
    /// 风险级别
    pub risk_level: RiskLevel,
    /// 风险报告
    pub reports: Vec<RiskReport>,
    /// 建议操作
    pub recommendations: Vec<RiskRecommendation>,
    /// 检查耗时
    pub check_duration: Duration,
}

/// 风险级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    /// 低风险
    Low,
    /// 中等风险
    Medium,
    /// 高风险
    High,
    /// 极高风险
    Critical,
}

/// 风险报告
#[derive(Debug, Clone)]
pub struct RiskReport {
    /// 报告类型
    pub report_type: RiskReportType,
    /// 报告消息
    pub message: String,
    /// 严重程度
    pub severity: RiskSeverity,
    /// 触发值
    pub trigger_value: Option<Decimal>,
    /// 限制值
    pub limit_value: Option<Decimal>,
    /// 风险模型ID
    pub model_id: String,
    /// 时间戳
    pub timestamp: TimestampNs,
}

/// 风险报告类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskReportType {
    /// 持仓风险
    PositionRisk,
    /// 资金风险
    CapitalRisk,
    /// 杠杆风险
    LeverageRisk,
    /// 集中度风险
    ConcentrationRisk,
    /// 流动性风险
    LiquidityRisk,
    /// 市场风险
    MarketRisk,
    /// 信用风险
    CreditRisk,
    /// 操作风险
    OperationalRisk,
}

/// 风险严重程度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 风险建议
#[derive(Debug, Clone)]
pub struct RiskRecommendation {
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 建议描述
    pub description: String,
    /// 建议的操作
    pub suggested_action: Option<String>,
    /// 优先级
    pub priority: RecommendationPriority,
}

/// 建议类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecommendationType {
    /// 减少持仓
    ReducePosition,
    /// 增加保证金
    IncreaseMargin,
    /// 多样化投资
    Diversify,
    /// 停止交易
    StopTrading,
    /// 立即平仓
    ImmediateLiquidation,
    /// 降低杠杆
    ReduceLeverage,
}

/// 建议优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// 风险模型运行时信息
#[derive(Debug)]
pub struct RiskModelRuntime {
    /// 风险模型插件
    pub plugin: Arc<Mutex<dyn RiskManagerPlugin>>,
    /// 模型状态
    pub state: RiskModelState,
    /// 模型配置
    pub config: HashMap<String, serde_json::Value>,
    /// 运行时统计
    pub stats: RiskModelStats,
    /// 上次检查时间
    pub last_check_time: Option<Instant>,
    /// 最后错误
    pub last_error: Option<String>,
}

/// 风险模型状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskModelState {
    /// 未启动
    Stopped,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 错误状态
    Error,
}

/// 风险模型统计信息
#[derive(Debug, Clone, Default)]
pub struct RiskModelStats {
    /// 总检查次数
    pub total_checks: u64,
    /// 通过检查次数
    pub passed_checks: u64,
    /// 失败检查次数
    pub failed_checks: u64,
    /// 平均检查时间
    pub avg_check_time: Duration,
    /// 最大风险分数
    pub max_risk_score: f64,
    /// 平均风险分数
    pub avg_risk_score: f64,
    /// 生成的报告数量
    pub reports_generated: u64,
    /// 触发的建议数量
    pub recommendations_generated: u64,
}

/// 风险管理器
pub struct RiskManager {
    /// 注册的风险模型
    risk_models: Arc<RwLock<HashMap<String, RiskModelRuntime>>>,
    /// 插件注册表
    plugin_registry: Arc<PluginRegistry>,
    /// 生命周期管理器
    lifecycle_manager: Arc<PluginLifecycleManager>,
    /// 通信管理器
    communication_manager: Arc<PluginCommunicationManager>,
    /// 风险管理器配置
    config: RiskManagerConfig,
    /// 当前投资组合状态
    portfolio_state: Arc<RwLock<Portfolio>>,
    /// 风险监控状态
    monitoring_state: Arc<RwLock<MonitoringState>>,
    /// 风险历史记录
    risk_history: Arc<RwLock<Vec<RiskCheckResult>>>,
}

/// 监控状态
#[derive(Debug, Clone)]
pub struct MonitoringState {
    /// 是否启用监控
    pub enabled: bool,
    /// 当前总风险分数
    pub current_risk_score: f64,
    /// 当前风险级别
    pub current_risk_level: RiskLevel,
    /// 日内亏损
    pub daily_pnl: Decimal,
    /// 当前杠杆倍数
    pub current_leverage: Decimal,
    /// 最后监控时间
    pub last_monitoring_time: Option<Instant>,
    /// 活跃警报数量
    pub active_alerts: usize,
}

impl Default for MonitoringState {
    fn default() -> Self {
        Self {
            enabled: true,
            current_risk_score: 0.0,
            current_risk_level: RiskLevel::Low,
            daily_pnl: Decimal::ZERO,
            current_leverage: Decimal::ZERO,
            last_monitoring_time: None,
            active_alerts: 0,
        }
    }
}

impl RiskManager {
    /// 创建新的风险管理器
    pub fn new(
        config: RiskManagerConfig,
        plugin_registry: Arc<PluginRegistry>,
        lifecycle_manager: Arc<PluginLifecycleManager>,
        communication_manager: Arc<PluginCommunicationManager>,
    ) -> Self {
        Self {
            risk_models: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry,
            lifecycle_manager,
            communication_manager,
            config,
            portfolio_state: Arc::new(RwLock::new(Portfolio::default())),
            monitoring_state: Arc::new(RwLock::new(MonitoringState::default())),
            risk_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册风险模型插件
    pub async fn register_risk_model(
        &self,
        model_id: String,
        plugin: Arc<Mutex<dyn RiskManagerPlugin>>,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // 检查模型数量限制
        {
            let models = self.risk_models.read().await;
            if models.len() >= self.config.max_concurrent_models {
                return Err(MosesQuantError::Internal {
                    message: "Maximum number of risk models reached".to_string()
                });
            }
        }

        // 创建风险模型运行时
        let runtime = RiskModelRuntime {
            plugin,
            state: RiskModelState::Stopped,
            config,
            stats: RiskModelStats::default(),
            last_check_time: None,
            last_error: None,
        };

        // 注册模型
        {
            let mut models = self.risk_models.write().await;
            models.insert(model_id.clone(), runtime);
        }

        info!("Risk model '{}' registered successfully", model_id);
        Ok(())
    }

    /// 启动风险模型
    pub async fn start_risk_model(&self, model_id: &str) -> Result<()> {
        let mut models = self.risk_models.write().await;
        
        if let Some(runtime) = models.get_mut(model_id) {
            match runtime.state {
                RiskModelState::Stopped => {
                    // 创建插件上下文
                    let plugin_context = PluginContext::new(model_id.to_string())
                        .with_config(runtime.config.clone());
                    
                    // 启动插件
                    {
                        let mut plugin = runtime.plugin.lock().await;
                        plugin.initialize(&plugin_context).await?;
                        plugin.start(&plugin_context).await?;
                    }
                    
                    runtime.state = RiskModelState::Running;
                    info!("Risk model '{}' started successfully", model_id);
                    Ok(())
                }
                _ => {
                    Err(MosesQuantError::Internal {
                        message: format!("Risk model '{}' is not in stopped state", model_id)
                    })
                }
            }
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Risk model '{}' not found", model_id)
            })
        }
    }

    /// 停止风险模型
    pub async fn stop_risk_model(&self, model_id: &str) -> Result<()> {
        let mut models = self.risk_models.write().await;
        
        if let Some(runtime) = models.get_mut(model_id) {
            if runtime.state == RiskModelState::Running {
                let plugin_context = PluginContext::new(model_id.to_string());
                
                {
                    let mut plugin = runtime.plugin.lock().await;
                    plugin.stop(&plugin_context).await?;
                }
                
                runtime.state = RiskModelState::Stopped;
                info!("Risk model '{}' stopped successfully", model_id);
                Ok(())
            } else {
                Err(MosesQuantError::Internal {
                    message: format!("Risk model '{}' is not running", model_id)
                })
            }
        } else {
            Err(MosesQuantError::Internal {
                message: format!("Risk model '{}' not found", model_id)
            })
        }
    }

    /// 执行风险检查
    pub async fn check_risk(&self, order: &Order) -> Result<RiskCheckResult> {
        let start_time = Instant::now();
        let mut all_reports = Vec::new();
        let mut all_recommendations = Vec::new();
        let mut max_risk_score = 0.0;

        // 获取当前投资组合状态
        let portfolio = self.portfolio_state.read().await.clone();

        // 执行所有风险模型检查
        {
            let models = self.risk_models.read().await;
            
            for (model_id, runtime) in models.iter() {
                if runtime.state == RiskModelState::Running {
                    let plugin = runtime.plugin.clone();
                    let model_id_clone = model_id.clone();
                    
                    let check_result = tokio::time::timeout(
                        self.config.risk_check_timeout,
                        async {
                            let mut plugin_guard = plugin.lock().await;
                            plugin_guard.check_order_risk(order, &portfolio).await
                        }
                    ).await;

                    match check_result {
                        Ok(Ok(result)) => {
                            all_reports.extend(result.reports);
                            all_recommendations.extend(result.recommendations);
                            max_risk_score = max_risk_score.max(result.risk_score);
                            
                            debug!("Risk model '{}' check completed with score: {}", 
                                   model_id_clone, result.risk_score);
                        }
                        Ok(Err(e)) => {
                            error!("Risk model '{}' check failed: {:?}", model_id_clone, e);
                            all_reports.push(RiskReport {
                                report_type: RiskReportType::OperationalRisk,
                                message: format!("Risk model check failed: {}", e),
                                severity: RiskSeverity::Error,
                                trigger_value: None,
                                limit_value: None,
                                model_id: model_id_clone,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                        Err(_) => {
                            error!("Risk model '{}' check timeout", model_id_clone);
                            all_reports.push(RiskReport {
                                report_type: RiskReportType::OperationalRisk,
                                message: "Risk check timeout".to_string(),
                                severity: RiskSeverity::Warning,
                                trigger_value: None,
                                limit_value: None,
                                model_id: model_id_clone,
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }

        // 执行框架级风险检查
        self.perform_framework_risk_checks(order, &portfolio, &mut all_reports, &mut all_recommendations, &mut max_risk_score).await;

        // 确定风险级别
        let risk_level = self.determine_risk_level(max_risk_score);

        // 确定是否通过检查
        let passed = max_risk_score < self.config.risk_warning_threshold && 
                    !all_reports.iter().any(|r| r.severity == RiskSeverity::Critical);

        let result = RiskCheckResult {
            passed,
            risk_score: max_risk_score,
            risk_level,
            reports: all_reports,
            recommendations: all_recommendations,
            check_duration: start_time.elapsed(),
        };

        // 更新统计信息
        self.update_risk_statistics(&result).await;

        // 保存风险历史
        {
            let mut history = self.risk_history.write().await;
            history.push(result.clone());
            
            // 保留最近1000条记录
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        Ok(result)
    }

    /// 更新投资组合状态
    pub async fn update_portfolio(&self, portfolio: Portfolio) -> Result<()> {
        {
            let mut current_portfolio = self.portfolio_state.write().await;
            *current_portfolio = portfolio;
        }

        // 如果启用实时监控，触发风险检查
        if self.config.enable_realtime_monitoring {
            self.perform_realtime_monitoring().await?;
        }

        Ok(())
    }

    /// 获取当前风险状态
    pub async fn get_current_risk_status(&self) -> MonitoringState {
        self.monitoring_state.read().await.clone()
    }

    /// 获取风险历史记录
    pub async fn get_risk_history(&self, limit: Option<usize>) -> Vec<RiskCheckResult> {
        let history = self.risk_history.read().await;
        let limit = limit.unwrap_or(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// 获取风险模型统计信息
    pub async fn get_risk_model_stats(&self, model_id: &str) -> Option<RiskModelStats> {
        let models = self.risk_models.read().await;
        models.get(model_id).map(|runtime| runtime.stats.clone())
    }

    /// 获取所有风险模型状态
    pub async fn get_all_risk_models_status(&self) -> HashMap<String, RiskModelState> {
        let models = self.risk_models.read().await;
        models.iter()
            .map(|(id, runtime)| (id.clone(), runtime.state.clone()))
            .collect()
    }

    // 私有方法

    /// 执行框架级风险检查
    async fn perform_framework_risk_checks(
        &self,
        order: &Order,
        portfolio: &Portfolio,
        reports: &mut Vec<RiskReport>,
        recommendations: &mut Vec<RiskRecommendation>,
        max_risk_score: &mut f64,
    ) {
        let current_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // 检查单笔订单金额限制
        let order_value = order.price * order.quantity;
        if order_value > self.config.max_single_order_amount {
            reports.push(RiskReport {
                report_type: RiskReportType::CapitalRisk,
                message: format!("Order amount {} exceeds limit {}", order_value, self.config.max_single_order_amount),
                severity: RiskSeverity::Error,
                trigger_value: Some(order_value),
                limit_value: Some(self.config.max_single_order_amount),
                model_id: "framework".to_string(),
                timestamp: current_time,
            });

            recommendations.push(RiskRecommendation {
                recommendation_type: RecommendationType::ReducePosition,
                description: "Reduce order size to comply with single order limit".to_string(),
                suggested_action: Some(format!("Reduce order quantity to {}", 
                    self.config.max_single_order_amount / order.price)),
                priority: RecommendationPriority::High,
            });

            *max_risk_score = (*max_risk_score).max(0.9);
        }

        // 检查总持仓价值限制
        if portfolio.total_value > self.config.max_total_position_value {
            reports.push(RiskReport {
                report_type: RiskReportType::PositionRisk,
                message: format!("Total position value {} exceeds limit {}", 
                                portfolio.total_value, self.config.max_total_position_value),
                severity: RiskSeverity::Critical,
                trigger_value: Some(portfolio.total_value),
                limit_value: Some(self.config.max_total_position_value),
                model_id: "framework".to_string(),
                timestamp: current_time,
            });

            recommendations.push(RiskRecommendation {
                recommendation_type: RecommendationType::ImmediateLiquidation,
                description: "Immediate position reduction required".to_string(),
                suggested_action: Some("Liquidate positions to reduce total exposure".to_string()),
                priority: RecommendationPriority::Critical,
            });

            *max_risk_score = 1.0;
        }

        // 检查杠杆限制
        if portfolio.cash_balance > Decimal::ZERO {
            let leverage = portfolio.total_value / portfolio.cash_balance;
            if leverage > self.config.max_leverage {
                reports.push(RiskReport {
                    report_type: RiskReportType::LeverageRisk,
                    message: format!("Current leverage {} exceeds limit {}", leverage, self.config.max_leverage),
                    severity: RiskSeverity::Warning,
                    trigger_value: Some(leverage),
                    limit_value: Some(self.config.max_leverage),
                    model_id: "framework".to_string(),
                    timestamp: current_time,
                });

                recommendations.push(RiskRecommendation {
                    recommendation_type: RecommendationType::ReduceLeverage,
                    description: "Reduce leverage by increasing margin or reducing positions".to_string(),
                    suggested_action: Some("Add more capital or close some positions".to_string()),
                    priority: RecommendationPriority::Medium,
                });

                *max_risk_score = (*max_risk_score).max(0.7);
            }
        }
    }

    /// 确定风险级别
    fn determine_risk_level(&self, risk_score: f64) -> RiskLevel {
        if risk_score >= self.config.force_liquidation_threshold {
            RiskLevel::Critical
        } else if risk_score >= self.config.risk_warning_threshold {
            RiskLevel::High
        } else if risk_score >= 0.5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// 更新风险统计信息
    async fn update_risk_statistics(&self, result: &RiskCheckResult) {
        let mut models = self.risk_models.write().await;
        
        for report in &result.reports {
            if let Some(runtime) = models.get_mut(&report.model_id) {
                runtime.stats.total_checks += 1;
                if result.passed {
                    runtime.stats.passed_checks += 1;
                } else {
                    runtime.stats.failed_checks += 1;
                }
                
                runtime.stats.max_risk_score = runtime.stats.max_risk_score.max(result.risk_score);
                
                // 更新平均风险分数
                let total_checks = runtime.stats.total_checks as f64;
                runtime.stats.avg_risk_score = 
                    (runtime.stats.avg_risk_score * (total_checks - 1.0) + result.risk_score) / total_checks;
                
                // 更新平均检查时间
                runtime.stats.avg_check_time = 
                    (runtime.stats.avg_check_time * (runtime.stats.total_checks - 1) as u32 + result.check_duration) / runtime.stats.total_checks as u32;
                
                runtime.stats.reports_generated += result.reports.len() as u64;
                runtime.stats.recommendations_generated += result.recommendations.len() as u64;
                
                runtime.last_check_time = Some(Instant::now());
            }
        }

        // 更新监控状态
        {
            let mut monitoring = self.monitoring_state.write().await;
            monitoring.current_risk_score = result.risk_score;
            monitoring.current_risk_level = result.risk_level.clone();
            monitoring.last_monitoring_time = Some(Instant::now());
            monitoring.active_alerts = result.reports.iter()
                .filter(|r| r.severity >= RiskSeverity::Warning)
                .count();
        }
    }

    /// 执行实时风险监控
    async fn perform_realtime_monitoring(&self) -> Result<()> {
        let portfolio = self.portfolio_state.read().await.clone();
        let monitoring = self.monitoring_state.read().await.clone();

        // 检查是否需要生成警报
        if monitoring.current_risk_score > self.config.risk_warning_threshold {
            warn!("High risk detected: score = {}, level = {:?}", 
                  monitoring.current_risk_score, monitoring.current_risk_level);
        }

        // 检查是否需要强制平仓
        if monitoring.current_risk_score >= self.config.force_liquidation_threshold {
            error!("Critical risk level reached: score = {}, initiating emergency procedures", 
                   monitoring.current_risk_score);
            
            // 这里可以触发紧急风险控制措施
            // 例如：停止所有交易、发送紧急通知等
        }

        Ok(())
    }
}

/// 风险管理器统计信息
#[derive(Debug, Clone)]
pub struct RiskManagerStats {
    /// 总风险模型数量
    pub total_models: usize,
    /// 运行中的模型数量
    pub running_models: usize,
    /// 总风险检查次数
    pub total_risk_checks: u64,
    /// 通过的检查次数
    pub passed_checks: u64,
    /// 当前风险分数
    pub current_risk_score: f64,
    /// 当前风险级别
    pub current_risk_level: RiskLevel,
    /// 活跃警报数量
    pub active_alerts: usize,
    /// 平均检查时间
    pub avg_check_time: Duration,
}

impl RiskManager {
    /// 获取风险管理器统计信息
    pub async fn get_manager_stats(&self) -> RiskManagerStats {
        let models = self.risk_models.read().await;
        let monitoring = self.monitoring_state.read().await;

        let mut total_checks = 0;
        let mut passed_checks = 0;
        let mut total_check_time = Duration::ZERO;
        let mut running_models = 0;

        for runtime in models.values() {
            if runtime.state == RiskModelState::Running {
                running_models += 1;
            }
            total_checks += runtime.stats.total_checks;
            passed_checks += runtime.stats.passed_checks;
            total_check_time += runtime.stats.avg_check_time;
        }

        let avg_check_time = if models.len() > 0 {
            total_check_time / models.len() as u32
        } else {
            Duration::ZERO
        };

        RiskManagerStats {
            total_models: models.len(),
            running_models,
            total_risk_checks: total_checks,
            passed_checks,
            current_risk_score: monitoring.current_risk_score,
            current_risk_level: monitoring.current_risk_level.clone(),
            active_alerts: monitoring.active_alerts,
            avg_check_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct MockRiskManagerPlugin {
        metadata: PluginMetadata,
        check_count: Arc<AtomicU64>,
        risk_score: f64,
    }

    impl MockRiskManagerPlugin {
        fn new(model_id: &str, risk_score: f64) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: model_id.to_string(),
                    name: format!("Mock Risk Model {}", model_id),
                    version: semver::Version::new(1, 0, 0),
                    description: "Mock risk manager for testing".to_string(),
                    author: "Test".to_string(),
                    plugin_type: PluginType::RiskManager,
                    capabilities: vec![PluginCapability::RiskCalculation],
                    dependencies: vec![],
                    min_framework_version: semver::Version::new(2, 0, 0),
                    max_framework_version: None,
                    config_schema: None,
                    tags: vec![],
                },
                check_count: Arc::new(AtomicU64::new(0)),
                risk_score,
            }
        }
    }

    #[async_trait]
    impl Plugin for MockRiskManagerPlugin {
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
                message: "Mock risk manager is healthy".to_string(),
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
    impl RiskManagerPlugin for MockRiskManagerPlugin {
        async fn check_order_risk(&mut self, _order: &Order, _portfolio: &Portfolio) -> Result<RiskCheckResult> {
            self.check_count.fetch_add(1, Ordering::Relaxed);
            
            let reports = if self.risk_score > 0.8 {
                vec![RiskReport {
                    report_type: RiskReportType::PositionRisk,
                    message: "High risk detected".to_string(),
                    severity: RiskSeverity::Warning,
                    trigger_value: Some(Decimal::from_f64(self.risk_score).unwrap()),
                    limit_value: Some(Decimal::from_f64(0.8).unwrap()),
                    model_id: self.metadata.id.clone(),
                    timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                }]
            } else {
                vec![]
            };

            let recommendations = if self.risk_score > 0.9 {
                vec![RiskRecommendation {
                    recommendation_type: RecommendationType::ReducePosition,
                    description: "Reduce position size".to_string(),
                    suggested_action: Some("Sell 50% of current position".to_string()),
                    priority: RecommendationPriority::High,
                }]
            } else {
                vec![]
            };

            Ok(RiskCheckResult {
                passed: self.risk_score < 0.8,
                risk_score: self.risk_score,
                risk_level: if self.risk_score > 0.9 { RiskLevel::Critical } 
                          else if self.risk_score > 0.7 { RiskLevel::High }
                          else if self.risk_score > 0.5 { RiskLevel::Medium }
                          else { RiskLevel::Low },
                reports,
                recommendations,
                check_duration: Duration::from_millis(10),
            })
        }

        async fn check_portfolio_risk(&mut self, _portfolio: &Portfolio) -> Result<RiskCheckResult> {
            self.check_order_risk(&Order::default(), _portfolio).await
        }

        async fn get_risk_metrics(&self) -> Result<HashMap<String, f64>> {
            let mut metrics = HashMap::new();
            metrics.insert("risk_score".to_string(), self.risk_score);
            metrics.insert("check_count".to_string(), self.check_count.load(Ordering::Relaxed) as f64);
            Ok(metrics)
        }
    }

    #[tokio::test]
    async fn test_risk_manager_creation() {
        let config = RiskManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = RiskManager::new(config, registry, lifecycle, communication);
        
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.total_models, 0);
        assert_eq!(stats.running_models, 0);
    }

    #[tokio::test]
    async fn test_risk_model_registration_and_lifecycle() {
        let config = RiskManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = RiskManager::new(config, registry, lifecycle, communication);

        // 注册风险模型
        let risk_model_plugin = Arc::new(Mutex::new(MockRiskManagerPlugin::new("test_model", 0.3)));
        let config = HashMap::new();
        
        manager.register_risk_model("test_model".to_string(), risk_model_plugin, config).await.unwrap();

        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.total_models, 1);
        assert_eq!(stats.running_models, 0);

        // 启动风险模型
        manager.start_risk_model("test_model").await.unwrap();
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.running_models, 1);

        // 停止风险模型
        manager.stop_risk_model("test_model").await.unwrap();
        let stats = manager.get_manager_stats().await;
        assert_eq!(stats.running_models, 0);
    }

    #[tokio::test]
    async fn test_risk_check() {
        let config = RiskManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = RiskManager::new(config, registry, lifecycle, communication);

        // 注册并启动风险模型
        let risk_model_plugin = Arc::new(Mutex::new(MockRiskManagerPlugin::new("test_model", 0.3)));
        manager.register_risk_model("test_model".to_string(), risk_model_plugin, HashMap::new()).await.unwrap();
        manager.start_risk_model("test_model").await.unwrap();

        // 创建测试订单
        let order = Order {
            id: "test_order".to_string(),
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            order_type: OrderType::Market,
            direction: Direction::Buy,
            quantity: Decimal::from(1),
            price: Decimal::from(50000),
            status: OrderStatus::New,
            filled_quantity: Decimal::ZERO,
            average_fill_price: Decimal::ZERO,
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            created_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            updated_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        // 执行风险检查
        let result = manager.check_risk(&order).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.risk_level, RiskLevel::Low);
        assert!(result.risk_score <= 0.5);
    }

    #[tokio::test]
    async fn test_high_risk_scenario() {
        let config = RiskManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let lifecycle = Arc::new(PluginLifecycleManager::new(
            crate::plugins::LifecycleManagerConfig::default(),
            None
        ));
        let communication = Arc::new(PluginCommunicationManager::new(None));

        let manager = RiskManager::new(config, registry, lifecycle, communication);

        // 注册高风险模型
        let risk_model_plugin = Arc::new(Mutex::new(MockRiskManagerPlugin::new("high_risk_model", 0.95)));
        manager.register_risk_model("high_risk_model".to_string(), risk_model_plugin, HashMap::new()).await.unwrap();
        manager.start_risk_model("high_risk_model").await.unwrap();

        // 创建测试订单
        let order = Order {
            id: "test_order".to_string(),
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            order_type: OrderType::Market,
            direction: Direction::Buy,
            quantity: Decimal::from(1),
            price: Decimal::from(50000),
            status: OrderStatus::New,
            filled_quantity: Decimal::ZERO,
            average_fill_price: Decimal::ZERO,
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            created_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            updated_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        // 执行风险检查
        let result = manager.check_risk(&order).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.risk_level, RiskLevel::Critical);
        assert!(result.risk_score > 0.9);
        assert!(!result.reports.is_empty());
        assert!(!result.recommendations.is_empty());
    }
}