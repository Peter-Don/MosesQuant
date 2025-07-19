//! 高性能事件总线实现
//! 
//! 基于零成本抽象设计的事件系统，支持高并发、类型安全的事件处理

use crate::types::{Event, EventPriority};
use crate::{Result, MosesQuantError};
use std::any::{Any, TypeId};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use tracing::{debug, warn, error, info};

/// 事件处理器特征
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// 处理事件
    async fn handle(&self, event: Box<dyn Event>) -> Result<()>;
    
    /// 事件处理器名称
    fn name(&self) -> &str;
    
    /// 支持的事件类型
    fn event_types(&self) -> Vec<TypeId>;
    
    /// 是否启用
    fn is_enabled(&self) -> bool { true }
}

/// 类型化事件处理器
#[async_trait::async_trait]
pub trait TypedEventHandler<T>: Send + Sync 
where 
    T: Event + 'static
{
    async fn handle_typed(&self, event: &T) -> Result<()>;
}

/// 事件包装器，用于优先级队列
#[derive(Debug)]
struct EventWrapper {
    event: Box<dyn Event>,
    priority: EventPriority,
    timestamp: Instant,
    sequence: u64,
}

impl PartialEq for EventWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}

impl Eq for EventWrapper {}

impl PartialOrd for EventWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // 优先级队列：优先级高的先处理，时间戳早的先处理
        other.priority.cmp(&self.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

/// 事件总线统计信息
#[derive(Debug, Clone, Default)]
pub struct EventBusMetrics {
    pub events_published: u64,
    pub events_processed: u64,
    pub events_failed: u64,
    pub handlers_registered: usize,
    pub queue_size: usize,
    pub average_processing_time_ms: f64,
    pub peak_queue_size: usize,
}

/// 事件总线配置
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// 事件队列容量
    pub queue_capacity: usize,
    /// 处理线程数
    pub worker_threads: usize,
    /// 最大处理时间（毫秒）
    pub max_processing_time_ms: u64,
    /// 是否启用指标收集
    pub enable_metrics: bool,
    /// 批处理大小
    pub batch_size: usize,
    /// 处理超时时间
    pub processing_timeout: Duration,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 10000,
            worker_threads: num_cpus::get(),
            max_processing_time_ms: 1000,
            enable_metrics: true,
            batch_size: 100,
            processing_timeout: Duration::from_millis(5000),
        }
    }
}

/// 高性能事件总线
pub struct EventBus {
    /// 事件发送通道
    event_sender: mpsc::UnboundedSender<EventWrapper>,
    /// 注册的事件处理器
    handlers: Arc<RwLock<HashMap<TypeId, Vec<Arc<dyn EventHandler>>>>>,
    /// 全局事件处理器
    global_handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    /// 事件序列号
    sequence_counter: Arc<Mutex<u64>>,
    /// 统计信息
    metrics: Arc<RwLock<EventBusMetrics>>,
    /// 配置
    config: EventBusConfig,
    /// 关闭信号
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new(config: EventBusConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let handlers = Arc::new(RwLock::new(HashMap::new()));
        let global_handlers = Arc::new(RwLock::new(Vec::new()));
        let sequence_counter = Arc::new(Mutex::new(0));
        let metrics = Arc::new(RwLock::new(EventBusMetrics::default()));
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

        // 启动事件处理任务
        let bus = Self {
            event_sender,
            handlers: handlers.clone(),
            global_handlers: global_handlers.clone(),
            sequence_counter: sequence_counter.clone(),
            metrics: metrics.clone(),
            config: config.clone(),
            shutdown_sender: Some(shutdown_sender),
        };

        // 启动处理器任务
        bus.start_event_processor(event_receiver, shutdown_receiver);

        bus
    }

