//! 市场数据相关类型定义

use super::*;

/// 时间周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeFrame {
    Tick,       // 逐笔
    Second1,    // 1秒
    Second5,    // 5秒
    Second15,   // 15秒
    Second30,   // 30秒
    Minute1,    // 1分钟
    Minute5,    // 5分钟
    Minute15,   // 15分钟
    Minute30,   // 30分钟
    Hour1,      // 1小时
    Hour4,      // 4小时
    Hour12,     // 12小时
    Day1,       // 1天
    Week1,      // 1周
    Month1,     // 1月
}

impl TimeFrame {
    /// 获取时间周期的秒数
    pub fn to_seconds(self) -> Option<i64> {
        match self {
            TimeFrame::Tick => None,
            TimeFrame::Second1 => Some(1),
            TimeFrame::Second5 => Some(5),
            TimeFrame::Second15 => Some(15),
            TimeFrame::Second30 => Some(30),
            TimeFrame::Minute1 => Some(60),
            TimeFrame::Minute5 => Some(300),
            TimeFrame::Minute15 => Some(900),
            TimeFrame::Minute30 => Some(1800),
            TimeFrame::Hour1 => Some(3600),
            TimeFrame::Hour4 => Some(14400),
            TimeFrame::Hour12 => Some(43200),
            TimeFrame::Day1 => Some(86400),
            TimeFrame::Week1 => Some(604800),
            TimeFrame::Month1 => Some(2592000), // 30天近似
        }
    }
}

/// Tick数据
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub timestamp: TimestampNs,
    pub bid_price: Price,
    pub ask_price: Price,
    pub bid_size: Quantity,
    pub ask_size: Quantity,
    pub last_price: Option<Price>,
    pub last_size: Option<Quantity>,
}

impl Tick {
    /// 计算买卖价差
    #[inline]
    pub fn spread(&self) -> Price {
        self.ask_price - self.bid_price
    }
    
    /// 计算中间价
    #[inline]
    pub fn mid_price(&self) -> Price {
        (self.bid_price + self.ask_price) / Decimal::from(2)
    }
    
    /// 计算买卖不平衡度
    #[inline]
    pub fn imbalance(&self) -> Decimal {
        if self.bid_size + self.ask_size == Decimal::ZERO {
            Decimal::ZERO
        } else {
            (self.bid_size - self.ask_size) / (self.bid_size + self.ask_size)
        }
    }
}

/// K线数据 (OHLCV)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[repr(C, align(32))]  // SIMD对齐优化
pub struct Bar {
    pub symbol: Symbol,
    pub timestamp: TimestampNs,
    pub timeframe: TimeFrame,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Quantity,
    pub turnover: Option<Price>,    // 成交额
    pub open_interest: Option<Quantity>, // 持仓量
}

impl Bar {
    /// 计算典型价格 (HLC/3)
    #[inline]
    pub fn typical_price(&self) -> Price {
        (self.high + self.low + self.close) / Decimal::from(3)
    }
    
    /// 计算加权价格 (HLCC/4)
    #[inline]
    pub fn weighted_price(&self) -> Price {
        (self.high + self.low + self.close * Decimal::from(2)) / Decimal::from(4)
    }
    
    /// 是否为看涨K线
    #[inline]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }
    
    /// 是否为看跌K线
    #[inline]
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }
    
    /// 计算实体大小
    #[inline]
    pub fn body_size(&self) -> Price {
        (self.close - self.open).abs()
    }
    
    /// 计算上影线长度
    #[inline]
    pub fn upper_shadow(&self) -> Price {
        self.high - self.open.max(self.close)
    }
    
    /// 计算下影线长度
    #[inline]
    pub fn lower_shadow(&self) -> Price {
        self.open.min(self.close) - self.low
    }
}

/// 深度数据
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: Symbol,
    pub timestamp: TimestampNs,
    pub bids: Vec<PriceLevel>,  // 买盘，按价格降序
    pub asks: Vec<PriceLevel>,  // 卖盘，按价格升序
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Price,
    pub size: Quantity,
}

impl OrderBook {
    /// 获取最佳买价
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }
    
    /// 获取最佳卖价
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }
    
    /// 计算买卖价差
    pub fn spread(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }
    
    /// 计算中间价
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid.price + ask.price) / Decimal::from(2)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_tick_calculations() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let tick = Tick {
            symbol,
            timestamp: 1000000000,
            bid_price: Decimal::from_str("50000.0").unwrap(),
            ask_price: Decimal::from_str("50001.0").unwrap(),
            bid_size: Decimal::from(10),
            ask_size: Decimal::from(5),
            last_price: None,
            last_size: None,
        };
        
        assert_eq!(tick.spread(), Decimal::from_str("1.0").unwrap());
        assert_eq!(tick.mid_price(), Decimal::from_str("50000.5").unwrap());
        
        let expected_imbalance = Decimal::from(5) / Decimal::from(15); // (10-5)/(10+5)
        assert_eq!(tick.imbalance(), expected_imbalance);
    }
    
    #[test]
    fn test_bar_analysis() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let bar = Bar {
            symbol,
            timestamp: 1000000000,
            timeframe: TimeFrame::Minute1,
            open: Decimal::from_str("50000.0").unwrap(),
            high: Decimal::from_str("50200.0").unwrap(),
            low: Decimal::from_str("49800.0").unwrap(),
            close: Decimal::from_str("50100.0").unwrap(),
            volume: Decimal::from(1000),
            turnover: None,
            open_interest: None,
        };
        
        assert!(bar.is_bullish());
        assert!(!bar.is_bearish());
        assert_eq!(bar.body_size(), Decimal::from_str("100.0").unwrap());
        
        // 典型价格: (50200 + 49800 + 50100) / 3 = 50033.333...
        let expected_typical = (Decimal::from_str("50200.0").unwrap() + 
                               Decimal::from_str("49800.0").unwrap() + 
                               Decimal::from_str("50100.0").unwrap()) / Decimal::from(3);
        assert_eq!(bar.typical_price(), expected_typical);
    }
    
    #[test]
    fn test_timeframe_seconds() {
        assert_eq!(TimeFrame::Tick.to_seconds(), None);
        assert_eq!(TimeFrame::Minute1.to_seconds(), Some(60));
        assert_eq!(TimeFrame::Hour1.to_seconds(), Some(3600));
        assert_eq!(TimeFrame::Day1.to_seconds(), Some(86400));
    }
    
    #[test]
    fn test_orderbook_operations() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let orderbook = OrderBook {
            symbol,
            timestamp: 1000000000,
            bids: vec![
                PriceLevel { price: Decimal::from_str("50000.0").unwrap(), size: Decimal::from(10) },
                PriceLevel { price: Decimal::from_str("49999.0").unwrap(), size: Decimal::from(5) },
            ],
            asks: vec![
                PriceLevel { price: Decimal::from_str("50001.0").unwrap(), size: Decimal::from(8) },
                PriceLevel { price: Decimal::from_str("50002.0").unwrap(), size: Decimal::from(3) },
            ],
        };
        
        assert_eq!(orderbook.best_bid().unwrap().price, Decimal::from_str("50000.0").unwrap());
        assert_eq!(orderbook.best_ask().unwrap().price, Decimal::from_str("50001.0").unwrap());
        assert_eq!(orderbook.spread().unwrap(), Decimal::from_str("1.0").unwrap());
        assert_eq!(orderbook.mid_price().unwrap(), Decimal::from_str("50000.5").unwrap());
    }
}