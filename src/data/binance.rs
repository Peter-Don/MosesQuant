//! Binance数字货币交易所连接器
//! 
//! 支持现货和期货交易，实时数据获取

use crate::data::DataSource;
use crate::types::*;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;

/// Binance API配置
#[derive(Debug, Clone)]
pub struct BinanceConfig {
    pub api_key: Option<String>,
    pub secret_key: Option<String>,
    pub base_url: String,
    pub ws_url: String,
    pub testnet: bool,
}

impl Default for BinanceConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            secret_key: None,
            base_url: "https://api.binance.com".to_string(),
            ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            testnet: false,
        }
    }
}

/// Binance连接器
#[derive(Debug)]
pub struct BinanceConnector {
    name: String,
    config: BinanceConfig,
    client: Client,
    connected: bool,
}

impl BinanceConnector {
    /// 创建新的Binance连接器
    pub fn new(name: String, config: BinanceConfig) -> Self {
        Self {
            name,
            config,
            client: Client::new(),
            connected: false,
        }
    }
    
    /// 连接到Binance
    pub async fn connect(&mut self) -> Result<()> {
        // 测试连接
        let url = format!("{}/api/v3/ping", self.config.base_url);
        let response = self.client.get(&url).send().await
            .map_err(|e| crate::CzscError::network(&format!("Failed to connect to Binance: {}", e)))?;
        
        if response.status().is_success() {
            self.connected = true;
            tracing::info!("Connected to Binance API");
            Ok(())
        } else {
            Err(crate::CzscError::network("Failed to connect to Binance API"))
        }
    }
    
    /// 获取交易对信息
    pub async fn get_exchange_info(&self) -> Result<Value> {
        let url = format!("{}/api/v3/exchangeInfo", self.config.base_url);
        let response = self.client.get(&url).send().await
            .map_err(|e| crate::CzscError::network(&format!("Failed to get exchange info: {}", e)))?;
        
        let data: Value = response.json().await
            .map_err(|e| crate::CzscError::data(&format!("Failed to parse exchange info: {}", e)))?;
        
        Ok(data)
    }
    
