//! 简化的内存管理系统
//! 
//! 提供基本的对象池和内存优化功能

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use crate::{Result, MosesQuantError};

/// 简单对象池
pub struct SimpleObjectPool<T> {
    objects: Arc<Mutex<VecDeque<T>>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
}

impl<T> SimpleObjectPool<T> 
where 
    T: Send + 'static
{
    /// 创建新的对象池
    pub fn new<F>(factory: F, max_size: usize) -> Self 
    where 
        F: Fn() -> T + Send + Sync + 'static
    {
        Self {
            objects: Arc::new(Mutex::new(VecDeque::new())),
            factory: Arc::new(factory),
            max_size,
        }
    }
    
    /// 获取对象
    pub async fn acquire(&self) -> Result<T> {
        let mut objects = self.objects.lock().await;
        
        if let Some(obj) = objects.pop_front() {
            Ok(obj)
        } else {
            Ok((self.factory)())
        }
    }
    
    /// 归还对象
    pub async fn release(&self, obj: T) {
        let mut objects = self.objects.lock().await;
        if objects.len() < self.max_size {
            objects.push_back(obj);
        }
        // 如果池已满，对象会被丢弃
    }
    
    /// 获取可用对象数量
    pub async fn available(&self) -> usize {
        self.objects.lock().await.len()
    }
    
    /// 清空池
    pub async fn clear(&self) {
        let mut objects = self.objects.lock().await;
        objects.clear();
    }
}

/// 内存使用统计
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub pool_objects_created: usize,
    pub pool_objects_reused: usize,
    pub total_allocations: usize,
}

/// 简化的内存管理器
pub struct SimpleMemoryManager {
    stats: Arc<Mutex<MemoryStats>>,
}

impl SimpleMemoryManager {
    /// 创建新的内存管理器
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(MemoryStats::default())),
        }
    }
    
    /// 更新统计
    pub async fn update_stats<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut MemoryStats)
    {
        let mut stats = self.stats.lock().await;
        updater(&mut *stats);
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> MemoryStats {
        self.stats.lock().await.clone()
    }
}

impl Default for SimpleMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestObject {
        id: u64,
        data: String,
    }

    #[tokio::test]
    async fn test_simple_object_pool() {
        let pool = SimpleObjectPool::new(
            || TestObject { id: 1, data: "test".to_string() },
            5
        );
        
        // 获取对象
        let obj1 = pool.acquire().await.unwrap();
        assert_eq!(obj1.id, 1);
        
        // 归还对象
        pool.release(obj1).await;
        assert_eq!(pool.available().await, 1);
        
        // 再次获取应该重用对象
        let obj2 = pool.acquire().await.unwrap();
        assert_eq!(obj2.id, 1);
    }

    #[tokio::test]
    async fn test_memory_manager() {
        let memory_manager = SimpleMemoryManager::new();
        
        // 更新统计
        memory_manager.update_stats(|stats| {
            stats.pool_objects_created += 1;
            stats.total_allocations += 100;
        }).await;
        
        let stats = memory_manager.get_stats().await;
        assert_eq!(stats.pool_objects_created, 1);
        assert_eq!(stats.total_allocations, 100);
    }
}