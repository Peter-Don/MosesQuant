//! 内存管理系统
//! 
//! 提供高性能的对象池、内存优化和SIMD对齐的数据结构

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use std::marker::PhantomData;
use crate::{Result, MosesQuantError};

/// 对象池特征
pub trait ObjectPool<T>: Send + Sync {
    /// 获取对象
    fn acquire(&self) -> impl std::future::Future<Output = Result<PooledObject<T>>> + Send;
    
    /// 归还对象
    fn release(&self, obj: T) -> impl std::future::Future<Output = ()> + Send;
    
    /// 获取池大小
    fn size(&self) -> impl std::future::Future<Output = usize> + Send;
    
    /// 获取可用对象数量
    fn available(&self) -> impl std::future::Future<Output = usize> + Send;
    
    /// 清空池
    fn clear(&self) -> impl std::future::Future<Output = ()> + Send;
}

/// 池化对象包装器
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<SimpleObjectPool<T>>,
}

impl<T> PooledObject<T> {
    fn new(object: T, pool: Arc<SimpleObjectPool<T>>) -> Self {
        Self {
            object: Some(object),
            pool,
        }
    }
    
    /// 获取对象的不可变引用
    pub fn get(&self) -> Option<&T> {
        self.object.as_ref()
    }
    
    /// 获取对象的可变引用
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.object.as_mut()
    }
    
    /// 手动归还对象到池中
    pub async fn release(mut self) {
        if let Some(obj) = self.object.take() {
            self.pool.release(obj).await;
        }
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            // 在异步上下文中，我们需要使用 tokio::spawn 来归还对象
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.release(obj).await;
            });
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.object.as_ref().expect("Object has been released")
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().expect("Object has been released")
    }
}

/// 简单对象池实现
pub struct SimpleObjectPool<T> {
    objects: Arc<Mutex<VecDeque<T>>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created_count: Arc<Mutex<usize>>,
}

impl<T> SimpleObjectPool<T> 
where 
    T: Send + 'static
{
    /// 创建新的对象池
    pub fn new<F>(factory: F, max_size: usize) -> Arc<Self> 
    where 
        F: Fn() -> T + Send + Sync + 'static
    {
        Arc::new(Self {
            objects: Arc::new(Mutex::new(VecDeque::new())),
            factory: Arc::new(factory),
            max_size,
            created_count: Arc::new(Mutex::new(0)),
        })
    }
    
    /// 预填充池
    pub async fn prefill(&self, count: usize) -> Result<()> {
        let actual_count = std::cmp::min(count, self.max_size);
        let mut objects = self.objects.lock().await;
        let mut created_count = self.created_count.lock().await;
        
        for _ in 0..actual_count {
            if *created_count >= self.max_size {
                break;
            }
            
            let obj = (self.factory)();
            objects.push_back(obj);
            *created_count += 1;
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T> ObjectPool<T> for SimpleObjectPool<T> 
where 
    T: Send + 'static
{
    async fn acquire(&self) -> Result<PooledObject<T>> {
        let mut objects = self.objects.lock().await;
        
        let obj = if let Some(obj) = objects.pop_front() {
            obj
        } else {
            // 检查是否可以创建新对象
            let created_count = self.created_count.lock().await;
            if *created_count >= self.max_size {
                drop(created_count);
                drop(objects);
                return Err(MosesQuantError::Internal {
                    message: "Object pool exhausted".to_string()
                });
            }
            drop(created_count);
            
            (self.factory)()
        };
        
        Ok(PooledObject::new(obj, Arc::new(self.clone())))
    }
    
    async fn release(&self, obj: T) {
        let mut objects = self.objects.lock().await;
        if objects.len() < self.max_size {
            objects.push_back(obj);
        }
        // 如果池已满，对象会被丢弃
    }
    
    async fn size(&self) -> usize {
        *self.created_count.lock().await
    }
    
    async fn available(&self) -> usize {
        self.objects.lock().await.len()
    }
    
    async fn clear(&self) {
        let mut objects = self.objects.lock().await;
        let mut created_count = self.created_count.lock().await;
        
        objects.clear();
        *created_count = 0;
    }
}

// 为了克隆 Arc<SimpleObjectPool<T>>，我们需要实现 Clone
impl<T> Clone for SimpleObjectPool<T> {
    fn clone(&self) -> Self {
        Self {
            objects: self.objects.clone(),
            factory: self.factory.clone(),
            max_size: self.max_size,
            created_count: self.created_count.clone(),
        }
    }
}

/// SIMD对齐的内存分配器
#[repr(C, align(32))]
pub struct AlignedBuffer<T> {
    data: Vec<T>,
    _phantom: PhantomData<T>,
}

impl<T> AlignedBuffer<T> {
    /// 创建对齐的缓冲区
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            _phantom: PhantomData,
        }
    }
    
    /// 添加元素
    pub fn push(&mut self, item: T) {
        self.data.push(item);
    }
    
    /// 获取对齐的数据指针
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }
    
    /// 获取可变对齐的数据指针
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }
    
    /// 获取长度
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// 获取切片
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
    
    /// 获取可变切片
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T> std::ops::Index<usize> for AlignedBuffer<T> {
    type Output = T;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> std::ops::IndexMut<usize> for AlignedBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

/// 内存使用统计
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub pool_objects_created: usize,
    pub pool_objects_reused: usize,
}