    /// 获取K线数据
    pub async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> Result<Vec<Bar>> {
        let url = format!(
            "{}/api/v3/klines?symbol={}&interval={}&limit={}",
            self.config.base_url, symbol, interval, limit
        );
        
        let response = self.client.get(&url).send().await
            .map_err(|e| crate::CzscError::network(&format!("Failed to get klines: {}", e)))?;
        
        let data: Value = response.json().await
            .map_err(|e| crate::CzscError::data(&format!("Failed to parse klines: {}", e)))?;
        
        let mut bars = Vec::new();
        
        if let Some(klines) = data.as_array() {
            for kline in klines {
                if let Some(kline_array) = kline.as_array() {
                    if kline_array.len() >= 6 {
                        let timestamp_ms = kline_array[0].as_i64().unwrap_or(0);
                        let open = kline_array[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                        let high = kline_array[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                        let low = kline_array[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                        let close = kline_array[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                        let volume = kline_array[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
                        
                        let bar = Bar {
                            symbol: Symbol::new(symbol, "BINANCE", AssetType::Crypto),
                            timestamp_ns: timestamp_ms * 1_000_000, // 转换为纳秒
                            open,
                            high,
                            low,
                            close,
                            volume,
                        };
                        
                        bars.push(bar);
                    }
                }
            }
        }
        
        Ok(bars)
    }
    
    /// 订阅WebSocket数据流
    pub async fn subscribe_websocket(&self, symbols: Vec<String>) -> Result<mpsc::UnboundedReceiver<MarketData>> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // 构建WebSocket订阅URL
        let streams: Vec<String> = symbols.iter()
            .map(|s| format!("{}@kline_1m", s.to_lowercase()))
            .collect();
        let stream_names = streams.join("/");
        let ws_url = format!("{}/stream?streams={}", self.config.ws_url, stream_names);
        
        let config = self.config.clone();
        tokio::spawn(async move {
            match Self::websocket_worker(ws_url, sender, config).await {
                Ok(_) => tracing::info!("WebSocket connection closed normally"),
                Err(e) => tracing::error!("WebSocket error: {}", e),
            }
        });
        
        Ok(receiver)
    }
    
    /// WebSocket工作线程
    async fn websocket_worker(
        url: String,
        sender: mpsc::UnboundedSender<MarketData>,
        _config: BinanceConfig,
    ) -> Result<()> {
        let (ws_stream, _) = connect_async(&url).await
            .map_err(|e| crate::CzscError::network(&format!("WebSocket connection failed: {}", e)))?;
        
        let (mut _write, mut read) = ws_stream.split();
        
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        if let Some(market_data) = Self::parse_websocket_message(&data) {
                            if sender.send(market_data).is_err() {
                                tracing::warn!("Failed to send WebSocket data");
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// 解析WebSocket消息
    fn parse_websocket_message(data: &Value) -> Option<MarketData> {
        if let Some(stream) = data.get("stream").and_then(|s| s.as_str()) {
            if stream.contains("kline") {
                if let Some(kline_data) = data.get("data").and_then(|d| d.get("k")) {
                    let symbol_str = kline_data.get("s")?.as_str()?;
                    let timestamp_ms = kline_data.get("t")?.as_i64()?;
                    let open = kline_data.get("o")?.as_str()?.parse::<f64>().ok()?;
                    let high = kline_data.get("h")?.as_str()?.parse::<f64>().ok()?;
                    let low = kline_data.get("l")?.as_str()?.parse::<f64>().ok()?;
                    let close = kline_data.get("c")?.as_str()?.parse::<f64>().ok()?;
                    let volume = kline_data.get("v")?.as_str()?.parse::<f64>().ok()?;
                    
                    let bar = Bar {
                        symbol: Symbol::new(symbol_str, "BINANCE", AssetType::Crypto),
                        timestamp_ns: timestamp_ms * 1_000_000,
                        open,
                        high,
                        low,
                        close,
                        volume,
                    };
                    
                    return Some(MarketData::Bar(bar));
                }
            }
        }
        
        None
    }
    
    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[async_trait]
impl DataSource for BinanceConnector {
    async fn get_bars(&self, symbol: &Symbol, count: usize) -> Result<Vec<Bar>> {
        if !self.connected {
            return Err(crate::CzscError::network("Not connected to Binance"));
        }
        
        // 将count限制在合理范围内
        let limit = std::cmp::min(count as u32, 1000);
        
        self.get_klines(&symbol.value, "1m", limit).await
    }
    
    async fn get_ticks(&self, _symbol: &Symbol, _count: usize) -> Result<Vec<Tick>> {
        // Binance API不直接提供tick数据，可以通过WebSocket获取
        Ok(Vec::new())
    }
    
    async fn subscribe_market_data(&self, symbols: Vec<Symbol>) -> Result<mpsc::UnboundedReceiver<MarketData>> {
        let symbol_strings: Vec<String> = symbols.iter()
            .map(|s| s.value.clone())
            .collect();
        
        self.subscribe_websocket(symbol_strings).await
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Binance响应数据结构
#[derive(Debug, Deserialize)]
pub struct BinanceKline {
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_binance_connector_creation() {
        let config = BinanceConfig::default();
        let connector = BinanceConnector::new("test_binance".to_string(), config);
        
        assert_eq!(connector.name(), "test_binance");
        assert!(!connector.is_connected());
    }
    
    #[test]
    fn test_websocket_message_parsing() {
        let json_data = serde_json::json!({
            "stream": "btcusdt@kline_1m",
            "data": {
                "k": {
                    "s": "BTCUSDT",
                    "t": 1609459200000i64,
                    "o": "50000.0",
                    "h": "51000.0",
                    "l": "49000.0",
                    "c": "50500.0",
                    "v": "100.5"
                }
            }
        });
        
        let market_data = BinanceConnector::parse_websocket_message(&json_data).unwrap();
        
        if let MarketData::Bar(bar) = market_data {
            assert_eq!(bar.symbol.value, "BTCUSDT");
            assert_eq!(bar.symbol.market, "BINANCE");
            assert_eq!(bar.open, 50000.0);
            assert_eq!(bar.close, 50500.0);
            assert_eq!(bar.volume, 100.5);
        } else {
            panic!("Expected Bar data");
        }
    }
}