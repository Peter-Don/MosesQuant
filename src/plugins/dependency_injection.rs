//! 依赖注入和解析系统
//! 
//! 提供强类型的依赖注入容器，支持生命周期管理、循环依赖检测和动态解析

use crate::plugins::core::*;
use crate::{Result, MosesQuantError};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use async_trait::async_trait;
use tracing::{debug, info, warn, error};

/// 依赖注入容器
pub struct DIContainer {
    /// 服务注册表
    services: Arc<RwLock<HashMap<ServiceKey, Box<dyn ServiceFactory>>>>,
    /// 单例服务实例缓存
    singletons: Arc<RwLock<HashMap<ServiceKey, Arc<dyn Any + Send + Sync>>>>,
    /// 服务元数据
    metadata: Arc<RwLock<HashMap<ServiceKey, ServiceMetadata>>>,
    /// 依赖图
    dependency_graph: Arc<RwLock<HashMap<ServiceKey, Vec<ServiceKey>>>>,
    /// 容器配置
    config: DIContainerConfig,
    /// 解析上下文栈（用于检测循环依赖）
    resolution_stack: Arc<Mutex<Vec<ServiceKey>>>,
}

/// 依赖注入容器配置
#[derive(Debug, Clone)]
pub struct DIContainerConfig {
    /// 是否启用循环依赖检测
    pub enable_circular_dependency_detection: bool,
    /// 最大解析深度
    pub max_resolution_depth: usize,
    /// 是否启用懒加载
    pub enable_lazy_loading: bool,
    /// 是否启用服务验证
    pub enable_service_validation: bool,
    /// 服务超时时间
    pub service_timeout: std::time::Duration,
}

impl Default for DIContainerConfig {
    fn default() -> Self {
        Self {
            enable_circular_dependency_detection: true,
            max_resolution_depth: 50,
            enable_lazy_loading: true,
            enable_service_validation: true,
            service_timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// 服务键，用于唯一标识服务
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceKey {
    /// 类型ID
    pub type_id: TypeId,
    /// 可选的名称标识
    pub name: Option<String>,
}

impl ServiceKey {
    /// 创建基于类型的服务键
    pub fn of_type<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name: None,
        }
    }

    /// 创建命名服务键
    pub fn named<T: 'static>(name: impl Into<String>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name: Some(name.into()),
        }
    }
}

/// 服务生命周期
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceLifetime {
    /// 瞬态 - 每次请求创建新实例
    Transient,
    /// 单例 - 容器生命周期内唯一实例
    Singleton,
    /// 作用域 - 在特定作用域内单例
    Scoped,
}

/// 服务元数据
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// 服务生命周期
    pub lifetime: ServiceLifetime,
    /// 服务描述
    pub description: String,
    /// 依赖的服务键列表
    pub dependencies: Vec<ServiceKey>,
    /// 是否为可选依赖
    pub optional_dependencies: Vec<ServiceKey>,
    /// 创建时间
    pub created_at: std::time::Instant,
    /// 使用计数
    pub usage_count: u64,
}

/// 服务工厂trait
#[async_trait]
pub trait ServiceFactory: Send + Sync {
    /// 创建服务实例
    async fn create(&self, container: &DIContainer) -> Result<Arc<dyn Any + Send + Sync>>;
    
    /// 获取服务元数据
    fn metadata(&self) -> &ServiceMetadata;
    