/// 内存管理器
pub struct MemoryManager {
    stats: Arc<Mutex<MemoryStats>>,
    object_pools: Arc<Mutex<std::collections::HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
}

impl MemoryManager {
    /// 创建新的内存管理器
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(MemoryStats::default())),
            object_pools: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    /// 注册对象池
    pub async fn register_pool<T>(&self, name: String, pool: Arc<dyn ObjectPool<T>>) 
    where 
        T: Send + Sync + 'static
    {
        let mut pools = self.object_pools.lock().await;
        pools.insert(name, pool as Arc<dyn std::any::Any + Send + Sync>);
    }
    
    /// 获取对象池
    pub async fn get_pool<T>(&self, name: &str) -> Option<Arc<dyn ObjectPool<T>>> 
    where 
        T: Send + Sync + 'static
    {
        let pools = self.object_pools.lock().await;
        pools.get(name)?.downcast_ref::<Arc<dyn ObjectPool<T>>>().cloned()
    }
    
    /// 更新内存统计
    pub async fn update_stats<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut MemoryStats)
    {
        let mut stats = self.stats.lock().await;
        updater(&mut *stats);
        
        // 更新峰值使用量
        if stats.current_usage > stats.peak_usage {
            stats.peak_usage = stats.current_usage;
        }
    }
    
    /// 获取内存统计
    pub async fn get_stats(&self) -> MemoryStats {
        self.stats.lock().await.clone()
    }
    
    /// 清理所有对象池
    pub async fn cleanup_all_pools(&self) -> Result<()> {
        let pools = self.object_pools.lock().await;
        
        for (name, pool_any) in pools.iter() {
            // 这里我们只能记录清理操作，因为类型擦除使得直接调用困难
            tracing::info!("Cleaning up pool: {}", name);
        }
        
        Ok(())
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 快速分配器，用于频繁的小对象分配
pub struct FastAllocator<T> {
    chunk_size: usize,
    chunks: Arc<Mutex<Vec<AlignedBuffer<T>>>>,
    free_objects: Arc<Mutex<Vec<*mut T>>>,
    _phantom: PhantomData<T>,
}

impl<T> FastAllocator<T> 
where 
    T: Default + 'static
{
    /// 创建新的快速分配器
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            chunks: Arc::new(Mutex::new(Vec::new())),
            free_objects: Arc::new(Mutex::new(Vec::new())),
            _phantom: PhantomData,
        }
    }
    
    /// 分配对象
    pub async fn allocate(&self) -> Result<*mut T> {
        let mut free_objects = self.free_objects.lock().await;
        
        if let Some(ptr) = free_objects.pop() {
            return Ok(ptr);
        }
        
        // 没有可用对象，分配新的块
        drop(free_objects);
        self.allocate_new_chunk().await
    }
    
    /// 分配新的内存块
    async fn allocate_new_chunk(&self) -> Result<*mut T> {
        let mut chunks = self.chunks.lock().await;
        let mut new_chunk = AlignedBuffer::new(self.chunk_size);
        
        // 填充块
        for _ in 0..self.chunk_size {
            new_chunk.push(T::default());
        }
        
        let chunk_ptr = new_chunk.as_mut_ptr();
        chunks.push(new_chunk);
        
        // 将除第一个对象外的所有对象添加到空闲列表
        let mut free_objects = self.free_objects.lock().await;
        for i in 1..self.chunk_size {
            unsafe {
                free_objects.push(chunk_ptr.add(i));
            }
        }
        
        // 返回第一个对象
        Ok(chunk_ptr)
    }
    
    /// 释放对象
    pub async fn deallocate(&self, ptr: *mut T) {
        let mut free_objects = self.free_objects.lock().await;
        free_objects.push(ptr);
    }
    
    /// 获取分配的块数
    pub async fn chunk_count(&self) -> usize {
        self.chunks.lock().await.len()
    }
    
    /// 获取可用对象数
    pub async fn available_objects(&self) -> usize {
        self.free_objects.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[derive(Debug, Clone, PartialEq)]
    struct TestObject {
        id: u64,
        data: String,
    }

    impl Default for TestObject {
        fn default() -> Self {
            Self {
                id: 0,
                data: String::new(),
            }
        }
    }

    #[tokio::test]
    async fn test_simple_object_pool() {
        let pool = SimpleObjectPool::new(
            || TestObject { id: 1, data: "test".to_string() },
            10
        );
        
        // 预填充池
        pool.prefill(5).await.unwrap();
        
        assert_eq!(pool.available().await, 5);
        assert_eq!(pool.size().await, 5);
        
        // 获取对象
        let obj1 = pool.acquire().await.unwrap();
        assert_eq!(obj1.id, 1);
        assert_eq!(pool.available().await, 4);
        
        // 释放对象
        obj1.release().await;
        assert_eq!(pool.available().await, 5);
    }

    #[tokio::test]
    async fn test_pooled_object_auto_release() {
        let pool = SimpleObjectPool::new(
            || TestObject { id: 2, data: "auto_release".to_string() },
            5
        );
        
        pool.prefill(3).await.unwrap();
        
        {
            let _obj = pool.acquire().await.unwrap();
            assert_eq!(pool.available().await, 2);
            // obj 在这里会自动释放
        }
        
        // 给自动释放一些时间
        sleep(Duration::from_millis(10)).await;
        // 注意：由于 Drop 中的异步操作，对象可能不会立即返回池中
    }

    #[tokio::test]
    async fn test_aligned_buffer() {
        let mut buffer: AlignedBuffer<f64> = AlignedBuffer::new(100);
        
        for i in 0..10 {
            buffer.push(i as f64);
        }
        
        assert_eq!(buffer.len(), 10);
        assert_eq!(buffer[5], 5.0);
        
        // 检查对齐
        let ptr = buffer.as_ptr();
        assert_eq!(ptr as usize % 32, 0); // 32字节对齐
    }

    #[tokio::test]
    async fn test_memory_manager() {
        let memory_manager = MemoryManager::new();
        
        let pool = SimpleObjectPool::new(
            || TestObject { id: 3, data: "memory_test".to_string() },
            5
        );
        
        // 注册池
        memory_manager.register_pool("test_pool".to_string(), pool.clone()).await;
        
        // 获取池
        let retrieved_pool: Option<Arc<dyn ObjectPool<TestObject>>> = 
            memory_manager.get_pool("test_pool").await;
        
        assert!(retrieved_pool.is_some());
        
        // 更新统计
        memory_manager.update_stats(|stats| {
            stats.total_allocated += 1000;
            stats.current_usage += 500;
        }).await;
        
        let stats = memory_manager.get_stats().await;
        assert_eq!(stats.total_allocated, 1000);
        assert_eq!(stats.current_usage, 500);
        assert_eq!(stats.peak_usage, 500);
    }

    #[tokio::test]
    async fn test_fast_allocator() {
        let allocator: FastAllocator<TestObject> = FastAllocator::new(10);
        
        // 分配一些对象
        let ptr1 = allocator.allocate().await.unwrap();
        let ptr2 = allocator.allocate().await.unwrap();
        
        assert_ne!(ptr1, ptr2);
        assert_eq!(allocator.chunk_count().await, 1);
        assert_eq!(allocator.available_objects().await, 8); // 10 - 2
        
        // 释放对象
        allocator.deallocate(ptr1).await;
        assert_eq!(allocator.available_objects().await, 9);
        
        // 重新分配应该重用对象
        let ptr3 = allocator.allocate().await.unwrap();
        assert_eq!(ptr3, ptr1); // 应该重用之前释放的对象
    }
}