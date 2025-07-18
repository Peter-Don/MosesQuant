//! MosesQuant 事件驱动系统
//! 
//! 基于 tokio 的高性能异步事件处理

use crate::types::*;
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

/// 事件处理器接口
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<()>;
    fn name(&self) -> &str;
    fn supported_events(&self) -> Vec<String>;
}

// 类型别名用于简化复杂类型
type EventHandlerMap = Arc<RwLock<HashMap<String, Vec<Arc<dyn EventHandler>>>>>;

/// 事件总线
pub struct EventBus {
    event_sender: mpsc::UnboundedSender<Event>,
    handlers: EventHandlerMap,
    running: Arc<AtomicBool>,
    stats: Arc<RwLock<EventStats>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let bus = Self {
            event_sender,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(EventStats::default())),
        };
        
        // 启动事件处理循环
        bus.start_event_loop(event_receiver);
        
        bus
    }
    
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);
        tracing::info!("EventBus started");
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::Relaxed);
        self.publish(Event::Shutdown).await?;
        tracing::info!("EventBus stopped");
        Ok(())
    }
    
    pub async fn publish(&self, event: Event) -> Result<()> {
        // 更新统计
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        stats.events_by_type.entry(event.event_type().to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        
        // 发送事件
        self.event_sender.send(event)
            .map_err(|e| crate::CzscError::generic(&format!("Failed to send event: {}", e)))?;
        
        Ok(())
    }
    
    pub async fn subscribe(&self, event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        handlers.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
        
        tracing::debug!("Subscribed to event type: {}", event_type);
        Ok(())
    }
    
    fn start_event_loop(&self, mut receiver: mpsc::UnboundedReceiver<Event>) {
        let handlers = Arc::clone(&self.handlers);
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                if let Some(event) = receiver.recv().await {
                    if matches!(event, Event::Shutdown) {
                        tracing::info!("Received shutdown event");
                        break;
                    }
                    
                    // 路由事件到处理器
                    if let Err(e) = Self::route_event(&event, &handlers).await {
                        tracing::error!("Error routing event: {}", e);
                    }
                }
            }
            
            tracing::info!("Event loop stopped");
        });
    }
    
    async fn route_event(
        event: &Event,
        handlers: &EventHandlerMap,
    ) -> Result<()> {
        let event_type = event.event_type();
        let handlers_lock = handlers.read().await;
        
        if let Some(event_handlers) = handlers_lock.get(event_type) {
            for handler in event_handlers {
                if let Err(e) = handler.handle(event).await {
                    tracing::error!("Handler error for event {}: {}", event_type, e);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn get_stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }
}

/// 事件统计
#[derive(Debug, Clone, Default)]
pub struct EventStats {
    pub total_events: u64,
    pub events_by_type: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[derive(Debug)]
    struct TestHandler {
        name: String,
        counter: Arc<std::sync::atomic::AtomicU64>,
    }
    
    impl TestHandler {
        fn new(name: String) -> Self {
            Self {
                name,
                counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            }
        }
        
        #[allow(dead_code)]
        fn get_count(&self) -> u64 {
            self.counter.load(Ordering::Relaxed)
        }
    }
    
    #[async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, _event: &Event) -> Result<()> {
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        
        fn name(&self) -> &str {
            &self.name
        }
        
        fn supported_events(&self) -> Vec<String> {
            vec!["Tick".to_string()]
        }
    }
    
    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        bus.start().await.unwrap();
        
        let handler = Arc::new(TestHandler::new("test_handler".to_string()));
        let counter = Arc::clone(&handler.counter);
        
        // 订阅事件
        bus.subscribe("Tick", handler).await.unwrap();
        
        // 发布事件
        let tick = Tick {
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            timestamp_ns: 1000000000,
            last_price: 50000.0,
            volume: 1.0,
            bid_price: 49999.0,
            ask_price: 50001.0,
            bid_volume: 1.0,
            ask_volume: 1.0,
        };
        
        bus.publish(Event::Tick(tick)).await.unwrap();
        
        // 等待处理
        sleep(Duration::from_millis(100)).await;
        
        // 验证处理器被调用
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        
        bus.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_event_priority_manager() {
        let priority_manager = EventPriorityManager::new();
        
        // 创建不同优先级的事件
        let high_priority_event = Event::OrderUpdate(Order {
            order_id: "order1".to_string(),
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            direction: Direction::Long,
            quantity: 100.0,
            price: Some(50000.0),
            filled_quantity: 0.0,
            order_type: OrderType::Market,
            status: OrderStatus::Submitted,
            created_time: 1000000000,
            updated_time: 1000000000,
        });
        
        let normal_priority_event = Event::Tick(Tick {
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            timestamp_ns: 1000000000,
            last_price: 50000.0,
            volume: 100.0,
            bid_price: 49999.0,
            ask_price: 50001.0,
            bid_volume: 50.0,
            ask_volume: 50.0,
        });
        
        let low_priority_event = Event::Timer(2000000000);
        
        // 添加事件到队列
        priority_manager.enqueue_event(low_priority_event).await.unwrap();
        priority_manager.enqueue_event(high_priority_event).await.unwrap();
        priority_manager.enqueue_event(normal_priority_event).await.unwrap();
        
        // 检查队列统计
        let (high_count, normal_count, low_count) = priority_manager.get_queue_stats().await;
        assert_eq!(high_count, 1);
        assert_eq!(normal_count, 1);
        assert_eq!(low_count, 1);
        
        // 按优先级顺序取出事件
        let first_event = priority_manager.dequeue_event().await.unwrap();
        assert!(matches!(first_event, Event::OrderUpdate(_)));
        
        let second_event = priority_manager.dequeue_event().await.unwrap();
        assert!(matches!(second_event, Event::Tick(_)));
        
        let third_event = priority_manager.dequeue_event().await.unwrap();
        assert!(matches!(third_event, Event::Timer(_)));
        
        // 队列应该为空
        assert!(priority_manager.dequeue_event().await.is_none());
    }
    
    #[tokio::test]
    async fn test_fast_path_processor() {
        let processor = FastPathProcessor::new();
        
        // 注册快速路径处理器
        let handler = Arc::new(TestHandler::new("fast_handler".to_string()));
        let counter = Arc::clone(&handler.counter);
        processor.register_fast_handler(handler).await.unwrap();
        
        // 创建事件
        let event = Event::OrderUpdate(Order {
            order_id: "order1".to_string(),
            symbol: Symbol::new("BTCUSDT", "BINANCE", AssetType::Crypto),
            direction: Direction::Long,
            quantity: 100.0,
            price: Some(50000.0),
            filled_quantity: 0.0,
            order_type: OrderType::Market,
            status: OrderStatus::Submitted,
            created_time: 1000000000,
            updated_time: 1000000000,
        });
        
        // 处理快速路径事件
        processor.process_fast_event(&event).await.unwrap();
        
        // 验证处理器被调用
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        
        // 检查统计信息
        let stats = processor.get_stats().await;
        assert_eq!(stats.total_fast_events, 1);
        assert_eq!(stats.successful_fast_events, 1);
        assert_eq!(stats.failed_fast_events, 0);
        assert!(stats.avg_fast_processing_time_us > 0.0);
    }
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    High,
    Normal,
    Low,
}

/// 事件优先级管理器
#[derive(Debug)]
pub struct EventPriorityManager {
    /// 高优先级事件队列
    high_priority_queue: Arc<RwLock<std::collections::VecDeque<Event>>>,
    /// 普通事件队列
    normal_priority_queue: Arc<RwLock<std::collections::VecDeque<Event>>>,
    /// 低优先级事件队列
    low_priority_queue: Arc<RwLock<std::collections::VecDeque<Event>>>,
    /// 处理器优先级映射
    handler_priorities: Arc<RwLock<HashMap<String, u8>>>,
}

impl Default for EventPriorityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPriorityManager {
    pub fn new() -> Self {
        Self {
            high_priority_queue: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            normal_priority_queue: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            low_priority_queue: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            handler_priorities: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 添加事件到优先级队列
    pub async fn enqueue_event(&self, event: Event) -> Result<()> {
        let priority = self.get_event_priority(&event);
        
        match priority {
            EventPriority::High => {
                let mut queue = self.high_priority_queue.write().await;
                queue.push_back(event);
            }
            EventPriority::Normal => {
                let mut queue = self.normal_priority_queue.write().await;
                queue.push_back(event);
            }
            EventPriority::Low => {
                let mut queue = self.low_priority_queue.write().await;
                queue.push_back(event);
            }
        }
        
        Ok(())
    }
    
    /// 按优先级获取下一个事件
    pub async fn dequeue_event(&self) -> Option<Event> {
        // 先处理高优先级事件
        {
            let mut high_queue = self.high_priority_queue.write().await;
            if let Some(event) = high_queue.pop_front() {
                return Some(event);
            }
        }
        
        // 再处理普通优先级事件
        {
            let mut normal_queue = self.normal_priority_queue.write().await;
            if let Some(event) = normal_queue.pop_front() {
                return Some(event);
            }
        }
        
        // 最后处理低优先级事件
        {
            let mut low_queue = self.low_priority_queue.write().await;
            low_queue.pop_front()
        }
    }
    
    /// 获取事件优先级
    fn get_event_priority(&self, event: &Event) -> EventPriority {
        match event {
            Event::OrderUpdate(_) | Event::Trade(_) => EventPriority::High,
            Event::Tick(_) | Event::Bar(_) => EventPriority::Normal,
            Event::Timer(_) => EventPriority::Low,
            Event::Shutdown => EventPriority::High,
        }
    }
    
    /// 设置处理器优先级
    pub async fn set_handler_priority(&self, handler_name: String, priority: u8) -> Result<()> {
        let mut priorities = self.handler_priorities.write().await;
        priorities.insert(handler_name, priority);
        Ok(())
    }
    
    /// 获取处理器优先级
    pub async fn get_handler_priority(&self, handler_name: &str) -> Option<u8> {
        let priorities = self.handler_priorities.read().await;
        priorities.get(handler_name).copied()
    }
    
    /// 获取队列统计信息
    pub async fn get_queue_stats(&self) -> (usize, usize, usize) {
        let high_queue = self.high_priority_queue.read().await;
        let normal_queue = self.normal_priority_queue.read().await;
        let low_queue = self.low_priority_queue.read().await;
        
        (high_queue.len(), normal_queue.len(), low_queue.len())
    }
}

/// 快速路径处理器统计
#[derive(Debug, Clone, Default)]
pub struct FastPathStats {
    /// 总快速事件数
    pub total_fast_events: u64,
    /// 成功处理的快速事件数
    pub successful_fast_events: u64,
    /// 失败的快速事件数
    pub failed_fast_events: u64,
    /// 平均快速处理时间(微秒)
    pub avg_fast_processing_time_us: f64,
    /// 最大快速处理时间(微秒)
    pub max_fast_processing_time_us: f64,
}

/// 快速路径处理器 - 专门处理高优先级事件
pub struct FastPathProcessor {
    /// 快速路径处理器
    fast_handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    /// 处理统计
    stats: Arc<RwLock<FastPathStats>>,
}

impl Default for FastPathProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl FastPathProcessor {
    pub fn new() -> Self {
        Self {
            fast_handlers: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(FastPathStats::default())),
        }
    }
    
    /// 处理快速路径事件
    pub async fn process_fast_event(&self, event: &Event) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.total_fast_events += 1;
        }
        
        // 并行处理所有快速路径处理器
        let handlers = self.fast_handlers.read().await;
        
        if handlers.is_empty() {
            return Ok(());
        }
        
        // 处理所有快速路径处理器
        let mut success_count = 0;
        let mut error_count = 0;
        
        for handler in handlers.iter() {
            match handler.handle(event).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    tracing::error!("Fast path handler error: {}", e);
                }
            }
        }
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            let processing_time = start_time.elapsed();
            
            stats.successful_fast_events += success_count;
            stats.failed_fast_events += error_count;
            
            let processing_time_us = processing_time.as_micros() as f64;
            stats.avg_fast_processing_time_us = 
                (stats.avg_fast_processing_time_us * (stats.total_fast_events - 1) as f64 + processing_time_us) / 
                stats.total_fast_events as f64;
            
            if processing_time_us > stats.max_fast_processing_time_us {
                stats.max_fast_processing_time_us = processing_time_us;
            }
        }
        
        Ok(())
    }
    
    /// 注册快速路径处理器
    pub async fn register_fast_handler(&self, handler: Arc<dyn EventHandler>) -> Result<()> {
        let mut handlers = self.fast_handlers.write().await;
        handlers.push(handler);
        Ok(())
    }
    
    /// 获取快速路径统计
    pub async fn get_stats(&self) -> FastPathStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}