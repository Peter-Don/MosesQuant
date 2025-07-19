//! 简化的事件总线实现
//! 
//! 基本的事件发布和订阅功能，用于核心系统建立

use crate::types::{Event, EventPriority};
use crate::{Result, MosesQuantError};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use tracing::{debug, info};

/// 简化的事件处理器
pub trait SimpleEventHandler: Send + Sync {
    /// 处理事件
    async fn handle(&self, event_type: &str, data: &str) -> Result<()>;
    
    /// 事件处理器名称
    fn name(&self) -> &str;
}

/// 简化的事件总线
pub struct SimpleEventBus {
    /// 事件发送通道
    event_sender: mpsc::UnboundedSender<SimpleEvent>,
    /// 注册的事件处理器
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn SimpleEventHandler>>>>>,
}

#[derive(Debug)]
struct SimpleEvent {
    event_type: String,
    data: String,
}

impl SimpleEventBus {
    /// 创建新的简化事件总线
    pub fn new() -> Self {
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let handlers = Arc::new(RwLock::new(HashMap::new()));

        // 启动事件处理任务
        let handlers_clone = handlers.clone();
        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                let handlers_guard = handlers_clone.read().await;
                if let Some(event_handlers) = handlers_guard.get(&event.event_type) {
                    for handler in event_handlers {
                        if let Err(e) = handler.handle(&event.event_type, &event.data).await {
                            debug!("Event handler {} failed: {:?}", handler.name(), e);
                        }
                    }
                }
            }
        });

        Self {
            event_sender,
            handlers,
        }
    }

    /// 发布事件
    pub async fn publish(&self, event_type: &str, data: &str) -> Result<()> {
        let event = SimpleEvent {
            event_type: event_type.to_string(),
            data: data.to_string(),
        };

        self.event_sender.send(event)
            .map_err(|_| MosesQuantError::EventBus { 
                message: "Failed to send event".to_string() 
            })?;

        Ok(())
    }

    /// 注册事件处理器
    pub async fn register_handler(&self, event_type: &str, handler: Arc<dyn SimpleEventHandler>) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        handlers.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler.clone());

        info!("Registered event handler {} for event type: {}", handler.name(), event_type);
        Ok(())
    }
}

impl Default for SimpleEventBus {
    fn default() -> Self {
        Self::new()
    }
}