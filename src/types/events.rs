//! 事件系统相关类型定义

use super::*;

/// 事件基础特征
pub trait Event: Send + Sync + Any {
    fn event_type(&self) -> &'static str;
    fn timestamp(&self) -> TimestampNs;
    fn priority(&self) -> EventPriority { EventPriority::Normal }
    fn source(&self) -> &str;
    
    /// 类型擦除转换
    fn as_any(&self) -> &dyn Any;
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Critical = 0,   // 关键事件，最高优先级
    High = 1,       // 高优先级
    Normal = 2,     // 普通优先级
    Low = 3,        // 低优先级
}

/// 市场数据事件
#[derive(Debug, Clone)]
pub struct MarketDataEvent {
    pub symbol: Symbol,
    pub data: MarketDataPayload,
    pub timestamp: TimestampNs,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum MarketDataPayload {
    Bar(Box<Bar>),
    Tick(Box<Tick>),
    OrderBook(Box<OrderBook>),
    Trade(Box<Trade>),
}

impl Event for MarketDataEvent {
    fn event_type(&self) -> &'static str { "MarketData" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { EventPriority::High }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 交易事件
#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub trade: Trade,
    pub timestamp: TimestampNs,
    pub source: String,
}

impl Event for TradeEvent {
    fn event_type(&self) -> &'static str { "Trade" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { EventPriority::Critical }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 订单状态变更事件
#[derive(Debug, Clone)]
pub struct OrderStatusEvent {
    pub order: Order,
    pub old_status: OrderStatus,
    pub new_status: OrderStatus,
    pub timestamp: TimestampNs,
    pub source: String,
}

impl Event for OrderStatusEvent {
    fn event_type(&self) -> &'static str { "OrderStatus" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { EventPriority::High }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 风险事件
#[derive(Debug, Clone)]
pub struct RiskEvent {
    pub risk_type: RiskType,
    pub severity: RiskSeverity,
    pub message: String,
    pub affected_positions: Vec<PositionId>,
    pub timestamp: TimestampNs,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum RiskType {
    PositionLimit,
    DailyLoss,
    Drawdown,
    Volatility,
    Liquidity,
    Concentration,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl Event for RiskEvent {
    fn event_type(&self) -> &'static str { "Risk" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { 
        match self.severity {
            RiskSeverity::Emergency => EventPriority::Critical,
            RiskSeverity::Critical => EventPriority::High,
            RiskSeverity::Warning => EventPriority::Normal,
            RiskSeverity::Info => EventPriority::Low,
        }
    }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 系统事件
#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub event_type: SystemEventType,
    pub message: String,
    pub timestamp: TimestampNs,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum SystemEventType {
    Startup,
    Shutdown,
    PluginLoaded,
    PluginUnloaded,
    ConfigChanged,
    HealthCheck,
    PerformanceAlert,
}

impl Event for SystemEvent {
    fn event_type(&self) -> &'static str { "System" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { EventPriority::Normal }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 策略事件
#[derive(Debug, Clone)]
pub struct StrategyEvent {
    pub strategy_id: StrategyId,
    pub event_type: StrategyEventType,
    pub message: String,
    pub timestamp: TimestampNs,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum StrategyEventType {
    SignalGenerated,
    OrderPlaced,
    PositionOpened,
    PositionClosed,
    StopLossTriggered,
    TakeProfitTriggered,
    Error,
}

impl Event for StrategyEvent {
    fn event_type(&self) -> &'static str { "Strategy" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { 
        match self.event_type {
            StrategyEventType::Error => EventPriority::High,
            StrategyEventType::StopLossTriggered | StrategyEventType::TakeProfitTriggered => EventPriority::High,
            _ => EventPriority::Normal,
        }
    }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

/// 定时器事件
#[derive(Debug, Clone)]
pub struct TimerEvent {
    pub timer_id: String,
    pub interval: std::time::Duration,
    pub timestamp: TimestampNs,
    pub source: String,
}

impl Event for TimerEvent {
    fn event_type(&self) -> &'static str { "Timer" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn priority(&self) -> EventPriority { EventPriority::Low }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_event_priority_ordering() {
        assert!(EventPriority::Critical < EventPriority::High);
        assert!(EventPriority::High < EventPriority::Normal);
        assert!(EventPriority::Normal < EventPriority::Low);
    }
    
    #[test]
    fn test_market_data_event() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let bar = Bar {
            symbol: symbol.clone(),
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
        
        let event = MarketDataEvent {
            symbol: symbol.clone(),
            data: MarketDataPayload::Bar(Box::new(bar)),
            timestamp: 1000000000,
            source: "test_source".to_string(),
        };
        
        assert_eq!(event.event_type(), "MarketData");
        assert_eq!(event.timestamp(), 1000000000);
        assert_eq!(event.priority(), EventPriority::High);
        assert_eq!(event.source(), "test_source");
    }
    
    #[test]
    fn test_risk_event_priority() {
        let event = RiskEvent {
            risk_type: RiskType::PositionLimit,
            severity: RiskSeverity::Emergency,
            message: "Position limit exceeded".to_string(),
            affected_positions: vec!["pos_1".to_string()],
            timestamp: 1000000000,
            source: "risk_manager".to_string(),
        };
        
        assert_eq!(event.priority(), EventPriority::Critical);
        
        let warning_event = RiskEvent {
            severity: RiskSeverity::Warning,
            ..event
        };
        
        assert_eq!(warning_event.priority(), EventPriority::Normal);
    }
    
    #[test]
    fn test_order_status_event() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let order = Order::market_buy(symbol, Decimal::from(100));
        
        let event = OrderStatusEvent {
            order: order.clone(),
            old_status: OrderStatus::Pending,
            new_status: OrderStatus::Filled,
            timestamp: 1000000000,
            source: "order_manager".to_string(),
        };
        
        assert_eq!(event.event_type(), "OrderStatus");
        assert_eq!(event.priority(), EventPriority::High);
        assert_eq!(event.old_status, OrderStatus::Pending);
        assert_eq!(event.new_status, OrderStatus::Filled);
    }
    
    #[test]
    fn test_strategy_event_priority() {
        let normal_event = StrategyEvent {
            strategy_id: "strategy_1".to_string(),
            event_type: StrategyEventType::SignalGenerated,
            message: "Signal generated".to_string(),
            timestamp: 1000000000,
            source: "strategy".to_string(),
        };
        
        assert_eq!(normal_event.priority(), EventPriority::Normal);
        
        let error_event = StrategyEvent {
            event_type: StrategyEventType::Error,
            ..normal_event
        };
        
        assert_eq!(error_event.priority(), EventPriority::High);
        
        let stop_loss_event = StrategyEvent {
            event_type: StrategyEventType::StopLossTriggered,
            ..normal_event
        };
        
        assert_eq!(stop_loss_event.priority(), EventPriority::High);
    }
}