    /// 验证服务配置
    async fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// 类型化服务工厂
pub struct TypedServiceFactory<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
{
    factory_fn: F,
    metadata: ServiceMetadata,
    _phantom: PhantomData<T>,
}

impl<T, F> TypedServiceFactory<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
{
    pub fn new(factory_fn: F, metadata: ServiceMetadata) -> Self {
        Self {
            factory_fn,
            metadata,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> ServiceFactory for TypedServiceFactory<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
{
    async fn create(&self, container: &DIContainer) -> Result<Arc<dyn Any + Send + Sync>> {
        let instance = (self.factory_fn)(container).await?;
        Ok(Arc::new(instance))
    }

    fn metadata(&self) -> &ServiceMetadata {
        &self.metadata
    }
}

/// 依赖注入构建器
pub struct DIBuilder {
    container: DIContainer,
}

impl DIBuilder {
    pub fn new() -> Self {
        Self {
            container: DIContainer::new(DIContainerConfig::default()),
        }
    }

    pub fn with_config(config: DIContainerConfig) -> Self {
        Self {
            container: DIContainer::new(config),
        }
    }

    /// 注册瞬态服务
    pub async fn register_transient<T, F>(mut self, factory: F) -> Result<Self>
    where
        T: Send + Sync + 'static,
        F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync + 'static,
    {
        let metadata = ServiceMetadata {
            lifetime: ServiceLifetime::Transient,
            description: format!("Transient service: {}", std::any::type_name::<T>()),
            dependencies: vec![],
            optional_dependencies: vec![],
            created_at: std::time::Instant::now(),
            usage_count: 0,
        };

        let service_factory = Box::new(TypedServiceFactory::new(factory, metadata.clone()));
        let service_key = ServiceKey::of_type::<T>();

        self.container.register_service(service_key, service_factory, metadata).await?;
        Ok(self)
    }

    /// 注册单例服务
    pub async fn register_singleton<T, F>(mut self, factory: F) -> Result<Self>
    where
        T: Send + Sync + 'static,
        F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync + 'static,
    {
        let metadata = ServiceMetadata {
            lifetime: ServiceLifetime::Singleton,
            description: format!("Singleton service: {}", std::any::type_name::<T>()),
            dependencies: vec![],
            optional_dependencies: vec![],
            created_at: std::time::Instant::now(),
            usage_count: 0,
        };

        let service_factory = Box::new(TypedServiceFactory::new(factory, metadata.clone()));
        let service_key = ServiceKey::of_type::<T>();

        self.container.register_service(service_key, service_factory, metadata).await?;
        Ok(self)
    }

    /// 注册命名服务
    pub async fn register_named<T, F>(mut self, name: impl Into<String>, factory: F) -> Result<Self>
    where
        T: Send + Sync + 'static,
        F: Fn(&DIContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync + 'static,
    {
        let metadata = ServiceMetadata {
            lifetime: ServiceLifetime::Singleton,
            description: format!("Named service: {} ({})", name.into(), std::any::type_name::<T>()),
            dependencies: vec![],
            optional_dependencies: vec![],
            created_at: std::time::Instant::now(),
            usage_count: 0,
        };

        let service_factory = Box::new(TypedServiceFactory::new(factory, metadata.clone()));
        let service_key = ServiceKey::named::<T>(name);

        self.container.register_service(service_key, service_factory, metadata).await?;
        Ok(self)
    }

    /// 构建容器
    pub async fn build(self) -> Result<DIContainer> {
        // 验证依赖图
        self.container.validate_dependency_graph().await?;
        Ok(self.container)
    }
}

impl DIContainer {
    /// 创建新的依赖注入容器
    pub fn new(config: DIContainerConfig) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            singletons: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            config,
            resolution_stack: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 注册服务
    pub async fn register_service(
        &self,
        key: ServiceKey,
        factory: Box<dyn ServiceFactory>,
        metadata: ServiceMetadata,
    ) -> Result<()> {
        // 验证服务工厂
        if self.config.enable_service_validation {
            factory.validate().await?;
        }

        {
            let mut services = self.services.write().await;
            services.insert(key.clone(), factory);
        }

        {
            let mut metadata_map = self.metadata.write().await;
            metadata_map.insert(key.clone(), metadata);
        }

        debug!("Registered service: {:?}", key);
        Ok(())
    }

    /// 解析服务
    pub async fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        let service_key = ServiceKey::of_type::<T>();
        self.resolve_by_key(&service_key).await
    }

    /// 解析命名服务
    pub async fn resolve_named<T: Send + Sync + 'static>(&self, name: impl Into<String>) -> Result<Arc<T>> {
        let service_key = ServiceKey::named::<T>(name);
        self.resolve_by_key(&service_key).await
    }

    /// 通过服务键解析服务
    pub async fn resolve_by_key<T: Send + Sync + 'static>(&self, key: &ServiceKey) -> Result<Arc<T>> {
        // 检查循环依赖
        if self.config.enable_circular_dependency_detection {
            self.check_circular_dependency(key).await?;
        }

        // 添加到解析栈
        {
            let mut stack = self.resolution_stack.lock().await;
            stack.push(key.clone());
            
            if stack.len() > self.config.max_resolution_depth {
                return Err(MosesQuantError::DependencyInjection {
                    message: "Maximum resolution depth exceeded".to_string()
                });
            }
        }

        let result = self.resolve_internal(key).await;

        // 从解析栈移除
        {
            let mut stack = self.resolution_stack.lock().await;
            stack.pop();
        }

        let instance = result?;
        
        // 类型转换
        instance.downcast::<T>()
            .map_err(|_| MosesQuantError::DependencyInjection {
                message: format!("Failed to downcast service to type: {}", std::any::type_name::<T>())
            })
    }

    /// 内部解析逻辑
    async fn resolve_internal(&self, key: &ServiceKey) -> Result<Arc<dyn Any + Send + Sync>> {
        // 检查是否为单例且已创建
        {
            let metadata_map = self.metadata.read().await;
            if let Some(metadata) = metadata_map.get(key) {
                if metadata.lifetime == ServiceLifetime::Singleton {
                    let singletons = self.singletons.read().await;
                    if let Some(instance) = singletons.get(key) {
                        return Ok(instance.clone());
                    }
                }
            }
        }

        // 获取服务工厂
        let factory = {
            let services = self.services.read().await;
            services.get(key).ok_or_else(|| MosesQuantError::DependencyInjection {
                message: format!("Service not registered: {:?}", key)
            })?.as_ref() as *const dyn ServiceFactory
        };

        // 创建服务实例
        let instance = unsafe { &*factory }.create(self).await?;

        // 对于单例，缓存实例
        {
            let metadata_map = self.metadata.read().await;
            if let Some(metadata) = metadata_map.get(key) {
                if metadata.lifetime == ServiceLifetime::Singleton {
                    let mut singletons = self.singletons.write().await;
                    singletons.insert(key.clone(), instance.clone());
                }
            }
        }

        // 更新使用计数
        {
            let mut metadata_map = self.metadata.write().await;
            if let Some(metadata) = metadata_map.get_mut(key) {
                metadata.usage_count += 1;
            }
        }

        Ok(instance)
    }

    /// 检查循环依赖
    async fn check_circular_dependency(&self, key: &ServiceKey) -> Result<()> {
        let stack = self.resolution_stack.lock().await;
        if stack.contains(key) {
            return Err(MosesQuantError::DependencyInjection {
                message: format!("Circular dependency detected: {:?}", key)
            });
        }
        Ok(())
    }

    /// 验证依赖图
    pub async fn validate_dependency_graph(&self) -> Result<()> {
        let metadata_map = self.metadata.read().await;
        let services = self.services.read().await;

        // 构建依赖图
        let mut graph = HashMap::new();
        for (key, metadata) in metadata_map.iter() {
            graph.insert(key.clone(), metadata.dependencies.clone());
        }

        // 检查所有依赖是否已注册
        for (service_key, dependencies) in &graph {
            for dep_key in dependencies {
                if !services.contains_key(dep_key) {
                    return Err(MosesQuantError::DependencyInjection {
                        message: format!("Unregistered dependency: {:?} required by {:?}", dep_key, service_key)
                    });
                }
            }
        }

        // 拓扑排序检测循环依赖
        self.topological_sort(&graph)?;

        info!("Dependency graph validation completed successfully");
        Ok(())
    }

    /// 拓扑排序检测循环依赖
    fn topological_sort(&self, graph: &HashMap<ServiceKey, Vec<ServiceKey>>) -> Result<Vec<ServiceKey>> {
        let mut in_degree = HashMap::new();
        let mut adj_list = HashMap::new();

        // 初始化入度和邻接表
        for (node, deps) in graph {
            in_degree.entry(node.clone()).or_insert(0);
            adj_list.entry(node.clone()).or_insert(Vec::new());

            for dep in deps {
                in_degree.entry(dep.clone()).or_insert(0);
                adj_list.entry(dep.clone()).or_insert(Vec::new());
                
                adj_list.get_mut(dep).unwrap().push(node.clone());
                *in_degree.get_mut(node).unwrap() += 1;
            }
        }

        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // 将入度为0的节点加入队列
        for (node, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(node.clone());
            }
        }

        // 拓扑排序
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // 检查是否存在循环依赖
        if result.len() != in_degree.len() {
            return Err(MosesQuantError::DependencyInjection {
                message: "Circular dependency detected in service graph".to_string()
            });
        }

        Ok(result)
    }

