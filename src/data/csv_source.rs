//! CSV数据源实现
//! 
//! 支持从CSV文件读取历史K线数据

use crate::data::DataSource;
use crate::types::*;
use crate::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use chrono::DateTime;

/// CSV数据源
#[derive(Debug)]
pub struct CsvDataSource {
    name: String,
    file_path: String,
    symbol: Symbol,
    data_cache: std::sync::Arc<tokio::sync::RwLock<Vec<Bar>>>,
}

impl CsvDataSource {
    /// 创建新的CSV数据源
    pub fn new(name: String, file_path: String, symbol: Symbol) -> Self {
        Self {
            name,
            file_path,
            symbol,
            data_cache: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
    
    /// 加载CSV数据
    pub async fn load_data(&self) -> Result<()> {
        let file_content = tokio::fs::read_to_string(&self.file_path).await
            .map_err(|e| crate::CzscError::data(&format!("Failed to read CSV file {}: {}", self.file_path, e)))?;
        
        let mut bars = Vec::new();
        let mut lines = file_content.lines();
        
        // 跳过标题行（如果存在）
        if let Some(first_line) = lines.next() {
            if !first_line.starts_with(char::is_numeric) {
                // 如果第一行不是数字开头，认为是标题行，跳过
            } else {
                // 否则解析第一行
                if let Ok(bar) = self.parse_csv_line(first_line) {
                    bars.push(bar);
                }
            }
        }
        
        // 解析剩余行
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            
            match self.parse_csv_line(line) {
                Ok(bar) => bars.push(bar),
                Err(e) => {
                    tracing::warn!("Failed to parse CSV line '{}': {}", line, e);
                }
            }
        }
        
        // 按时间排序
        bars.sort_by(|a, b| a.timestamp_ns.cmp(&b.timestamp_ns));
        
        // 缓存数据
        let mut cache = self.data_cache.write().await;
        *cache = bars;
        
        tracing::info!("Loaded {} bars from CSV file: {}", cache.len(), self.file_path);
        Ok(())
    }
    
    /// 解析CSV行
    fn parse_csv_line(&self, line: &str) -> Result<Bar> {
        let parts: Vec<&str> = line.split(',').collect();
        
        if parts.len() < 6 {
            return Err(crate::CzscError::data(&format!("Invalid CSV line format: {}", line)));
        }
        
        // 尝试不同的CSV格式
        // 格式1: timestamp,open,high,low,close,volume
        // 格式2: open_time,open,high,low,close,volume,close_time,...
        
        let timestamp_str = parts[0];
        let open: f64 = parts[1].parse()
            .map_err(|e| crate::CzscError::data(&format!("Invalid open price: {}", e)))?;
        let high: f64 = parts[2].parse()
            .map_err(|e| crate::CzscError::data(&format!("Invalid high price: {}", e)))?;
        let low: f64 = parts[3].parse()
            .map_err(|e| crate::CzscError::data(&format!("Invalid low price: {}", e)))?;
        let close: f64 = parts[4].parse()
            .map_err(|e| crate::CzscError::data(&format!("Invalid close price: {}", e)))?;
        let volume: f64 = parts[5].parse()
            .map_err(|e| crate::CzscError::data(&format!("Invalid volume: {}", e)))?;
        
        // 解析时间戳
        let timestamp_ns = self.parse_timestamp(timestamp_str)?;
        
        Ok(Bar {
            symbol: self.symbol.clone(),
            timestamp_ns,
            open,
            high,
            low,
            close,
            volume,
        })
    }
    
    /// 解析时间戳
    fn parse_timestamp(&self, timestamp_str: &str) -> Result<TimestampNs> {
        // 尝试不同的时间戳格式
        
        // 格式1: Unix毫秒时间戳
        if let Ok(millis) = timestamp_str.parse::<i64>() {
            if millis > 1000000000000 {
                // 毫秒时间戳
                return Ok(millis * 1_000_000);
            } else if millis > 1000000000 {
                // 秒时间戳
                return Ok(millis * 1_000_000_000);
            }
        }
        
        // 格式2: ISO8601字符串
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
            return Ok(dt.timestamp_nanos_opt().unwrap_or(0));
        }
        
        // 格式3: 自定义格式 YYYY-MM-DD HH:MM:SS
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt.and_utc().timestamp_nanos_opt().unwrap_or(0));
        }
        
        Err(crate::CzscError::data(&format!("Unsupported timestamp format: {}", timestamp_str)))
    }
    
    /// 获取数据数量
    pub async fn get_data_count(&self) -> usize {
        self.data_cache.read().await.len()
    }
}

#[async_trait]
impl DataSource for CsvDataSource {
    async fn get_bars(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>> {
        if symbol != &self.symbol {
            return Ok(Vec::new());
        }
        
        let cache = self.data_cache.read().await;
        
        if cache.is_empty() {
            return Ok(Vec::new());
        }
        
        // 返回最后count条数据
        let start_idx = if cache.len() > count {
            cache.len() - count
        } else {
            0
        };
        
        Ok(cache[start_idx..].to_vec())
    }
    
    async fn get_ticks(&self, _symbol: &Symbol, _count: usize) -> Result<Vec<Tick>> {
        // CSV通常不包含tick数据
        Ok(Vec::new())
    }
    
    async fn subscribe_market_data(&self, _symbols: Vec<Symbol>) -> Result<mpsc::UnboundedReceiver<MarketData>> {
        // CSV数据源不支持实时订阅
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
    async fn test_csv_data_source() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let csv_source = CsvDataSource::new(
            "test_csv".to_string(),
            "test_data.csv".to_string(),
            symbol.clone()
        );
        
        // 测试时间戳解析
        assert!(csv_source.parse_timestamp("1609459200000").is_ok()); // 毫秒时间戳
        assert!(csv_source.parse_timestamp("1609459200").is_ok()); // 秒时间戳
        assert!(csv_source.parse_timestamp("2021-01-01 00:00:00").is_ok()); // 字符串格式
        
        // 测试CSV行解析
        let line = "1609459200000,50000.0,51000.0,49000.0,50500.0,100.5";
        let bar = csv_source.parse_csv_line(line).unwrap();
        assert_eq!(bar.open, 50000.0);
        assert_eq!(bar.high, 51000.0);
        assert_eq!(bar.low, 49000.0);
        assert_eq!(bar.close, 50500.0);
        assert_eq!(bar.volume, 100.5);
        assert_eq!(bar.symbol, symbol);
    }
}