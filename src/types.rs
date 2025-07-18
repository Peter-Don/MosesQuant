//! MosesQuant 核心数据类型
//! 
//! 基于零成本抽象原则设计的高性能数据结构

use serde::{Deserialize, Serialize};
use std::fmt;

/// 基础数值类型
pub type Price = f64;
pub type Quantity = f64;
pub type TimestampNs = i64;
pub type OrderId = String;

/// 资产类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Stock,
    Future,
    Option,
    Forex,
    Crypto,
    Bond,
    Index,
    Commodity,
}

/// 交易方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Long,
    Short,
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

/// 洞见方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightDirection {
    Up,
    Down,
    Flat,
}

/// 交易标的
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub value: String,
    pub market: String,
    pub asset_type: AssetType,
}

impl Symbol {
    pub fn new(value: &str, market: &str, asset_type: AssetType) -> Self {
        Self {
            value: value.to_string(),
            market: market.to_string(),
            asset_type,
        }
    }
    
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.value, self.market)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

/// 市场数据 - Tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub timestamp_ns: TimestampNs,
    pub last_price: Price,
    pub volume: Quantity,
    pub bid_price: Price,
    pub ask_price: Price,
    pub bid_volume: Quantity,
    pub ask_volume: Quantity,
}

/// 市场数据 - Bar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub symbol: Symbol,
    pub timestamp_ns: TimestampNs,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Quantity,
}

/// 订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub direction: Direction,
    pub order_type: OrderType,
    pub price: Option<Price>,
    pub quantity: Quantity,
    pub filled_quantity: Quantity,
    pub status: OrderStatus,
    pub created_time: TimestampNs,
    pub updated_time: TimestampNs,
}

/// 成交记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: String,
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub direction: Direction,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp_ns: TimestampNs,
}

/// 持仓
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub quantity: Quantity,
    pub average_price: Price,
    pub market_price: Price,
    pub unrealized_pnl: Price,
    pub realized_pnl: Price,
    pub updated_time: TimestampNs,
}

/// Alpha洞见
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub symbol: Symbol,
    pub direction: InsightDirection,
    pub magnitude: Option<f64>,
    pub confidence: Option<f64>,
    pub period: Option<i64>,
    pub generated_time: TimestampNs,
    pub expiry_time: Option<TimestampNs>,
}

impl Insight {
    pub fn is_expired(&self, current_time: TimestampNs) -> bool {
        if let Some(expiry) = self.expiry_time {
            current_time > expiry
        } else {
            false
        }
    }
    
    /// 计算洞见评分
    pub fn score(&self) -> f64 {
        let magnitude = self.magnitude.unwrap_or(0.0);
        let confidence = self.confidence.unwrap_or(0.0);
        magnitude * confidence
    }
}

/// 投资组合目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTarget {
    pub symbol: Symbol,
    pub target_percent: f64,
    pub target_quantity: Option<Quantity>,
    pub target_value: Option<Price>,
    pub generated_time: TimestampNs,
    pub priority: Option<u8>,
    pub tag: Option<String>,
}

/// 系统事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Tick(Tick),
    Bar(Bar),
    OrderUpdate(Order),
    Trade(Trade),
    Timer(TimestampNs),
    Shutdown,
}

impl Event {
    pub fn event_type(&self) -> &str {
        match self {
            Event::Tick(_) => "Tick",
            Event::Bar(_) => "Bar",
            Event::OrderUpdate(_) => "OrderUpdate",
            Event::Trade(_) => "Trade",
            Event::Timer(_) => "Timer",
            Event::Shutdown => "Shutdown",
        }
    }
}

/// 统一市场数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketData {
    Tick(Tick),
    Bar(Bar),
}

impl MarketData {
    pub fn symbol(&self) -> &Symbol {
        match self {
            MarketData::Tick(tick) => &tick.symbol,
            MarketData::Bar(bar) => &bar.symbol,
        }
    }
    
    pub fn timestamp(&self) -> TimestampNs {
        match self {
            MarketData::Tick(tick) => tick.timestamp_ns,
            MarketData::Bar(bar) => bar.timestamp_ns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        assert_eq!(symbol.full_name(), "BTCUSDT.BINANCE");
        assert_eq!(symbol.value, "BTCUSDT");
        assert_eq!(symbol.market, "BINANCE");
        assert_eq!(symbol.asset_type, AssetType::Crypto);
    }
    
    #[test]
    fn test_insight_expiry() {
        let insight = Insight {
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            direction: InsightDirection::Up,
            magnitude: Some(0.8),
            confidence: Some(0.9),
            period: None,
            generated_time: 1000000000,
            expiry_time: Some(2000000000),
        };
        
        assert!(!insight.is_expired(1500000000));
        assert!(insight.is_expired(2500000000));
    }
}