    /// 获取所有已注册的服务
    pub async fn get_registered_services(&self) -> Vec<ServiceKey> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// 获取服务元数据
    pub async fn get_service_metadata(&self, key: &ServiceKey) -> Option<ServiceMetadata> {
        let metadata_map = self.metadata.read().await;
        metadata_map.get(key).cloned()
    }

    /// 检查服务是否已注册
    pub async fn is_registered(&self, key: &ServiceKey) -> bool {
        let services = self.services.read().await;
        services.contains_key(key)
    }

    /// 获取容器统计信息
    pub async fn get_statistics(&self) -> DIContainerStats {
        let services = self.services.read().await;
        let singletons = self.singletons.read().await;
        let metadata_map = self.metadata.read().await;

        let mut total_usage = 0;
        let mut transient_count = 0;
        let mut singleton_count = 0;
        let mut scoped_count = 0;

        for metadata in metadata_map.values() {
            total_usage += metadata.usage_count;
            match metadata.lifetime {
                ServiceLifetime::Transient => transient_count += 1,
                ServiceLifetime::Singleton => singleton_count += 1,
                ServiceLifetime::Scoped => scoped_count += 1,
            }
        }

        DIContainerStats {
            total_services: services.len(),
            singleton_instances: singletons.len(),
            transient_services: transient_count,
            singleton_services: singleton_count,
            scoped_services: scoped_count,
            total_usage_count: total_usage,
        }
    }

