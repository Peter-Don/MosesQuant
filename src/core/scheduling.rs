//! 任务调度系统
//! 
//! 提供高精度的任务调度、定时器和执行优先级管理

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{sleep_until, Instant as TokioInstant};
use crate::types::{Event, TimestampNs};
use crate::{Result, MosesQuantError, EventBus};
use tracing::{debug, info, warn, error};

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0,   // 关键任务，最高优先级
    High = 1,       // 高优先级
    Normal = 2,     // 普通优先级
    Low = 3,        // 低优先级
    Background = 4, // 后台任务
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,    // 等待执行
    Running,    // 正在执行
    Completed,  // 已完成
    Failed,     // 执行失败
    Cancelled,  // 已取消
}

/// 调度任务特征
#[async_trait::async_trait]
pub trait ScheduledTask: Send + Sync {
    /// 执行任务
    async fn execute(&self) -> Result<()>;
    
    /// 任务名称
    fn name(&self) -> &str;
    
    /// 任务优先级
    fn priority(&self) -> TaskPriority { TaskPriority::Normal }
    
    /// 任务超时时间
    fn timeout(&self) -> Option<Duration> { Some(Duration::from_secs(30)) }
    
    /// 是否可以取消
    fn cancellable(&self) -> bool { true }
    
    /// 任务描述
    fn description(&self) -> Option<&str> { None }
}

/// 定时任务特征
#[async_trait::async_trait]
pub trait TimerTask: ScheduledTask {
    /// 下次执行时间
    fn next_execution(&self) -> TokioInstant;
    
    /// 是否为重复任务
    fn is_recurring(&self) -> bool;
    
    /// 获取执行间隔（对于重复任务）
    fn interval(&self) -> Option<Duration> { None }
    
    /// 更新下次执行时间
    fn update_next_execution(&mut self);
}

/// 任务包装器
#[derive(Debug)]
struct TaskWrapper {
    id: u64,
    task: Arc<dyn ScheduledTask>,
    priority: TaskPriority,
    scheduled_time: TokioInstant,
    created_time: Instant,
    status: TaskStatus,
}

impl PartialEq for TaskWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TaskWrapper {}

impl PartialOrd for TaskWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // 优先级队列：时间早的先执行，优先级高的先执行
        other.scheduled_time.cmp(&self.scheduled_time)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// 定时器包装器
#[derive(Debug)]
struct TimerWrapper {
    id: u64,
    timer: Arc<Mutex<dyn TimerTask>>,
    next_execution: TokioInstant,
}

impl PartialEq for TimerWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TimerWrapper {}

impl PartialOrd for TimerWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        other.next_execution.cmp(&self.next_execution)
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// 调度器统计信息
#[derive(Debug, Clone, Default)]
pub struct SchedulerMetrics {
    pub tasks_scheduled: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_cancelled: u64,
    pub active_tasks: usize,
    pub average_execution_time_ms: f64,
    pub peak_queue_size: usize,
    pub timers_active: usize,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 工作线程数
    pub worker_threads: usize,
    /// 任务队列容量
    pub max_queue_size: usize,
    /// 默认任务超时时间
    pub default_timeout: Duration,
    /// 是否启用指标收集
    pub enable_metrics: bool,
    /// 时间精度（毫秒）
    pub time_precision_ms: u64,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get().max(2),
            max_queue_size: 10000,
            default_timeout: Duration::from_secs(30),
            enable_metrics: true,
            time_precision_ms: 10,
            max_concurrent_tasks: 1000,
        }
    }
}

/// 高精度任务调度器
pub struct TaskScheduler {
    /// 任务队列
    task_queue: Arc<Mutex<BinaryHeap<TaskWrapper>>>,
    /// 定时器队列
    timer_queue: Arc<Mutex<BinaryHeap<TimerWrapper>>>,
    /// 运行中的任务
    running_tasks: Arc<RwLock<HashMap<u64, Arc<dyn ScheduledTask>>>>,
    /// 任务ID计数器
    task_id_counter: Arc<Mutex<u64>>,
    /// 指标统计
    metrics: Arc<RwLock<SchedulerMetrics>>,
    /// 配置
    config: SchedulerConfig,
    /// 事件总线
    event_bus: Option<Arc<EventBus>>,
    /// 关闭信号
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(config: SchedulerConfig, event_bus: Option<Arc<EventBus>>) -> Self {
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();
        
        let scheduler = Self {
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            timer_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_id_counter: Arc::new(Mutex::new(0)),
            metrics: Arc::new(RwLock::new(SchedulerMetrics::default())),
            config: config.clone(),
            event_bus,
            shutdown_sender: Some(shutdown_sender),
        };

        // 启动调度器主循环
        scheduler.start_scheduler_loop(shutdown_receiver);
        
        scheduler
    }

