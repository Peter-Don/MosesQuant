//! MosesQuant 数据管理系统
//! 
//! 高性能数据获取、缓存和管理

pub mod csv_source;
pub mod binance;

pub use csv_source::CsvDataSource;
pub use binance::{BinanceConnector, BinanceConfig};

use crate::types::*;
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// 数据源接口
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn get_bars(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>>;
    async fn get_ticks(&self, symbol: &Symbol, count: usize) -> Result<Vec<Tick>>;
    async fn subscribe_market_data(&self, symbols: Vec<Symbol>) -> Result<mpsc::UnboundedReceiver<MarketData>>;
    fn name(&self) -> &str;
}

/// 数据管理器
pub struct DataManager {
    sources: Arc<RwLock<HashMap<String, Arc<dyn DataSource>>>>,
    cache: Arc<RwLock<HashMap<String, Vec<MarketData>>>>,
    stats: Arc<RwLock<DataStats>>,
}

impl Default for DataManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DataManager {
    pub fn new() -> Self {
        Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DataStats::default())),
        }
    }
    
    pub async fn register_source(&self, name: String, source: Arc<dyn DataSource>) -> Result<()> {
        let mut sources = self.sources.write().await;
        sources.insert(name.clone(), source);
        
        tracing::info!("Registered data source: {}", name);
        Ok(())
    }
    
    pub async fn get_bars(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>> {
        // 更新请求统计
        {
            let mut stats = self.stats.write().await;
            stats.requests += 1;
        }
        
        // 先检查缓存
        let cache_key = format!("bars_{}_{}", symbol.full_name(), count);
        {
            let cache = self.cache.read().await;
            if let Some(cached_data) = cache.get(&cache_key) {
                let bars: Vec<Bar> = cached_data.iter()
                    .filter_map(|data| match data {
                        MarketData::Bar(bar) => Some(bar.clone()),
                        _ => None,
                    })
                    .collect();
                
                if bars.len() >= count {
                    // 更新缓存命中统计
                    let mut stats = self.stats.write().await;
                    stats.cache_hits += 1;
                    return Ok(bars.into_iter().take(count).collect());
                }
            }
        }
        
        // 从数据源获取
        let sources = self.sources.read().await;
        for source in sources.values() {
            match source.get_bars(symbol, count).await {
                Ok(bars) => {
                    // 缓存数据
                    let mut cache = self.cache.write().await;
                    let market_data: Vec<MarketData> = bars.iter()
                        .map(|bar| MarketData::Bar(bar.clone()))
                        .collect();
                    cache.insert(cache_key, market_data);
                    
                    // 更新缓存未命中统计
                    let mut stats = self.stats.write().await;
                    stats.cache_misses += 1;
                    
                    return Ok(bars);
                }
                Err(e) => {
                    tracing::warn!("Failed to get bars from source {}: {}", source.name(), e);
                }
            }
        }
        
        Err(crate::CzscError::data("No data source available"))
    }
    
    pub async fn get_ticks(&self, symbol: &Symbol, count: usize) -> Result<Vec<Tick>> {
        // 更新请求统计
        {
            let mut stats = self.stats.write().await;
            stats.requests += 1;
        }
        
        let sources = self.sources.read().await;
        for source in sources.values() {
            match source.get_ticks(symbol, count).await {
                Ok(ticks) => {
                    return Ok(ticks);
                }
                Err(e) => {
                    tracing::warn!("Failed to get ticks from source {}: {}", source.name(), e);
                }
            }
        }
        
        Err(crate::CzscError::data("No data source available"))
    }
    
    pub async fn subscribe_market_data(&self, symbols: Vec<Symbol>) -> Result<mpsc::UnboundedReceiver<MarketData>> {
        let sources = self.sources.read().await;
        for source in sources.values() {
            match source.subscribe_market_data(symbols.clone()).await {
                Ok(receiver) => return Ok(receiver),
                Err(e) => {
                    tracing::warn!("Failed to subscribe to market data from source {}: {}", source.name(), e);
                }
            }
        }
        
        Err(crate::CzscError::data("No data source available for subscription"))
    }
    
    pub async fn get_stats(&self) -> DataStats {
        self.stats.read().await.clone()
    }
}

/// 数据统计
#[derive(Debug, Clone, Default)]
pub struct DataStats {
    pub requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl DataStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.requests > 0 {
            self.cache_hits as f64 / self.requests as f64
        } else {
            0.0
        }
    }
}

/// 内存数据源实现（用于测试）
#[derive(Debug)]
pub struct MemoryDataSource {
    name: String,
    bars: Arc<RwLock<HashMap<String, Vec<Bar>>>>,
    ticks: Arc<RwLock<HashMap<String, Vec<Tick>>>>,
}

impl MemoryDataSource {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bars: Arc::new(RwLock::new(HashMap::new())),
            ticks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn add_bars(&self, symbol: &Symbol, bars: Vec<Bar>) -> Result<()> {
        let mut bars_storage = self.bars.write().await;
        bars_storage.insert(symbol.full_name(), bars);
        Ok(())
    }
    
    pub async fn add_ticks(&self, symbol: &Symbol, ticks: Vec<Tick>) -> Result<()> {
        let mut ticks_storage = self.ticks.write().await;
        ticks_storage.insert(symbol.full_name(), ticks);
        Ok(())
    }
}

#[async_trait]
impl DataSource for MemoryDataSource {
    async fn get_bars(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>> {
        let bars_storage = self.bars.read().await;
        if let Some(bars) = bars_storage.get(&symbol.full_name()) {
            Ok(bars.iter().take(count).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn get_ticks(&self, symbol: &Symbol, count: usize) -> Result<Vec<Tick>> {
        let ticks_storage = self.ticks.read().await;
        if let Some(ticks) = ticks_storage.get(&symbol.full_name()) {
            Ok(ticks.iter().take(count).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn subscribe_market_data(&self, _symbols: Vec<Symbol>) -> Result<mpsc::UnboundedReceiver<MarketData>> {
        let (_, receiver) = mpsc::unbounded_channel();
        Ok(receiver)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_data_manager() {
        let manager = DataManager::new();
        
        // 创建内存数据源
        let source = Arc::new(MemoryDataSource::new("test_source".to_string()));
        
        // 添加测试数据
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let bars = vec![Bar {
            symbol: symbol.clone(),
            timestamp_ns: 1000000000,
            open: 50000.0,
            high: 51000.0,
            low: 49000.0,
            close: 50500.0,
            volume: 100.0,
        }];
        
        source.add_bars(&symbol, bars).await.unwrap();
        
        // 注册数据源
        manager.register_source("test".to_string(), source).await.unwrap();
        
        // 获取数据
        let result = manager.get_bars(&symbol, 10).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].open, 50000.0);
        assert_eq!(result[0].close, 50500.0);
        
        // 检查统计
        let stats = manager.get_stats().await;
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.cache_misses, 1);
        
        // 获取ticks数据
        let ticks = manager.get_ticks(&symbol, 10).await.unwrap();
        assert_eq!(ticks.len(), 0); // 没有添加tick数据
        
        // 检查更新后的统计
        let stats2 = manager.get_stats().await;
        assert_eq!(stats2.requests, 2);
    }
}