    /// 清理未使用的单例实例
    pub async fn cleanup_unused_singletons(&self) {
        let mut singletons = self.singletons.write().await;
        let metadata_map = self.metadata.read().await;

        let keys_to_remove: Vec<ServiceKey> = singletons
            .keys()
            .filter(|key| {
                if let Some(metadata) = metadata_map.get(key) {
                    metadata.usage_count == 0
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        for key in keys_to_remove {
            singletons.remove(&key);
            debug!("Cleaned up unused singleton: {:?}", key);
        }
    }
}

/// 依赖注入容器统计信息
#[derive(Debug, Clone)]
pub struct DIContainerStats {
    /// 总注册服务数
    pub total_services: usize,
    /// 单例实例数
    pub singleton_instances: usize,
    /// 瞬态服务数
    pub transient_services: usize,
    /// 单例服务数
    pub singleton_services: usize,
    /// 作用域服务数
    pub scoped_services: usize,
    /// 总使用次数
    pub total_usage_count: u64,
}

/// 依赖注入装饰器宏
#[macro_export]
macro_rules! injectable {
    ($type:ty) => {
        impl $type {
            pub async fn resolve_from_container(container: &DIContainer) -> Result<Arc<Self>> {
                container.resolve::<Self>().await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // 测试服务接口
    #[async_trait]
    trait TestService: Send + Sync {
        async fn get_value(&self) -> u32;
    }

    // 测试服务实现
    struct TestServiceImpl {
        value: AtomicU32,
    }

    impl TestServiceImpl {
        fn new(value: u32) -> Self {
            Self {
                value: AtomicU32::new(value),
            }
        }
    }

    #[async_trait]
    impl TestService for TestServiceImpl {
        async fn get_value(&self) -> u32 {
            self.value.load(Ordering::Relaxed)
        }
    }

    // 依赖其他服务的测试服务
    struct DependentService {
        dependency: Arc<dyn TestService>,
    }

    impl DependentService {
        fn new(dependency: Arc<dyn TestService>) -> Self {
            Self { dependency }
        }

        async fn get_dependent_value(&self) -> u32 {
            self.dependency.get_value().await * 2
        }
    }

    #[tokio::test]
    async fn test_simple_service_resolution() {
        let container = DIBuilder::new()
            .register_singleton::<TestServiceImpl, _>(|_| {
                Box::pin(async { Ok(TestServiceImpl::new(42)) })
            })
            .await
            .unwrap()
            .build()
            .await
            .unwrap();

        let service = container.resolve::<TestServiceImpl>().await.unwrap();
        assert_eq!(service.get_value().await, 42);

        // 测试单例行为
        let service2 = container.resolve::<TestServiceImpl>().await.unwrap();
        assert!(Arc::ptr_eq(&service, &service2));
    }

    #[tokio::test]
    async fn test_named_service_resolution() {
        let container = DIBuilder::new()
            .register_named::<TestServiceImpl, _>("test_service", |_| {
                Box::pin(async { Ok(TestServiceImpl::new(100)) })
            })
            .await
            .unwrap()
            .build()
            .await
            .unwrap();

        let service = container.resolve_named::<TestServiceImpl>("test_service").await.unwrap();
        assert_eq!(service.get_value().await, 100);
    }

    #[tokio::test]
    async fn test_transient_service() {
        let container = DIBuilder::new()
            .register_transient::<TestServiceImpl, _>(|_| {
                Box::pin(async { Ok(TestServiceImpl::new(42)) })
            })
            .await
            .unwrap()
            .build()
            .await
            .unwrap();

        let service1 = container.resolve::<TestServiceImpl>().await.unwrap();
        let service2 = container.resolve::<TestServiceImpl>().await.unwrap();

        // 瞬态服务应该是不同的实例
        assert!(!Arc::ptr_eq(&service1, &service2));
        assert_eq!(service1.get_value().await, 42);
        assert_eq!(service2.get_value().await, 42);
    }

    #[tokio::test]
    async fn test_service_with_dependencies() {
        let container = DIBuilder::new()
            .register_singleton::<TestServiceImpl, _>(|_| {
                Box::pin(async { Ok(TestServiceImpl::new(21)) })
            })
            .await
            .unwrap()
            .register_singleton::<DependentService, _>(|container| {
                Box::pin(async move {
                    let dependency = container.resolve::<TestServiceImpl>().await?;
                    Ok(DependentService::new(dependency))
                })
            })
            .await
            .unwrap()
            .build()
            .await
            .unwrap();

        let dependent = container.resolve::<DependentService>().await.unwrap();
        assert_eq!(dependent.get_dependent_value().await, 42); // 21 * 2
    }

    #[tokio::test]
    async fn test_container_statistics() {
        let container = DIBuilder::new()
            .register_singleton::<TestServiceImpl, _>(|_| {
                Box::pin(async { Ok(TestServiceImpl::new(42)) })
            })
            .await
            .unwrap()
            .register_transient::<DependentService, _>(|container| {
                Box::pin(async move {
                    let dependency = container.resolve::<TestServiceImpl>().await?;
                    Ok(DependentService::new(dependency))
                })
            })
            .await
            .unwrap()
            .build()
            .await
            .unwrap();

        // 解析一些服务
        let _service1 = container.resolve::<TestServiceImpl>().await.unwrap();
        let _service2 = container.resolve::<DependentService>().await.unwrap();

        let stats = container.get_statistics().await;
        assert_eq!(stats.total_services, 2);
        assert_eq!(stats.singleton_services, 1);
        assert_eq!(stats.transient_services, 1);
        assert!(stats.total_usage_count > 0);
    }

    #[tokio::test]
    async fn test_unregistered_service_error() {
        let container = DIContainer::new(DIContainerConfig::default());
        
        let result = container.resolve::<TestServiceImpl>().await;
        assert!(result.is_err());
    }
}