//! 交易相关类型定义

use super::*;

/// 订单有效期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,    // Good Till Cancelled - 撤销前有效
    IOC,    // Immediate or Cancel - 立即成交或撤销
    FOK,    // Fill or Kill - 全部成交或撤销
    GTD,    // Good Till Date - 指定日期前有效
}

/// 订单结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub symbol: Symbol,
    pub direction: Direction,
    pub order_type: OrderType,
    pub quantity: Quantity,
    pub price: Option<Price>,       // 限价单需要价格
    pub stop_price: Option<Price>,  // 止损价格
    pub time_in_force: TimeInForce,
    pub status: OrderStatus,
    pub filled_quantity: Quantity,
    pub average_fill_price: Option<Price>,
    pub created_at: TimestampNs,
    pub updated_at: TimestampNs,
    pub strategy_id: Option<StrategyId>,
    pub client_order_id: Option<String>,
    pub metadata: OrderMetadata,
}

impl Order {
    /// 创建市价买单
    pub fn market_buy(symbol: Symbol, quantity: Quantity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol,
            direction: Direction::Buy,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::IOC,
            status: OrderStatus::Pending,
            filled_quantity: Decimal::ZERO,
            average_fill_price: None,
            created_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            updated_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            strategy_id: None,
            client_order_id: None,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// 创建限价卖单
    pub fn limit_sell(symbol: Symbol, quantity: Quantity, price: Price) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol,
            direction: Direction::Sell,
            order_type: OrderType::Limit,
            quantity,
            price: Some(price),
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            status: OrderStatus::Pending,
            filled_quantity: Decimal::ZERO,
            average_fill_price: None,
            created_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            updated_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            strategy_id: None,
            client_order_id: None,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// 是否完全成交
    #[inline]
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }
    
    /// 是否活跃订单
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartiallyFilled)
    }
    
    /// 剩余数量
    #[inline]
    pub fn remaining_quantity(&self) -> Quantity {
        self.quantity - self.filled_quantity
    }
}

/// 订单元数据
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderMetadata {
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub risk_params: Option<RiskParameters>,
    pub execution_params: Option<ExecutionParameters>,
}

/// 成交记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub id: TradeId,
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub direction: Direction,
    pub quantity: Quantity,
    pub price: Price,
    pub commission: Price,
    pub timestamp: TimestampNs,
    pub strategy_id: Option<StrategyId>,
}

impl Trade {
    /// 计算成交金额
    #[inline]
    pub fn notional(&self) -> Price {
        self.quantity * self.price
    }
    
    /// 计算净金额 (扣除手续费)
    #[inline]
    pub fn net_amount(&self) -> Price {
        match self.direction {
            Direction::Buy => self.notional() + self.commission,
            Direction::Sell => self.notional() - self.commission,
        }
    }
}

/// 持仓信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub id: PositionId,
    pub symbol: Symbol,
    pub direction: Direction,
    pub quantity: Quantity,
    pub average_price: Price,
    pub market_price: Price,
    pub unrealized_pnl: Price,
    pub realized_pnl: Price,
    pub created_at: TimestampNs,
    pub updated_at: TimestampNs,
    pub strategy_id: Option<StrategyId>,
}

impl Position {
    /// 计算市值
    #[inline]
    pub fn market_value(&self) -> Price {
        self.quantity * self.market_price
    }
    
    /// 计算成本
    #[inline]
    pub fn cost_basis(&self) -> Price {
        self.quantity * self.average_price
    }
    
    /// 更新市场价格并重算未实现盈亏
    pub fn update_market_price(&mut self, new_price: Price) {
        self.market_price = new_price;
        self.unrealized_pnl = match self.direction {
            Direction::Buy => (new_price - self.average_price) * self.quantity,
            Direction::Sell => (self.average_price - new_price) * self.quantity,
        };
        self.updated_at = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    }
}

/// 风险参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskParameters {
    pub max_position_size: Option<Quantity>,
    pub max_order_size: Option<Quantity>,
    pub stop_loss: Option<Price>,
    pub take_profit: Option<Price>,
    pub max_daily_loss: Option<Price>,
}