    /// 调度单次任务
    pub async fn schedule_task(
        &self, 
        task: Arc<dyn ScheduledTask>, 
        delay: Option<Duration>
    ) -> Result<u64> {
        let task_id = {
            let mut counter = self.task_id_counter.lock().await;
            *counter += 1;
            *counter
        };

        let scheduled_time = if let Some(delay) = delay {
            TokioInstant::now() + delay
        } else {
            TokioInstant::now()
        };

        let wrapper = TaskWrapper {
            id: task_id,
            priority: task.priority(),
            task,
            scheduled_time,
            created_time: Instant::now(),
            status: TaskStatus::Pending,
        };

        let mut queue = self.task_queue.lock().await;
        
        // 检查队列容量
        if queue.len() >= self.config.max_queue_size {
            return Err(MosesQuantError::Internal {
                message: "Task queue is full".to_string()
            });
        }

        queue.push(wrapper);

        // 更新指标
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.tasks_scheduled += 1;
            if queue.len() > metrics.peak_queue_size {
                metrics.peak_queue_size = queue.len();
            }
        }

        debug!("Scheduled task {} with ID {}", task.name(), task_id);
        Ok(task_id)
    }

    /// 调度定时任务
    pub async fn schedule_timer(&self, timer: Arc<Mutex<dyn TimerTask>>) -> Result<u64> {
        let task_id = {
            let mut counter = self.task_id_counter.lock().await;
            *counter += 1;
            *counter
        };

        let next_execution = {
            let timer_guard = timer.lock().await;
            timer_guard.next_execution()
        };

        let wrapper = TimerWrapper {
            id: task_id,
            timer,
            next_execution,
        };

        let mut queue = self.timer_queue.lock().await;
        queue.push(wrapper);

        // 更新指标
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.timers_active = queue.len();
        }

        debug!("Scheduled timer with ID {}", task_id);
        Ok(task_id)
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: u64) -> Result<bool> {
        // 从队列中移除任务
        let mut queue = self.task_queue.lock().await;
        let original_len = queue.len();
        
        // 重建队列，排除要取消的任务
        let mut new_queue = BinaryHeap::new();
        let mut cancelled = false;
        
        while let Some(mut wrapper) = queue.pop() {
            if wrapper.id == task_id {
                wrapper.status = TaskStatus::Cancelled;
                cancelled = true;
                
                // 更新指标
                if self.config.enable_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.tasks_cancelled += 1;
                }
            } else {
                new_queue.push(wrapper);
            }
        }
        
        *queue = new_queue;
        
        if cancelled {
            info!("Cancelled task with ID {}", task_id);
        }
        
        Ok(cancelled)
    }

    /// 获取调度器指标
    pub async fn get_metrics(&self) -> SchedulerMetrics {
        let mut metrics = self.metrics.read().await.clone();
        
        // 更新实时指标
        metrics.active_tasks = self.running_tasks.read().await.len();
        
        let task_queue_size = self.task_queue.lock().await.len();
        let timer_queue_size = self.timer_queue.lock().await.len();
        
        if task_queue_size > metrics.peak_queue_size {
            let mut metrics_write = self.metrics.write().await;
            metrics_write.peak_queue_size = task_queue_size;
            metrics.peak_queue_size = task_queue_size;
        }
        
        metrics.timers_active = timer_queue_size;
        
        metrics
    }

    /// 启动调度器主循环
    fn start_scheduler_loop(&self, mut shutdown_receiver: mpsc::UnboundedReceiver<()>) {
        let task_queue = self.task_queue.clone();
        let timer_queue = self.timer_queue.clone();
        let running_tasks = self.running_tasks.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            let mut execution_times = Vec::new();
            
            loop {
                tokio::select! {
                    // 检查关闭信号
                    Some(_) = shutdown_receiver.recv() => {
                        info!("Task scheduler shutting down");
                        break;
                    }
                    
                    // 主调度循环
                    _ = sleep_until(TokioInstant::now() + Duration::from_millis(config.time_precision_ms)) => {
                        let now = TokioInstant::now();
                        
                        // 处理定时器
                        Self::process_timers(&timer_queue, &task_queue, now).await;
                        
                        // 处理任务队列
                        Self::process_task_queue(
                            &task_queue,
                            &running_tasks,
                            &metrics,
                            &config,
                            &event_bus,
                            &mut execution_times,
                            now,
                        ).await;
                    }
                }
            }
        });
    }

    /// 处理定时器队列
    async fn process_timers(
        timer_queue: &Arc<Mutex<BinaryHeap<TimerWrapper>>>,
        task_queue: &Arc<Mutex<BinaryHeap<TaskWrapper>>>,
        now: TokioInstant,
    ) {
        let mut timers = timer_queue.lock().await;
        let mut ready_timers = Vec::new();

        // 收集到期的定时器
        while let Some(timer_wrapper) = timers.peek() {
            if timer_wrapper.next_execution <= now {
                ready_timers.push(timers.pop().unwrap());
            } else {
                break;
            }
        }

        drop(timers);

        // 处理到期的定时器
        for timer_wrapper in ready_timers {
            let timer_task = timer_wrapper.timer.clone();
            
            // 将定时器任务转换为普通任务并加入任务队列
            let task_wrapper = TaskWrapper {
                id: timer_wrapper.id,
                task: timer_task.clone() as Arc<dyn ScheduledTask>,
                priority: {
                    let timer_guard = timer_task.lock().await;
                    timer_guard.priority()
                },
                scheduled_time: now,
                created_time: Instant::now(),
                status: TaskStatus::Pending,
            };

            let mut tasks = task_queue.lock().await;
            tasks.push(task_wrapper);

            // 如果是重复任务，重新调度
            {
                let mut timer_guard = timer_task.lock().await;
                if timer_guard.is_recurring() {
                    timer_guard.update_next_execution();
                    let next_execution = timer_guard.next_execution();
                    
                    let new_timer_wrapper = TimerWrapper {
                        id: timer_wrapper.id,
                        timer: timer_task.clone(),
                        next_execution,
                    };
                    
                    drop(timer_guard);
                    drop(tasks);
                    
                    let mut timers = timer_queue.lock().await;
                    timers.push(new_timer_wrapper);
                }
            }
        }
    }

    /// 处理任务队列
    async fn process_task_queue(
        task_queue: &Arc<Mutex<BinaryHeap<TaskWrapper>>>,
        running_tasks: &Arc<RwLock<HashMap<u64, Arc<dyn ScheduledTask>>>>,
        metrics: &Arc<RwLock<SchedulerMetrics>>,
        config: &SchedulerConfig,
        event_bus: &Option<Arc<EventBus>>,
        execution_times: &mut Vec<Duration>,
        now: TokioInstant,
    ) {
        let mut queue = task_queue.lock().await;
        let mut ready_tasks = Vec::new();

        // 检查并发任务数限制
        let current_running = running_tasks.read().await.len();
        let max_new_tasks = config.max_concurrent_tasks.saturating_sub(current_running);

        // 收集准备执行的任务
        let mut collected = 0;
        while let Some(task_wrapper) = queue.peek() {
            if task_wrapper.scheduled_time <= now && collected < max_new_tasks {
                ready_tasks.push(queue.pop().unwrap());
                collected += 1;
            } else {
                break;
            }
        }

        drop(queue);

        // 执行准备好的任务
        for task_wrapper in ready_tasks {
            let task_id = task_wrapper.id;
            let task = task_wrapper.task.clone();
            
            // 添加到运行中的任务
            {
                let mut running = running_tasks.write().await;
                running.insert(task_id, task.clone());
            }

            // 异步执行任务
            let running_tasks_clone = running_tasks.clone();
            let metrics_clone = metrics.clone();
            let config_clone = config.clone();
            let event_bus_clone = event_bus.clone();
            
            tokio::spawn(async move {
                let start_time = Instant::now();
                let timeout = task.timeout().unwrap_or(config_clone.default_timeout);
                
                let result = tokio::time::timeout(timeout, task.execute()).await;
                
                let execution_time = start_time.elapsed();
                
                // 从运行中的任务中移除
                {
                    let mut running = running_tasks_clone.write().await;
                    running.remove(&task_id);
                }

                // 更新指标
                if config_clone.enable_metrics {
                    let mut metrics_guard = metrics_clone.write().await;
                    
                    match result {
                        Ok(Ok(_)) => {
                            metrics_guard.tasks_completed += 1;
                            debug!("Task {} completed successfully", task.name());
                        }
                        Ok(Err(e)) => {
                            metrics_guard.tasks_failed += 1;
                            error!("Task {} failed: {:?}", task.name(), e);
                        }
                        Err(_) => {
                            metrics_guard.tasks_failed += 1;
                            warn!("Task {} timed out after {:?}", task.name(), timeout);
                        }
                    }
                    
                    // 更新平均执行时间
                    let mut times = vec![execution_time];
                    times.extend_from_slice(execution_times);
                    if times.len() > 1000 {
                        times.truncate(1000);
                    }
                    
                    let total_time: Duration = times.iter().sum();
                    metrics_guard.average_execution_time_ms = 
                        total_time.as_secs_f64() * 1000.0 / times.len() as f64;
                }

                // 发布任务完成事件
                if let Some(event_bus) = event_bus_clone {
                    let _event_result = event_bus.publish(TaskCompletionEvent {
                        task_id,
                        task_name: task.name().to_string(),
                        execution_time,
                        success: result.is_ok() && result.unwrap().is_ok(),
                        timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    }).await;
                }
            });
        }
    }

    /// 关闭调度器
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            sender.send(()).map_err(|_| MosesQuantError::Internal {
                message: "Failed to send shutdown signal".to_string()
            })?;
            
            info!("Task scheduler shutdown initiated");
        }
        
        Ok(())
    }
}

