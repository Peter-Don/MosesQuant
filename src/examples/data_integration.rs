//! MosesQuant 数据集成示例
//! 
//! 演示如何使用CSV历史数据和Binance实时数据

use crate::data::{DataManager, CsvDataSource, BinanceConnector, BinanceConfig};
use crate::types::*;
use crate::strategy::*;
use crate::events::EventBus;
use crate::Result;
use std::sync::Arc;
use std::collections::HashMap;

/// 数据集成示例
pub struct DataIntegrationExample {
    data_manager: DataManager,
    #[allow(dead_code)]
    event_bus: EventBus,
}

impl Default for DataIntegrationExample {
    fn default() -> Self {
        Self::new()
    }
}

impl DataIntegrationExample {
    /// 创建新的示例
    pub fn new() -> Self {
        Self {
            data_manager: DataManager::new(),
            event_bus: EventBus::new(),
        }
    }
    
    /// 设置CSV历史数据源
    pub async fn setup_csv_data_source(&self, csv_path: &str) -> Result<()> {
        // 创建BTCUSDT符号
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        
        // 创建CSV数据源
        let csv_source = CsvDataSource::new(
            "btc_historical".to_string(),
            csv_path.to_string(),
            symbol,
        );
        
        // 加载CSV数据
        csv_source.load_data().await?;
        
        // 注册到数据管理器
        self.data_manager.register_source(
            "csv_historical".to_string(),
            Arc::new(csv_source),
        ).await?;
        
        tracing::info!("CSV historical data source setup completed");
        Ok(())
    }
    
    /// 设置Binance实时数据源
    pub async fn setup_binance_data_source(&self) -> Result<()> {
        // 创建Binance配置（使用默认配置，不需要API密钥来获取公共数据）
        let config = BinanceConfig::default();
        
        // 创建Binance连接器
        let mut binance_connector = BinanceConnector::new(
            "binance_live".to_string(),
            config,
        );
        
        // 连接到Binance
        binance_connector.connect().await?;
        
        // 注册到数据管理器
        self.data_manager.register_source(
            "binance_live".to_string(),
            Arc::new(binance_connector),
        ).await?;
        
        tracing::info!("Binance live data source setup completed");
        Ok(())
    }
    
    /// 获取历史数据
    pub async fn get_historical_data(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>> {
        let bars = self.data_manager.get_bars(symbol, count).await?;
        
        tracing::info!("Retrieved {} historical bars for {}", bars.len(), symbol);
        if !bars.is_empty() {
            let first_bar = &bars[0];
            let last_bar = &bars[bars.len() - 1];
            tracing::info!(
                "Data range: first bar O:{} H:{} L:{} C:{} V:{}, last bar O:{} H:{} L:{} C:{} V:{}",
                first_bar.open,
                first_bar.high,
                first_bar.low,
                first_bar.close,
                first_bar.volume,
                last_bar.open,
                last_bar.high,
                last_bar.low,
                last_bar.close,
                last_bar.volume
            );
        }
        
        Ok(bars)
    }
    
    /// 运行回测策略
    pub async fn run_backtest_strategy(&self, symbol: &Symbol, bars: Vec<Bar>) -> Result<()> {
        // 创建策略上下文
        let _context = StrategyContext {
            strategy_id: "btc_trend_strategy".to_string(),
            portfolio_value: 100000.0,
            cash: 100000.0,
            positions: HashMap::new(),
            current_time: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            market_data: HashMap::new(),
        };
        
        // 创建简单的策略组件
        let _universe_selector = Arc::new(SimpleUniverseSelector::new(
            "btc_universe".to_string(),
            vec![symbol.clone()],
        ));
        
        let _alpha_model = Arc::new(SimpleAlphaModel::new(
            "trend_alpha".to_string(),
        ));
        
        // 这里可以添加更多的策略组件...
        
        tracing::info!("Starting backtest with {} bars", bars.len());
        
        // 简单的回测逻辑 - 遍历历史数据
        for (i, bar) in bars.iter().enumerate() {
            if i % 1000 == 0 {
                tracing::info!("Processing bar {}/{}: {} (O:{} H:{} L:{} C:{} V:{})", 
                    i + 1, 
                    bars.len(), 
                    bar.close,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume
                );
            }
            
            // 这里可以添加策略逻辑
            // 例如：检查买入/卖出信号，更新持仓，计算收益等
        }
        
        tracing::info!("Backtest completed");
        Ok(())
    }
    
    /// 订阅实时数据
    pub async fn subscribe_real_time_data(&self, symbols: Vec<Symbol>) -> Result<()> {
        let mut receiver = self.data_manager.subscribe_market_data(symbols.clone()).await?;
        
        tracing::info!("Subscribed to real-time data for {} symbols", symbols.len());
        
        // 启动实时数据处理任务
        tokio::spawn(async move {
            while let Some(market_data) = receiver.recv().await {
                match market_data {
                    MarketData::Bar(bar) => {
                        tracing::info!("Real-time bar: {} O:{} H:{} L:{} C:{} V:{}",
                            bar.symbol.value,
                            bar.open,
                            bar.high,
                            bar.low,
                            bar.close,
                            bar.volume
                        );
                    }
                    MarketData::Tick(tick) => {
                        tracing::info!("Real-time tick: {} price:{}",
                            tick.symbol.value,
                            tick.last_price
                        );
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 获取数据统计信息
    pub async fn get_data_stats(&self) -> Result<()> {
        let stats = self.data_manager.get_stats().await;
        
        tracing::info!("Data Manager Statistics:");
        tracing::info!("  Total requests: {}", stats.requests);
        tracing::info!("  Cache hits: {}", stats.cache_hits);
        tracing::info!("  Cache misses: {}", stats.cache_misses);
        tracing::info!("  Cache hit rate: {:.2}%", stats.cache_hit_rate() * 100.0);
        
        Ok(())
    }
}

/// 运行完整的数据集成示例
pub async fn run_example() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 创建示例实例
    let example = DataIntegrationExample::new();
    
    // 设置CSV历史数据源
    let csv_path = r"E:\code\QuantTrade\czsc_enhanced\test\data\BTCUSDT_1m_2023-09.csv";
    example.setup_csv_data_source(csv_path).await?;
    
    // 设置Binance实时数据源
    example.setup_binance_data_source().await?;
    
    // 创建BTCUSDT符号
    let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
    
    // 获取历史数据
    let bars = example.get_historical_data(&symbol, 1000).await?;
    
    // 运行回测策略
    if !bars.is_empty() {
        example.run_backtest_strategy(&symbol, bars).await?;
    }
    
    // 订阅实时数据（可选）
    // example.subscribe_real_time_data(vec![symbol]).await?;
    
    // 显示统计信息
    example.get_data_stats().await?;
    
    tracing::info!("Data integration example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_data_integration_example() {
        let example = DataIntegrationExample::new();
        
        // 测试创建
        assert!(example.data_manager.get_stats().await.requests == 0);
        
        // 这里可以添加更多测试...
    }
}

/// 主函数示例
#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<()> {
    run_example().await
}