/// 执行参数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionParameters {
    pub urgency: ExecutionUrgency,
    pub slice_size: Option<Quantity>,
    pub time_limit: Option<TimestampNs>,
    pub min_fill_size: Option<Quantity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionUrgency {
    Low,        // 低紧急度，优先考虑成本
    Medium,     // 中等紧急度，平衡成本和速度
    High,       // 高紧急度，优先考虑速度
    Critical,   // 关键紧急度，立即执行
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn test_order_creation() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let order = Order::market_buy(symbol.clone(), Decimal::from(100));
        
        assert_eq!(order.symbol, symbol);
        assert_eq!(order.direction, Direction::Buy);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.quantity, Decimal::from(100));
        assert_eq!(order.status, OrderStatus::Pending);
        assert!(order.is_active());
        assert!(!order.is_filled());
        assert_eq!(order.remaining_quantity(), Decimal::from(100));
        
        let limit_order = Order::limit_sell(
            symbol.clone(), 
            Decimal::from(50), 
            Decimal::from_str("50000.0").unwrap()
        );
        
        assert_eq!(limit_order.direction, Direction::Sell);
        assert_eq!(limit_order.order_type, OrderType::Limit);
        assert_eq!(limit_order.price, Some(Decimal::from_str("50000.0").unwrap()));
    }
    
    #[test]
    fn test_trade_calculations() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let trade = Trade {
            id: "trade_1".to_string(),
            order_id: "order_1".to_string(),
            symbol,
            direction: Direction::Buy,
            quantity: Decimal::from(10),
            price: Decimal::from_str("50000.0").unwrap(),
            commission: Decimal::from_str("5.0").unwrap(),
            timestamp: 1000000000,
            strategy_id: None,
        };
        
        // notional = 10 * 50000 = 500000
        assert_eq!(trade.notional(), Decimal::from_str("500000.0").unwrap());
        
        // net_amount for buy = notional + commission = 500000 + 5 = 500005
        assert_eq!(trade.net_amount(), Decimal::from_str("500005.0").unwrap());
        
        let sell_trade = Trade {
            direction: Direction::Sell,
            ..trade
        };
        
        // net_amount for sell = notional - commission = 500000 - 5 = 499995
        assert_eq!(sell_trade.net_amount(), Decimal::from_str("499995.0").unwrap());
    }
    
    #[test]
    fn test_position_operations() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let mut position = Position {
            id: "pos_1".to_string(),
            symbol,
            direction: Direction::Buy,
            quantity: Decimal::from(10),
            average_price: Decimal::from_str("50000.0").unwrap(),
            market_price: Decimal::from_str("51000.0").unwrap(),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            created_at: 1000000000,
            updated_at: 1000000000,
            strategy_id: None,
        };
        
        // market_value = 10 * 51000 = 510000
        assert_eq!(position.market_value(), Decimal::from_str("510000.0").unwrap());
        
        // cost_basis = 10 * 50000 = 500000
        assert_eq!(position.cost_basis(), Decimal::from_str("500000.0").unwrap());
        
        // 更新市场价格
        position.update_market_price(Decimal::from_str("52000.0").unwrap());
        
        // unrealized_pnl for buy = (52000 - 50000) * 10 = 20000
        assert_eq!(position.unrealized_pnl, Decimal::from_str("20000.0").unwrap());
    }
    
    #[test]
    fn test_order_status_transitions() {
        let symbol = Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto);
        let mut order = Order::market_buy(symbol, Decimal::from(100));
        
        // 初始状态
        assert_eq!(order.status, OrderStatus::Pending);
        assert!(order.is_active());
        assert!(!order.is_filled());
        
        // 部分成交
        order.status = OrderStatus::PartiallyFilled;
        order.filled_quantity = Decimal::from(50);
        assert!(order.is_active());
        assert!(!order.is_filled());
        assert_eq!(order.remaining_quantity(), Decimal::from(50));
        
        // 完全成交
        order.status = OrderStatus::Filled;
        order.filled_quantity = Decimal::from(100);
        assert!(!order.is_active());
        assert!(order.is_filled());
        assert_eq!(order.remaining_quantity(), Decimal::ZERO);
    }
}