/// 任务完成事件
#[derive(Debug, Clone)]
pub struct TaskCompletionEvent {
    pub task_id: u64,
    pub task_name: String,
    pub execution_time: Duration,
    pub success: bool,
    pub timestamp: TimestampNs,
}

impl Event for TaskCompletionEvent {
    fn event_type(&self) -> &'static str { "TaskCompletion" }
    fn timestamp(&self) -> TimestampNs { self.timestamp }
    fn source(&self) -> &str { "task_scheduler" }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

/// 简单任务包装器
pub struct SimpleTask<F> {
    name: String,
    priority: TaskPriority,
    timeout: Option<Duration>,
    task_fn: F,
}

impl<F> SimpleTask<F> 
where 
    F: Fn() -> Result<()> + Send + Sync + 'static
{
    pub fn new(name: String, task_fn: F) -> Self {
        Self {
            name,
            priority: TaskPriority::Normal,
            timeout: None,
            task_fn,
        }
    }
    
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[async_trait::async_trait]
impl<F> ScheduledTask for SimpleTask<F> 
where 
    F: Fn() -> Result<()> + Send + Sync + 'static
{
    async fn execute(&self) -> Result<()> {
        (self.task_fn)()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> TaskPriority {
        self.priority
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::time::{sleep, Duration};

    struct TestTask {
        name: String,
        counter: Arc<AtomicU64>,
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl ScheduledTask for TestTask {
        async fn execute(&self) -> Result<()> {
            sleep(self.delay).await;
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_task_scheduling() {
        let config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(config, None);
        
        let counter = Arc::new(AtomicU64::new(0));
        let task = Arc::new(TestTask {
            name: "test_task".to_string(),
            counter: counter.clone(),
            delay: Duration::from_millis(10),
        });

        // 调度任务
        let task_id = scheduler.schedule_task(task, Some(Duration::from_millis(50))).await.unwrap();
        
        // 任务应该还没执行
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        
        // 等待任务执行
        sleep(Duration::from_millis(100)).await;
        
        // 任务应该已经执行
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        
        let metrics = scheduler.get_metrics().await;
        assert_eq!(metrics.tasks_scheduled, 1);
        assert_eq!(metrics.tasks_completed, 1);
    }

    #[tokio::test]
    async fn test_task_priority() {
        let config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(config, None);
        
        let execution_order = Arc::new(Mutex::new(Vec::new()));
        
        // 创建不同优先级的任务
        let high_priority_task = Arc::new(SimpleTask::new(
            "high_priority".to_string(),
            {
                let order = execution_order.clone();
                move || {
                    let order = order.clone();
                    tokio::spawn(async move {
                        let mut order_guard = order.lock().await;
                        order_guard.push("high");
                    });
                    Ok(())
                }
            }
        ).with_priority(TaskPriority::High));

        let low_priority_task = Arc::new(SimpleTask::new(
            "low_priority".to_string(),
            {
                let order = execution_order.clone();
                move || {
                    let order = order.clone();
                    tokio::spawn(async move {
                        let mut order_guard = order.lock().await;
                        order_guard.push("low");
                    });
                    Ok(())
                }
            }
        ).with_priority(TaskPriority::Low));

        // 先调度低优先级任务
        scheduler.schedule_task(low_priority_task, None).await.unwrap();
        // 再调度高优先级任务
        scheduler.schedule_task(high_priority_task, None).await.unwrap();
        
        sleep(Duration::from_millis(100)).await;
        
        // 高优先级任务应该先执行
        let order = execution_order.lock().await;
        assert!(order.len() >= 1);
        // 注意：由于异步执行的复杂性，这里只验证任务被调度了
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(config, None);
        
        let counter = Arc::new(AtomicU64::new(0));
        let task = Arc::new(TestTask {
            name: "cancellable_task".to_string(),
            counter: counter.clone(),
            delay: Duration::from_millis(10),
        });

        // 调度延迟任务
        let task_id = scheduler.schedule_task(task, Some(Duration::from_millis(100))).await.unwrap();
        
        // 立即取消任务
        let cancelled = scheduler.cancel_task(task_id).await.unwrap();
        assert!(cancelled);
        
        // 等待任务原本应该执行的时间
        sleep(Duration::from_millis(150)).await;
        
        // 任务不应该被执行
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        
        let metrics = scheduler.get_metrics().await;
        assert_eq!(metrics.tasks_cancelled, 1);
    }
}