//! MosesQuant 核心数据类型模块
//! 
//! 基于零成本抽象原则设计的高性能数据结构
//! 使用高精度Decimal类型避免浮点误差，确保金融计算的准确性

pub mod market_data;
pub mod trading;
pub mod events;

// 重新导出所有公共类型
pub use market_data::*;
pub use trading::*;
pub use events::*;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::any::Any;
use std::str::FromStr;

/// 统一的价格类型 - 使用高精度Decimal避免浮点误差
pub type Price = Decimal;

/// 统一的数量类型
pub type Quantity = Decimal;

/// 纳秒时间戳 - 支持高精度时间
pub type TimestampNs = i64;

/// 唯一标识符类型
pub type OrderId = String;
pub type TradeId = String;
pub type PluginId = String;
pub type StrategyId = String;
pub type PositionId = String;
pub type DataSourceId = String;

/// 资产类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    Spot,           // 现货
    Future,         // 期货
    Option,         // 期权
    Swap,           // 掉期
    Bond,           // 债券
    Index,          // 指数
    Commodity,      // 大宗商品
    Crypto,         // 加密货币
    Forex,          // 外汇
    Stock,          // 股票
}

/// 方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Buy,
    Sell,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Direction::Buy => Direction::Sell,
            Direction::Sell => Direction::Buy,
        }
    }
    
    pub fn sign(self) -> i8 {
        match self {
            Direction::Buy => 1,
            Direction::Sell => -1,
        }
    }
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,        // 等待中
    Submitted,      // 已提交
    PartiallyFilled,// 部分成交
    Filled,         // 完全成交
    Cancelled,      // 已撤销
    Rejected,       // 已拒绝
    Expired,        // 已过期
}

/// 订单类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    Market,                     // 市价单
    Limit,                      // 限价单
    Stop,                       // 止损单
    StopLimit,                  // 止损限价单
    TrailingStop,               // 跟踪止损
    FillOrKill,                 // 全部成交或撤销
    ImmediateOrCancel,          // 立即成交或撤销
}

/// 洞见方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightDirection {
    Up,
    Down,
    Flat,
}

/// 符号标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub code: String,           // 代码，如 "BTCUSDT"
    pub exchange: String,       // 交易所，如 "binance"
    pub asset_type: AssetType,  // 资产类型
}

impl Symbol {
    pub fn new(code: &str, exchange: &str, asset_type: AssetType) -> Self {
        Self {
            code: code.to_uppercase(),
            exchange: exchange.to_lowercase(),
            asset_type,
        }
    }
    
    /// 生成唯一标识符
    pub fn unique_id(&self) -> String {
        format!("{}:{}:{:?}", self.exchange, self.code, self.asset_type)
    }
    
    /// 向后兼容的全名方法
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.code, self.exchange)
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.unique_id())
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
            MarketData::Tick(tick) => tick.timestamp,
            MarketData::Bar(bar) => bar.timestamp,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        assert_eq!(symbol.full_name(), "BTCUSDT.binance");
        assert_eq!(symbol.unique_id(), "binance:BTCUSDT:Crypto");
        assert_eq!(symbol.code, "BTCUSDT");
        assert_eq!(symbol.exchange, "binance");
        assert_eq!(symbol.asset_type, AssetType::Crypto);
    }
    
    #[test]
    fn test_direction_operations() {
        assert_eq!(Direction::Buy.opposite(), Direction::Sell);
        assert_eq!(Direction::Sell.opposite(), Direction::Buy);
        assert_eq!(Direction::Buy.sign(), 1);
        assert_eq!(Direction::Sell.sign(), -1);
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
        assert_eq!(insight.score(), 0.72); // 0.8 * 0.9
    }
    
    #[test]
    fn test_market_data_operations() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let tick = Tick {
            symbol: symbol.clone(),
            timestamp: 1000000000,
            bid_price: Decimal::from_str("50000.0").unwrap(),
            ask_price: Decimal::from_str("50001.0").unwrap(),
            bid_size: Decimal::from(10),
            ask_size: Decimal::from(5),
            last_price: None,
            last_size: None,
        };
        
        let market_data = MarketData::Tick(tick);
        assert_eq!(market_data.symbol(), &symbol);
        assert_eq!(market_data.timestamp(), 1000000000);
    }
}