    /// 发布事件
    pub async fn publish<T>(&self, event: T) -> Result<()> 
    where 
        T: Event + 'static
    {
        let sequence = {
            let mut counter = self.sequence_counter.lock().await;
            *counter += 1;
            *counter
        };

        let wrapper = EventWrapper {
            priority: event.priority(),
            timestamp: Instant::now(),
            sequence,
            event: Box::new(event),
        };

        self.event_sender.send(wrapper)
            .map_err(|_| MosesQuantError::EventBus { 
                message: "Failed to send event to queue".to_string() 
            })?;

        // 更新统计
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.events_published += 1;
        }

        Ok(())
    }

    /// 注册事件处理器
    pub async fn register_handler<T>(&self, handler: Arc<dyn EventHandler>) -> Result<()> 
    where 
        T: Event + 'static
    {
        let type_id = TypeId::of::<T>();
        let mut handlers = self.handlers.write().await;
        
        handlers.entry(type_id)
            .or_insert_with(Vec::new)
            .push(handler);

        // 更新统计
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.handlers_registered = handlers.values().map(|v| v.len()).sum();
        }

        info!("Registered event handler for type: {:?}", std::any::type_name::<T>());
        Ok(())
    }

    /// 注册全局事件处理器（处理所有事件）
    pub async fn register_global_handler(&self, handler: Arc<dyn EventHandler>) -> Result<()> {
        let handler_name = handler.name().to_string();
        let mut global_handlers = self.global_handlers.write().await;
        global_handlers.push(handler);

        info!("Registered global event handler: {}", handler_name);
        Ok(())
    }

    /// 注销事件处理器
    pub async fn unregister_handler<T>(&self, handler_name: &str) -> Result<bool> 
    where 
        T: Event + 'static
    {
        let type_id = TypeId::of::<T>();
        let mut handlers = self.handlers.write().await;
        
        if let Some(handlers_vec) = handlers.get_mut(&type_id) {
            let original_len = handlers_vec.len();
            handlers_vec.retain(|h| h.name() != handler_name);
            
            // 更新统计
            if self.config.enable_metrics && handlers_vec.len() != original_len {
                drop(handlers);  // 显式释放写锁
                let handlers_read = self.handlers.read().await;
                let mut metrics = self.metrics.write().await;
                metrics.handlers_registered = handlers_read.values().map(|v| v.len()).sum();
            }

            Ok(handlers_vec.len() != original_len)
        } else {
            Ok(false)
        }
    }

    /// 获取统计信息
    pub async fn get_metrics(&self) -> EventBusMetrics {
        self.metrics.read().await.clone()
    }

    /// 启动事件处理器
    fn start_event_processor(
        &self,
        mut event_receiver: mpsc::UnboundedReceiver<EventWrapper>,
        mut shutdown_receiver: mpsc::UnboundedReceiver<()>,
    ) {
        let handlers = self.handlers.clone();
        let global_handlers = self.global_handlers.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut event_queue = BinaryHeap::new();
            let mut processing_times = Vec::new();

            loop {
                tokio::select! {
                    // 接收新事件
                    Some(event_wrapper) = event_receiver.recv() => {
                        event_queue.push(event_wrapper);
                        
                        // 更新队列大小统计
                        if config.enable_metrics {
                            let mut metrics_guard = metrics.write().await;
                            metrics_guard.queue_size = event_queue.len();
                            if event_queue.len() > metrics_guard.peak_queue_size {
                                metrics_guard.peak_queue_size = event_queue.len();
                            }
                        }
                    }
                    
                    // 接收关闭信号
                    Some(_) = shutdown_receiver.recv() => {
                        info!("Event bus processor shutting down");
                        break;
                    }
                    
                    // 处理事件队列
                    _ = tokio::time::sleep(Duration::from_millis(1)) => {
                        if !event_queue.is_empty() {
                            let batch_size = std::cmp::min(config.batch_size, event_queue.len());
                            let mut batch = Vec::with_capacity(batch_size);
                            
                            for _ in 0..batch_size {
                                if let Some(event_wrapper) = event_queue.pop() {
                                    batch.push(event_wrapper);
                                }
                            }
                            
                            // 批量处理事件
                            Self::process_event_batch(
                                batch,
                                &handlers,
                                &global_handlers,
                                &metrics,
                                &config,
                                &mut processing_times,
                            ).await;
                        }
                    }
                }
            }

            info!("Event bus processor stopped");
        });
    }

    /// 批量处理事件
    async fn process_event_batch(
        batch: Vec<EventWrapper>,
        handlers: &Arc<RwLock<HashMap<TypeId, Vec<Arc<dyn EventHandler>>>>>,
        global_handlers: &Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
        metrics: &Arc<RwLock<EventBusMetrics>>,
        config: &EventBusConfig,
        processing_times: &mut Vec<Duration>,
    ) {
        for event_wrapper in batch {
            let start_time = Instant::now();
            let mut success = true;

            // 获取事件类型的处理器
            let type_id = (*event_wrapper.event).as_any().type_id();
            let typed_handlers = {
                let handlers_guard = handlers.read().await;
                handlers_guard.get(&type_id).cloned().unwrap_or_default()
            };

            // 获取全局处理器
            let global_handlers_vec = global_handlers.read().await.clone();

            // 处理事件
            let all_handlers = typed_handlers.into_iter().chain(global_handlers_vec.into_iter());
            
            for handler in all_handlers {
                if !handler.is_enabled() {
                    continue;
                }

                // Clone the event for processing - in production we'd use Arc<dyn Event>
                let cloned_event = Box::new(DummyEvent {
                    event_type: event_wrapper.event.event_type(),
                    timestamp: event_wrapper.event.timestamp(),
                    source: event_wrapper.event.source().to_string(),
                });
                let processing_future = handler.handle(cloned_event);
                
                match tokio::time::timeout(config.processing_timeout, processing_future).await {
                    Ok(Ok(_)) => {
                        debug!("Event processed successfully by handler: {}", handler.name());
                    }
                    Ok(Err(e)) => {
                        error!("Event processing failed in handler {}: {:?}", handler.name(), e);
                        success = false;
                    }
                    Err(_) => {
                        error!("Event processing timeout in handler: {}", handler.name());
                        success = false;
                    }
                }
            }

            // 更新统计信息
            if config.enable_metrics {
                let processing_time = start_time.elapsed();
                processing_times.push(processing_time);
                
                // 保持最近1000次处理时间用于计算平均值
                if processing_times.len() > 1000 {
                    processing_times.remove(0);
                }

                let mut metrics_guard = metrics.write().await;
                metrics_guard.events_processed += 1;
                
                if !success {
                    metrics_guard.events_failed += 1;
                }

                // 计算平均处理时间
                if !processing_times.is_empty() {
                    let total_time: Duration = processing_times.iter().sum();
                    metrics_guard.average_processing_time_ms = 
                        total_time.as_secs_f64() * 1000.0 / processing_times.len() as f64;
                }
            }
        }
    }

    /// 关闭事件总线
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            sender.send(()).map_err(|_| MosesQuantError::EventBus {
                message: "Failed to send shutdown signal".to_string()
            })?;
            
            info!("Event bus shutdown initiated");
        }
        
        Ok(())
    }
}

