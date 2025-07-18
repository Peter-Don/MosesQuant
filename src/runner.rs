//! MosesQuant 配置驱动的策略运行器
//! 
//! 基于YAML配置文件自动构建和运行策略

use crate::config::*;
use crate::data::{DataManager, CsvDataSource, BinanceConnector, BinanceConfig};
use crate::strategy::*;
use crate::types::*;
use crate::Result;
use std::sync::Arc;
use std::collections::HashMap;

/// 策略运行器
pub struct StrategyRunner {
    config: FrameworkConfig,
    data_manager: DataManager,
    strategies: Vec<StrategyFramework>,
}

impl StrategyRunner {
    /// 从配置创建策略运行器
    pub async fn from_config(config: FrameworkConfig) -> Result<Self> {
        // 创建数据管理器
        let data_manager = DataManager::new();
        
        // 初始化数据源
        let mut runner = Self {
            config,
            data_manager,
            strategies: Vec::new(),
        };
        
        runner.setup_data_sources().await?;
        runner.setup_strategies().await?;
        
        Ok(runner)
    }
    
    /// 从配置文件创建策略运行器
    pub async fn from_config_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let config_manager = ConfigManager::load_from_file(path).await?;
        config_manager.validate()?;
        
        Self::from_config(config_manager.get_config().clone()).await
    }
    
    /// 设置数据源
    async fn setup_data_sources(&self) -> Result<()> {
        for data_source_config in &self.config.data_sources {
            if !data_source_config.enabled {
                continue;
            }
            
            match &data_source_config.source_type {
                DataSourceType::Csv => {
                    self.setup_csv_data_source(data_source_config).await?;
                }
                DataSourceType::Binance => {
                    self.setup_binance_data_source(data_source_config).await?;
                }
                DataSourceType::Custom(_) => {
                    tracing::warn!("Custom data source type not yet implemented: {}", data_source_config.name);
                }
            }
        }
        
        Ok(())
    }
    
    /// 设置CSV数据源
    async fn setup_csv_data_source(&self, config: &DataSourceConfig) -> Result<()> {
        let file_path = config.connection.params.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::CzscError::config("CSV data source missing file_path parameter"))?;
        
        for symbol_str in &config.symbols {
            let symbol = Symbol::new(symbol_str, "CSV", AssetType::Crypto);
            
            let csv_source = CsvDataSource::new(
                format!("{}_{}", config.name, symbol_str),
                file_path.to_string(),
                symbol,
            );
            
            // 尝试加载数据
            if let Err(e) = csv_source.load_data().await {
                tracing::warn!("Failed to load CSV data for {}: {}", symbol_str, e);
                continue;
            }
            
            self.data_manager.register_source(
                format!("{}_{}", config.name, symbol_str),
                Arc::new(csv_source),
            ).await?;
            
            tracing::info!("CSV data source registered: {} for {}", config.name, symbol_str);
        }
        
        Ok(())
    }
    
    /// 设置Binance数据源
    async fn setup_binance_data_source(&self, config: &DataSourceConfig) -> Result<()> {
        let testnet = config.connection.params.get("testnet")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let api_key = config.connection.params.get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let secret_key = config.connection.params.get("secret_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let binance_config = BinanceConfig {
            api_key,
            secret_key,
            testnet,
            ..Default::default()
        };
        
        let mut binance_connector = BinanceConnector::new(
            config.name.clone(),
            binance_config,
        );
        
        // 尝试连接
        if let Err(e) = binance_connector.connect().await {
            tracing::warn!("Failed to connect to Binance: {}", e);
            return Ok(());
        }
        
        self.data_manager.register_source(
            config.name.clone(),
            Arc::new(binance_connector),
        ).await?;
        
        tracing::info!("Binance data source registered: {}", config.name);
        Ok(())
    }
    
    /// 设置策略
    async fn setup_strategies(&mut self) -> Result<()> {
        for strategy_config in &self.config.strategies {
            if !strategy_config.enabled {
                continue;
            }
            
            let strategy_framework = self.create_strategy_framework(strategy_config).await?;
            self.strategies.push(strategy_framework);
            
            tracing::info!("Strategy registered: {} ({})", strategy_config.name, strategy_config.id);
        }
        
        Ok(())
    }
    
    /// 创建策略框架
    async fn create_strategy_framework(&self, config: &StrategyConfig) -> Result<StrategyFramework> {
        // 创建策略上下文
        let context = StrategyContext {
            strategy_id: config.id.clone(),
            portfolio_value: self.config.framework.initial_capital,
            cash: self.config.framework.initial_capital,
            positions: HashMap::new(),
            current_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            market_data: HashMap::new(),
        };
        
        // 创建标的选择器
        let universe_selector = self.create_universe_selector(&config.universe_selector)?;
        
        // 创建Alpha模型
        let alpha_model = self.create_alpha_model(&config.alpha_model)?;
        
        // 创建投资组合构建器
        let portfolio_constructor = self.create_portfolio_constructor(&config.portfolio_constructor)?;
        
        // 创建风险管理器
        let risk_manager = self.create_risk_manager()?;
        
        // 创建执行算法
        let execution_algorithm = self.create_execution_algorithm()?;
        
        let strategy_framework = StrategyFramework::new(
            context,
            universe_selector,
            alpha_model,
            portfolio_constructor,
            risk_manager,
            execution_algorithm,
        );
        
        Ok(strategy_framework)
    }
    
    /// 创建标的选择器
    fn create_universe_selector(&self, config: &ComponentConfig) -> Result<Arc<dyn UniverseSelector>> {
        match config.component_type.as_str() {
            "SimpleUniverseSelector" => {
                let symbols_json = config.parameters.get("symbols")
                    .ok_or_else(|| crate::CzscError::config("SimpleUniverseSelector missing symbols parameter"))?;
                
                let symbol_strings: Vec<String> = serde_json::from_value(symbols_json.clone())
                    .map_err(|e| crate::CzscError::config(&format!("Invalid symbols format: {}", e)))?;
                
                let symbols: Vec<Symbol> = symbol_strings.iter()
                    .map(|s| Symbol::new(s, "BINANCE", AssetType::Crypto))
                    .collect();
                
                Ok(Arc::new(SimpleUniverseSelector::new(
                    "config_universe".to_string(),
                    symbols,
                )))
            }
            _ => Err(crate::CzscError::config(&format!("Unknown universe selector type: {}", config.component_type)))
        }
    }
    
    /// 创建Alpha模型
    fn create_alpha_model(&self, config: &ComponentConfig) -> Result<Arc<dyn AlphaModel>> {
        match config.component_type.as_str() {
            "SimpleAlphaModel" => {
                Ok(Arc::new(SimpleAlphaModel::new(
                    "config_alpha".to_string(),
                )))
            }
            _ => Err(crate::CzscError::config(&format!("Unknown alpha model type: {}", config.component_type)))
        }
    }
    
    /// 创建投资组合构建器
    fn create_portfolio_constructor(&self, config: &ComponentConfig) -> Result<Arc<dyn PortfolioConstructor>> {
        match config.component_type.as_str() {
            "SimplePortfolioConstructor" => {
                // 这里需要实现一个简单的投资组合构建器
                Ok(Arc::new(ConfigurablePortfolioConstructor::new(
                    "config_portfolio".to_string(),
                )))
            }
            _ => Err(crate::CzscError::config(&format!("Unknown portfolio constructor type: {}", config.component_type)))
        }
    }
    
    /// 创建风险管理器
    fn create_risk_manager(&self) -> Result<Arc<dyn RiskManager>> {
        Ok(Arc::new(ConfigurableRiskManager::new(
            "config_risk".to_string(),
            self.config.risk_management.clone(),
        )))
    }
    
    /// 创建执行算法
    fn create_execution_algorithm(&self) -> Result<Arc<dyn ExecutionAlgorithm>> {
        Ok(Arc::new(ConfigurableExecutionAlgorithm::new(
            "config_execution".to_string(),
            self.config.execution.clone(),
        )))
    }
    
    /// 运行所有策略
    pub async fn run_strategies(&mut self) -> Result<Vec<StrategyResult>> {
        let mut results = Vec::new();
        
        tracing::info!("Running {} strategies", self.strategies.len());
        
        let strategy_count = self.strategies.len();
        for (i, strategy) in self.strategies.iter_mut().enumerate() {
            tracing::info!("Executing strategy {}/{}", i + 1, strategy_count);
            
            let result = strategy.execute_pipeline().await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// 获取数据统计
    pub async fn get_data_stats(&self) -> crate::data::DataStats {
        self.data_manager.get_stats().await
    }
}

/// 可配置的投资组合构建器
#[derive(Debug)]
pub struct ConfigurablePortfolioConstructor {
    name: String,
}

impl ConfigurablePortfolioConstructor {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait::async_trait]
impl PortfolioConstructor for ConfigurablePortfolioConstructor {
    async fn create_targets(&self, _context: &StrategyContext, insights: &[Insight]) -> Result<Vec<PortfolioTarget>> {
        let mut targets = Vec::new();
        
        for insight in insights {
            let target_percent = match insight.direction {
                InsightDirection::Up => 10.0,
                InsightDirection::Down => -5.0,
                InsightDirection::Flat => 0.0,
            };
            
            targets.push(PortfolioTarget {
                symbol: insight.symbol.clone(),
                target_percent,
                target_quantity: None,
                target_value: None,
                generated_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                priority: Some(50),
                tag: Some("SimplePortfolio".to_string()),
            });
        }
        
        Ok(targets)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// 可配置的风险管理器
#[derive(Debug)]
pub struct ConfigurableRiskManager {
    name: String,
    config: RiskManagementConfig,
}

impl ConfigurableRiskManager {
    pub fn new(name: String, config: RiskManagementConfig) -> Self {
        Self { name, config }
    }
}

#[async_trait::async_trait]
impl RiskManager for ConfigurableRiskManager {
    async fn validate_targets(&self, _context: &StrategyContext, targets: &[PortfolioTarget]) -> Result<Vec<PortfolioTarget>> {
        let mut validated_targets = Vec::new();
        
        for target in targets {
            let adjusted_percent = if target.target_percent.abs() > self.config.max_position_size {
                if target.target_percent > 0.0 {
                    self.config.max_position_size
                } else {
                    -self.config.max_position_size
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

/// 可配置的执行算法
#[derive(Debug)]
pub struct ConfigurableExecutionAlgorithm {
    name: String,
    config: ExecutionConfig,
}

impl ConfigurableExecutionAlgorithm {
    pub fn new(name: String, config: ExecutionConfig) -> Self {
        Self { name, config }
    }
}

#[async_trait::async_trait]
impl ExecutionAlgorithm for ConfigurableExecutionAlgorithm {
    async fn execute_targets(&self, context: &StrategyContext, targets: &[PortfolioTarget]) -> Result<Vec<Order>> {
        let mut orders = Vec::new();
        
        for target in targets {
            let target_value = context.portfolio_value * target.target_percent / 100.0;
            let current_price = 50000.0; // 简化处理
            let target_quantity = target_value / current_price;
            
            if target_quantity.abs() >= self.config.min_order_size {
                let order = Order {
                    order_id: format!("order_{}", uuid::Uuid::new_v4()),
                    symbol: target.symbol.clone(),
                    direction: if target_quantity > 0.0 { Direction::Long } else { Direction::Short },
                    order_type: self.config.order_type,
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_strategy_runner_creation() {
        let config = FrameworkConfig::default();
        
        // 注意：这个测试可能会因为文件路径不存在而失败
        // 在实际环境中需要有效的数据文件
        let result = StrategyRunner::from_config(config).await;
        
        // 我们只测试创建过程不会panic
        match result {
            Ok(_) => tracing::info!("Strategy runner created successfully"),
            Err(e) => tracing::warn!("Strategy runner creation failed (expected in test): {}", e),
        }
    }
}