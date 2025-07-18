//! MosesQuant 使用示例
//! 
//! 演示如何使用MosesQuant框架进行数字货币量化交易

use czsc_core::{
    data::{DataManager, CsvDataSource, BinanceConnector, BinanceConfig},
    strategy::{StrategyFramework, SimpleUniverseSelector, SimpleAlphaModel},
    types::*,
    Result,
};
use std::sync::Arc;
use std::collections::HashMap;

/// 简单的投资组合构建器
#[derive(Debug)]
pub struct SimplePortfolioConstructor {
    name: String,
}

impl SimplePortfolioConstructor {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl czsc_core::strategy::PortfolioConstructor for SimplePortfolioConstructor {
    async fn create_targets(
        &self,
        _context: &czsc_core::strategy::StrategyContext,
        insights: &[Insight],
    ) -> Result<Vec<PortfolioTarget>> {
        let mut targets = Vec::new();
        
        for insight in insights {
            let target_percent = match insight.direction {
                InsightDirection::Up => 10.0,   // 看涨时分配10%
                InsightDirection::Down => -5.0, // 看跌时做空5%
                InsightDirection::Flat => 0.0,  // 中性时不分配
            };
            
            let target = PortfolioTarget {
                symbol: insight.symbol.clone(),
                target_percent,
                target_quantity: None,
                target_value: None,
                generated_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                priority: Some(50),
                tag: Some("Example".to_string()),
            };
            
            targets.push(target);
        }
        
        Ok(targets)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// 简单的风险管理器
#[derive(Debug)]
pub struct SimpleRiskManager {
    name: String,
    max_position_size: f64,
}

impl SimpleRiskManager {
    pub fn new(name: String, max_position_size: f64) -> Self {
        Self { name, max_position_size }
    }
}

#[async_trait::async_trait]
impl czsc_core::strategy::RiskManager for SimpleRiskManager {
    async fn validate_targets(
        &self,
        _context: &czsc_core::strategy::StrategyContext,
        targets: &[PortfolioTarget],
    ) -> Result<Vec<PortfolioTarget>> {
        let mut validated_targets = Vec::new();
        
        for target in targets {
            // 限制单个持仓大小
            let adjusted_percent = if target.target_percent.abs() > self.max_position_size {
                if target.target_percent > 0.0 {
                    self.max_position_size
                } else {
                    -self.max_position_size
                }
            } else {
                target.target_percent
            };
            
            let mut adjusted_target = target.clone();
            adjusted_target.target_percent = adjusted_percent;
            
            validated_targets.push(adjusted_target);
        }
        
        Ok(validated_targets)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// 简单的执行算法
#[derive(Debug)]
pub struct SimpleExecutionAlgorithm {
    name: String,
}

impl SimpleExecutionAlgorithm {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl czsc_core::strategy::ExecutionAlgorithm for SimpleExecutionAlgorithm {
    async fn execute_targets(
        &self,
        context: &czsc_core::strategy::StrategyContext,
        targets: &[PortfolioTarget],
    ) -> Result<Vec<Order>> {
        let mut orders = Vec::new();
        
        for target in targets {
            // 计算目标数量
            let target_value = context.portfolio_value * target.target_percent / 100.0;
            
            // 假设当前价格为50000 USDT
            let current_price = 50000.0;
            let target_quantity = target_value / current_price;
            
            if target_quantity.abs() > 0.001 { // 最小订单大小
                let order = Order {
                    order_id: format!("order_{}", uuid::Uuid::new_v4()),
                    symbol: target.symbol.clone(),
                    direction: if target_quantity > 0.0 { Direction::Long } else { Direction::Short },
                    order_type: OrderType::Market,
                    price: None,
                    quantity: target_quantity.abs(),
                    filled_quantity: 0.0,
                    status: OrderStatus::Pending,
                    created_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    updated_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                };
                
                orders.push(order);
            }
        }
        
        Ok(orders)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// 运行完整的量化交易示例
pub async fn run_quantitative_trading_example() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    tracing::info!("🚀 启动 MosesQuant 量化交易示例");
    
    // 1. 创建数据管理器
    let data_manager = DataManager::new();
    
    // 2. 设置CSV历史数据源
    let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
    let csv_source = CsvDataSource::new(
        "btc_historical".to_string(),
        r"E:\code\QuantTrade\czsc_enhanced\test\data\BTCUSDT_1m_2023-09.csv".to_string(),
        symbol.clone(),
    );
    
    // 如果CSV文件存在，加载数据
    if let Err(e) = csv_source.load_data().await {
        tracing::warn!("无法加载CSV数据: {}，使用模拟数据", e);
    }
    
    data_manager.register_source("csv_historical".to_string(), Arc::new(csv_source)).await?;
    
    // 3. 设置Binance连接器（可选）
    let binance_config = BinanceConfig::default();
    let mut binance_connector = BinanceConnector::new("binance_live".to_string(), binance_config);
    
    if let Err(e) = binance_connector.connect().await {
        tracing::warn!("无法连接到Binance: {}，仅使用历史数据", e);
    } else {
        data_manager.register_source("binance_live".to_string(), Arc::new(binance_connector)).await?;
    }
    
    // 4. 创建策略上下文
    let strategy_context = czsc_core::strategy::StrategyContext {
        strategy_id: "btc_momentum_strategy".to_string(),
        portfolio_value: 100000.0,
        cash: 100000.0,
        positions: HashMap::new(),
        current_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        market_data: HashMap::new(),
    };
    
    // 5. 创建五阶段策略组件
    let universe_selector = Arc::new(SimpleUniverseSelector::new(
        "crypto_universe".to_string(),
        vec![symbol.clone()],
    ));
    
    let alpha_model = Arc::new(SimpleAlphaModel::new(
        "momentum_alpha".to_string(),
    ));
    
    let portfolio_constructor = Arc::new(SimplePortfolioConstructor::new(
        "equal_weight_portfolio".to_string(),
    ));
    
    let risk_manager = Arc::new(SimpleRiskManager::new(
        "basic_risk_manager".to_string(),
        15.0, // 最大单个持仓15%
    ));
    
    let execution_algorithm = Arc::new(SimpleExecutionAlgorithm::new(
        "market_execution".to_string(),
    ));
    
    // 6. 创建策略框架
    let strategy_framework = StrategyFramework::new(
        strategy_context,
        universe_selector,
        alpha_model,
        portfolio_constructor,
        risk_manager,
        execution_algorithm,
    );
    
    // 7. 获取历史数据
    let bars = data_manager.get_bars(&symbol, 100).await?;
    tracing::info!("获取到 {} 条历史数据", bars.len());
    
    // 8. 运行策略流水线
    tracing::info!("开始执行策略流水线...");
    let result = strategy_framework.execute_pipeline().await?;
    
    if result.success {
        tracing::info!("✅ 策略执行成功!");
        tracing::info!("  - 标的数量: {}", result.universe_size);
        tracing::info!("  - 生成洞见: {}", result.insights_generated);
        tracing::info!("  - 投资组合目标: {}", result.targets_created);
        tracing::info!("  - 生成订单: {}", result.orders_generated);
        tracing::info!("  - 执行时间: {:?}", result.execution_time);
        
        // 显示生成的订单
        for order in &result.orders {
            tracing::info!("📋 订单: {} {} {} {} @ {:?}",
                order.order_id,
                order.symbol.value,
                match order.direction {
                    Direction::Long => "BUY",
                    Direction::Short => "SELL",
                },
                order.quantity,
                order.price
            );
        }
    } else {
        tracing::warn!("❌ 策略执行失败");
    }
    
    // 9. 显示统计信息
    let stats = strategy_framework.get_stats().await;
    tracing::info!("📊 策略统计:");
    tracing::info!("  - 总执行次数: {}", stats.total_executions);
    tracing::info!("  - 平均执行时间: {:?}", stats.avg_execution_time());
    
    let data_stats = data_manager.get_stats().await;
    tracing::info!("📈 数据统计:");
    tracing::info!("  - 总请求数: {}", data_stats.requests);
    tracing::info!("  - 缓存命中率: {:.2}%", data_stats.cache_hit_rate() * 100.0);
    
    tracing::info!("🎉 MosesQuant 量化交易示例完成!");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run_quantitative_trading_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simple_portfolio_constructor() {
        let constructor = SimplePortfolioConstructor::new("test".to_string());
        assert_eq!(constructor.name(), "test");
    }
    
    #[tokio::test]
    async fn test_simple_risk_manager() {
        let risk_manager = SimpleRiskManager::new("test".to_string(), 10.0);
        assert_eq!(risk_manager.name(), "test");
    }
    
    #[tokio::test]
    async fn test_simple_execution_algorithm() {
        let execution = SimpleExecutionAlgorithm::new("test".to_string());
        assert_eq!(execution.name(), "test");
    }
}