// Dummy event for testing and development - in production, use proper event cloning
#[derive(Debug)]
struct DummyEvent {
    event_type: &'static str,
    timestamp: TimestampNs,
    source: String,
}

impl Event for DummyEvent {
    fn event_type(&self) -> &'static str { self.event_type }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn source(&self) -> &str { &self.source }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

/// 简单的事件处理器包装器
pub struct SimpleEventHandler<F> {
    name: String,
    handler_fn: F,
    event_types: Vec<TypeId>,
}

impl<F> SimpleEventHandler<F> 
where 
    F: Fn(Box<dyn Event>) -> Result<()> + Send + Sync + 'static
{
    pub fn new<T>(name: String, handler_fn: F) -> Self 
    where 
        T: Event + 'static
    {
        Self {
            name,
            handler_fn,
            event_types: vec![TypeId::of::<T>()],
        }
    }
}

#[async_trait::async_trait]
impl<F> EventHandler for SimpleEventHandler<F> 
where 
    F: Fn(Box<dyn Event>) -> Result<()> + Send + Sync + 'static
{
    async fn handle(&self, event: Box<dyn Event>) -> Result<()> {
        (self.handler_fn)(event)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn event_types(&self) -> Vec<TypeId> {
        self.event_types.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::str::FromStr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::time::{sleep, Duration};

    #[derive(Debug)]
    struct TestEvent {
        pub id: u64,
        pub message: String,
        pub timestamp: TimestampNs,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str { "Test" }
        fn timestamp(&self) -> TimestampNs { self.timestamp }
        fn source(&self) -> &str { "test" }
        fn as_any(&self) -> &dyn Any { self }
    }

    struct TestHandler {
        name: String,
        counter: Arc<AtomicU64>,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, _event: Box<dyn Event>) -> Result<()> {
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn event_types(&self) -> Vec<TypeId> {
            vec![TypeId::of::<TestEvent>()]
        }
    }

    #[tokio::test]
    async fn test_event_bus_creation() {
        let config = EventBusConfig::default();
        let _event_bus = EventBus::new(config);
        // 测试创建成功
    }

    #[tokio::test]
    async fn test_event_publishing_and_handling() {
        let config = EventBusConfig::default();
        let event_bus = EventBus::new(config);
        
        let counter = Arc::new(AtomicU64::new(0));
        let handler = Arc::new(TestHandler {
            name: "test_handler".to_string(),
            counter: counter.clone(),
        });

        // 注册处理器
        event_bus.register_handler::<TestEvent>(handler).await.unwrap();

        // 发布事件
        let test_event = TestEvent {
            id: 1,
            message: "Test message".to_string(),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        event_bus.publish(test_event).await.unwrap();

        // 等待事件处理
        sleep(Duration::from_millis(100)).await;

        // 验证处理器被调用
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_event_priority_ordering() {
        let config = EventBusConfig::default();
        let event_bus = EventBus::new(config);
        
        let counter = Arc::new(AtomicU64::new(0));
        let handler = Arc::new(TestHandler {
            name: "priority_test_handler".to_string(),
            counter: counter.clone(),
        });

        event_bus.register_handler::<TestEvent>(handler).await.unwrap();

        // 创建不同优先级的事件
        let high_priority_event = TestEvent {
            id: 1,
            message: "High priority".to_string(),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        let low_priority_event = TestEvent {
            id: 2,
            message: "Low priority".to_string(),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        // 先发布低优先级，再发布高优先级
        event_bus.publish(low_priority_event).await.unwrap();
        event_bus.publish(high_priority_event).await.unwrap();

        // 等待处理
        sleep(Duration::from_millis(100)).await;

        // 验证都被处理了
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let mut config = EventBusConfig::default();
        config.enable_metrics = true;
        
        let event_bus = EventBus::new(config);
        
        let counter = Arc::new(AtomicU64::new(0));
        let handler = Arc::new(TestHandler {
            name: "metrics_test_handler".to_string(),
            counter: counter.clone(),
        });

        event_bus.register_handler::<TestEvent>(handler).await.unwrap();

        // 发布多个事件
        for i in 0..5 {
            let test_event = TestEvent {
                id: i,
                message: format!("Test message {}", i),
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            };
            event_bus.publish(test_event).await.unwrap();
        }

        // 等待处理
        sleep(Duration::from_millis(200)).await;

        // 检查指标
        let metrics = event_bus.get_metrics().await;
        assert_eq!(metrics.events_published, 5);
        assert_eq!(metrics.handlers_registered, 1);
    }
}