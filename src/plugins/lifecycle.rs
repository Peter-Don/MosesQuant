//! 插件生命周期管理器
//! 
//! 负责插件的加载、初始化、启动、停止和卸载的完整生命周期管理

use super::core::*;
use crate::{Result, MosesQuantError};
use crate::types::PluginId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};

/// 插件生命周期状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLifecycleState {
    /// 插件ID
    pub plugin_id: PluginId,
    /// 当前状态
    pub current_state: PluginState,
    /// 目标状态
    pub target_state: Option<PluginState>,
    /// 状态变更时间
    pub state_changed_at: Instant,
    /// 重试次数
    pub retry_count: u32,
    /// 最后错误
    pub last_error: Option<String>,
}

/// 生命周期管理器配置
#[derive(Debug, Clone)]
pub struct LifecycleManagerConfig {
    /// 启动超时时间
    pub startup_timeout: Duration,
    /// 停止超时时间
    pub shutdown_timeout: Duration,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔
    pub retry_interval: Duration,
    /// 健康检查间隔
    pub health_check_interval: Duration,
    /// 并发启动数
    pub concurrent_startups: usize,
}

impl Default for LifecycleManagerConfig {
    fn default() -> Self {
        Self {
            startup_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_interval: Duration::from_secs(5),
            health_check_interval: Duration::from_secs(60),
            concurrent_startups: 4,
        }
    }
}

/// 插件生命周期事件
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    /// 插件ID
    pub plugin_id: PluginId,
    /// 事件类型
    pub event_type: LifecycleEventType,
    /// 事件时间
    pub timestamp: crate::TimestampNs,
    /// 事件详情
    pub details: HashMap<String, serde_json::Value>,
}

/// 生命周期事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEventType {
    /// 插件注册
    Registered,
    /// 插件初始化开始
    InitializationStarted,
    /// 插件初始化完成
    InitializationCompleted,
    /// 插件启动开始
    StartupStarted,
    /// 插件启动完成
    StartupCompleted,
    /// 插件暂停
    Paused,
    /// 插件恢复
    Resumed,
    /// 插件停止开始
    ShutdownStarted,
    /// 插件停止完成
    ShutdownCompleted,
    /// 插件错误
    Error,
    /// 健康检查失败
    HealthCheckFailed,
}

/// 插件生命周期管理器
pub struct PluginLifecycleManager {
    /// 已注册的插件
    plugins: Arc<RwLock<HashMap<PluginId, Arc<Mutex<dyn Plugin>>>>>,
    /// 插件上下文
    contexts: Arc<RwLock<HashMap<PluginId, PluginContext>>>,
    /// 生命周期状态
    states: Arc<RwLock<HashMap<PluginId, PluginLifecycleState>>>,
    /// 依赖关系图
    dependency_graph: Arc<RwLock<HashMap<PluginId, Vec<PluginId>>>>,
    /// 配置
    config: LifecycleManagerConfig,
    /// 事件总线
    event_bus: Option<Arc<crate::SimpleEventBus>>,
    /// 健康检查任务句柄
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PluginLifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new(config: LifecycleManagerConfig, event_bus: Option<Arc<crate::SimpleEventBus>>) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            contexts: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            config,
            event_bus,
            health_check_handle: None,
        }
    }

    /// 注册插件
    pub async fn register_plugin(
        &self, 
        plugin: Arc<Mutex<dyn Plugin>>,
        context: PluginContext,
    ) -> Result<()> {
        let metadata = {
            let plugin_guard = plugin.lock().await;
            plugin_guard.metadata().clone()
        };

        let plugin_id = metadata.id.clone();

        // 检查是否已注册
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&plugin_id) {
                return Err(MosesQuantError::PluginAlreadyRegistered { plugin_id });
            }
        }

        // 验证依赖关系
        self.validate_dependencies(&metadata).await?;

        // 注册插件
        {
            let mut plugins = self.plugins.write().await;
            let mut contexts = self.contexts.write().await;
            let mut states = self.states.write().await;

            plugins.insert(plugin_id.clone(), plugin);
            contexts.insert(plugin_id.clone(), context);
            states.insert(plugin_id.clone(), PluginLifecycleState {
                plugin_id: plugin_id.clone(),
                current_state: PluginState::Uninitialized,
                target_state: None,
                state_changed_at: Instant::now(),
                retry_count: 0,
                last_error: None,
            });
        }

        // 更新依赖关系图
        self.update_dependency_graph(&metadata).await;

        // 发布注册事件
        self.publish_lifecycle_event(
            plugin_id.clone(),
            LifecycleEventType::Registered,
            HashMap::new(),
        ).await;

        info!("Plugin '{}' registered successfully", plugin_id);
        Ok(())
    }

    /// 初始化插件
    pub async fn initialize_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        self.transition_plugin_state(plugin_id, PluginState::Initializing).await?;

        let (plugin, context) = {
            let plugins = self.plugins.read().await;
            let contexts = self.contexts.read().await;
            
            let plugin = plugins.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            let context = contexts.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            
            (plugin, context)
        };

        self.publish_lifecycle_event(
            plugin_id.clone(),
            LifecycleEventType::InitializationStarted,
            HashMap::new(),
        ).await;

        // 执行初始化
        let result = tokio::time::timeout(
            self.config.startup_timeout,
            async {
                let mut plugin_guard = plugin.lock().await;
                plugin_guard.initialize(&context).await
            }
        ).await;

        match result {
            Ok(Ok(_)) => {
                self.transition_plugin_state(plugin_id, PluginState::Stopped).await?;
                self.publish_lifecycle_event(
                    plugin_id.clone(),
                    LifecycleEventType::InitializationCompleted,
                    HashMap::new(),
                ).await;
                info!("Plugin '{}' initialized successfully", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, &e).await;
                Err(e)
            }
            Err(_) => {
                let error = MosesQuantError::Internal {
                    message: format!("Plugin '{}' initialization timeout", plugin_id)
                };
                self.handle_plugin_error(plugin_id, &error).await;
                Err(error)
            }
        }
    }

    /// 启动插件
    pub async fn start_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // 检查依赖是否已启动
        self.check_dependencies_started(plugin_id).await?;

        self.transition_plugin_state(plugin_id, PluginState::Running).await?;

        let (plugin, context) = {
            let plugins = self.plugins.read().await;
            let contexts = self.contexts.read().await;
            
            let plugin = plugins.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            let context = contexts.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            
            (plugin, context)
        };

        self.publish_lifecycle_event(
            plugin_id.clone(),
            LifecycleEventType::StartupStarted,
            HashMap::new(),
        ).await;

        // 执行启动
        let result = tokio::time::timeout(
            self.config.startup_timeout,
            async {
                let mut plugin_guard = plugin.lock().await;
                plugin_guard.start(&context).await
            }
        ).await;

        match result {
            Ok(Ok(_)) => {
                self.publish_lifecycle_event(
                    plugin_id.clone(),
                    LifecycleEventType::StartupCompleted,
                    HashMap::new(),
                ).await;
                info!("Plugin '{}' started successfully", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, &e).await;
                Err(e)
            }
            Err(_) => {
                let error = MosesQuantError::Internal {
                    message: format!("Plugin '{}' startup timeout", plugin_id)
                };
                self.handle_plugin_error(plugin_id, &error).await;
                Err(error)
            }
        }
    }

    /// 停止插件
    pub async fn stop_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        self.transition_plugin_state(plugin_id, PluginState::Stopping).await?;

        let (plugin, context) = {
            let plugins = self.plugins.read().await;
            let contexts = self.contexts.read().await;
            
            let plugin = plugins.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            let context = contexts.get(plugin_id)
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
                .clone();
            
            (plugin, context)
        };

        self.publish_lifecycle_event(
            plugin_id.clone(),
            LifecycleEventType::ShutdownStarted,
            HashMap::new(),
        ).await;

        // 执行停止
        let result = tokio::time::timeout(
            self.config.shutdown_timeout,
            async {
                let mut plugin_guard = plugin.lock().await;
                plugin_guard.stop(&context).await
            }
        ).await;

        match result {
            Ok(Ok(_)) => {
                self.transition_plugin_state(plugin_id, PluginState::Stopped).await?;
                self.publish_lifecycle_event(
                    plugin_id.clone(),
                    LifecycleEventType::ShutdownCompleted,
                    HashMap::new(),
                ).await;
                info!("Plugin '{}' stopped successfully", plugin_id);
                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, &e).await;
                Err(e)
            }
            Err(_) => {
                let error = MosesQuantError::Internal {
                    message: format!("Plugin '{}' shutdown timeout", plugin_id)
                };
                self.handle_plugin_error(plugin_id, &error).await;
                Err(error)
            }
        }
    }

    /// 批量启动插件（按依赖顺序）
    pub async fn start_all_plugins(&self) -> Result<()> {
        let startup_order = self.calculate_startup_order().await?;
        
        for batch in startup_order {
            // 并发启动同一批次的插件
            let mut tasks = Vec::new();
            
            for plugin_id in batch {
                let manager = self.clone();
                let id = plugin_id.clone();
                
                let task = tokio::spawn(async move {
                    if let Err(e) = manager.initialize_plugin(&id).await {
                        error!("Failed to initialize plugin '{}': {:?}", id, e);
                        return Err(e);
                    }
                    
                    if let Err(e) = manager.start_plugin(&id).await {
                        error!("Failed to start plugin '{}': {:?}", id, e);
                        return Err(e);
                    }
                    
                    Ok(())
                });
                
                tasks.push(task);
            }
            
            // 等待当前批次完成
            for task in tasks {
                task.await.map_err(|e| MosesQuantError::Internal {
                    message: format!("Plugin startup task failed: {}", e)
                })??;
            }
        }
        
        info!("All plugins started successfully");
        Ok(())
    }

    /// 批量停止插件（逆依赖顺序）
    pub async fn stop_all_plugins(&self) -> Result<()> {
        let mut shutdown_order = self.calculate_startup_order().await?;
        shutdown_order.reverse(); // 逆序停止
        
        for batch in shutdown_order {
            let mut tasks = Vec::new();
            
            for plugin_id in batch {
                let manager = self.clone();
                let id = plugin_id.clone();
                
                let task = tokio::spawn(async move {
                    if let Err(e) = manager.stop_plugin(&id).await {
                        error!("Failed to stop plugin '{}': {:?}", id, e);
                        return Err(e);
                    }
                    Ok(())
                });
                
                tasks.push(task);
            }
            
            // 等待当前批次完成
            for task in tasks {
                task.await.map_err(|e| MosesQuantError::Internal {
                    message: format!("Plugin shutdown task failed: {}", e)
                })??;
            }
        }
        
        info!("All plugins stopped successfully");
        Ok(())
    }

    /// 获取插件状态
    pub async fn get_plugin_state(&self, plugin_id: &PluginId) -> Option<PluginLifecycleState> {
        let states = self.states.read().await;
        states.get(plugin_id).cloned()
    }

    /// 获取所有插件状态
    pub async fn get_all_plugin_states(&self) -> HashMap<PluginId, PluginLifecycleState> {
        let states = self.states.read().await;
        states.clone()
    }

    /// 启动健康检查
    pub async fn start_health_checks(&mut self) {
        if self.health_check_handle.is_some() {
            return; // 已经在运行
        }

        let plugins = self.plugins.clone();
        let states = self.states.clone();
        let event_bus = self.event_bus.clone();
        let interval = self.config.health_check_interval;

        let handle = tokio::spawn(async move {
            let mut check_interval = tokio::time::interval(interval);
            
            loop {
                check_interval.tick().await;
                
                let plugin_list = {
                    let plugins_guard = plugins.read().await;
                    plugins_guard.keys().cloned().collect::<Vec<_>>()
                };

                for plugin_id in plugin_list {
                    // 检查插件是否在运行状态
                    let is_running = {
                        let states_guard = states.read().await;
                        states_guard.get(&plugin_id)
                            .map(|state| state.current_state == PluginState::Running)
                            .unwrap_or(false)
                    };

                    if !is_running {
                        continue;
                    }

                    // 执行健康检查
                    let health_result = {
                        let plugins_guard = plugins.read().await;
                        if let Some(plugin) = plugins_guard.get(&plugin_id) {
                            let plugin_guard = plugin.lock().await;
                            plugin_guard.health_check().await
                        } else {
                            continue;
                        }
                    };

                    match health_result {
                        Ok(health_status) => {
                            if !health_status.healthy {
                                warn!("Plugin '{}' health check failed: {}", plugin_id, health_status.message);
                                
                                // 发布健康检查失败事件
                                if let Some(ref event_bus) = event_bus {
                                    let _ = event_bus.publish(
                                        "plugin_health_check_failed",
                                        &format!("{{\"plugin_id\": \"{}\", \"message\": \"{}\"}}", 
                                                plugin_id, health_status.message)
                                    ).await;
                                }
                            } else {
                                debug!("Plugin '{}' health check passed", plugin_id);
                            }
                        }
                        Err(e) => {
                            error!("Plugin '{}' health check error: {:?}", plugin_id, e);
                        }
                    }
                }
            }
        });

        self.health_check_handle = Some(handle);
        info!("Plugin health checks started");
    }

    /// 停止健康检查
    pub async fn stop_health_checks(&mut self) {
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
            info!("Plugin health checks stopped");
        }
    }

    // 私有辅助方法

    async fn validate_dependencies(&self, metadata: &PluginMetadata) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        for dependency in &metadata.dependencies {
            if !dependency.optional && !plugins.contains_key(&dependency.plugin_id) {
                return Err(MosesQuantError::DependencyNotFound { 
                    dependency: dependency.plugin_id.clone() 
                });
            }
        }
        
        Ok(())
    }

    async fn update_dependency_graph(&self, metadata: &PluginMetadata) {
        let mut graph = self.dependency_graph.write().await;
        
        let dependencies: Vec<PluginId> = metadata.dependencies
            .iter()
            .filter(|dep| !dep.optional)
            .map(|dep| dep.plugin_id.clone())
            .collect();
        
        graph.insert(metadata.id.clone(), dependencies);
    }

    async fn check_dependencies_started(&self, plugin_id: &PluginId) -> Result<()> {
        let dependencies = {
            let graph = self.dependency_graph.read().await;
            graph.get(plugin_id).cloned().unwrap_or_default()
        };

        let states = self.states.read().await;
        
        for dep_id in dependencies {
            if let Some(dep_state) = states.get(&dep_id) {
                if dep_state.current_state != PluginState::Running {
                    return Err(MosesQuantError::DependencyNotFound {
                        dependency: format!("Plugin '{}' dependency '{}' is not running", plugin_id, dep_id)
                    });
                }
            }
        }
        
        Ok(())
    }

    async fn calculate_startup_order(&self) -> Result<Vec<Vec<PluginId>>> {
        let graph = self.dependency_graph.read().await;
        let plugins: Vec<PluginId> = graph.keys().cloned().collect();
        
        // 使用拓扑排序计算启动顺序
        let mut in_degree: HashMap<PluginId, usize> = HashMap::new();
        let mut adj_list: HashMap<PluginId, Vec<PluginId>> = HashMap::new();
        
        // 初始化
        for plugin_id in &plugins {
            in_degree.insert(plugin_id.clone(), 0);
            adj_list.insert(plugin_id.clone(), Vec::new());
        }
        
        // 构建邻接表和入度计算
        for (plugin_id, dependencies) in graph.iter() {
            for dep_id in dependencies {
                adj_list.get_mut(dep_id).unwrap().push(plugin_id.clone());
                *in_degree.get_mut(plugin_id).unwrap() += 1;
            }
        }
        
        let mut result = Vec::new();
        let mut queue: VecDeque<PluginId> = VecDeque::new();
        
        // 找到所有入度为0的节点
        for (plugin_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(plugin_id.clone());
            }
        }
        
        while !queue.is_empty() {
            let mut current_batch = Vec::new();
            let batch_size = queue.len();
            
            // 处理当前批次
            for _ in 0..batch_size {
                if let Some(plugin_id) = queue.pop_front() {
                    current_batch.push(plugin_id.clone());
                    
                    // 更新邻接节点的入度
                    for neighbor in &adj_list[&plugin_id] {
                        let degree = in_degree.get_mut(neighbor).unwrap();
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
            
            result.push(current_batch);
        }
        
        // 检查是否存在循环依赖
        let total_processed: usize = result.iter().map(|batch| batch.len()).sum();
        if total_processed != plugins.len() {
            return Err(MosesQuantError::CircularDependency);
        }
        
        Ok(result)
    }

    async fn transition_plugin_state(&self, plugin_id: &PluginId, new_state: PluginState) -> Result<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(plugin_id) {
            state.current_state = new_state;
            state.state_changed_at = Instant::now();
            Ok(())
        } else {
            Err(MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })
        }
    }

    async fn handle_plugin_error(&self, plugin_id: &PluginId, error: &MosesQuantError) {
        {
            let mut states = self.states.write().await;
            if let Some(state) = states.get_mut(plugin_id) {
                state.current_state = PluginState::Error;
                state.last_error = Some(error.to_string());
                state.retry_count += 1;
            }
        }

        self.publish_lifecycle_event(
            plugin_id.clone(),
            LifecycleEventType::Error,
            [("error".to_string(), serde_json::json!(error.to_string()))]
                .iter().cloned().collect(),
        ).await;

        error!("Plugin '{}' encountered error: {:?}", plugin_id, error);
    }

    async fn publish_lifecycle_event(
        &self,
        plugin_id: PluginId,
        event_type: LifecycleEventType,
        details: HashMap<String, serde_json::Value>,
    ) {
        if let Some(ref event_bus) = self.event_bus {
            let event_data = serde_json::json!({
                "plugin_id": plugin_id,
                "event_type": format!("{:?}", event_type),
                "timestamp": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                "details": details
            });

            let _ = event_bus.publish("plugin_lifecycle", &event_data.to_string()).await;
        }
    }
}

impl Clone for PluginLifecycleManager {
    fn clone(&self) -> Self {
        Self {
            plugins: self.plugins.clone(),
            contexts: self.contexts.clone(),
            states: self.states.clone(),
            dependency_graph: self.dependency_graph.clone(),
            config: self.config.clone(),
            event_bus: self.event_bus.clone(),
            health_check_handle: None, // 不克隆任务句柄
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug)]
    struct TestPlugin {
        metadata: PluginMetadata,
        state: PluginState,
        initialized: Arc<AtomicBool>,
        started: Arc<AtomicBool>,
    }

    impl TestPlugin {
        fn new(id: &str) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: id.to_string(),
                    name: format!("Test Plugin {}", id),
                    version: semver::Version::new(1, 0, 0),
                    description: "Test plugin".to_string(),
                    author: "Test".to_string(),
                    plugin_type: PluginType::Utility,
                    capabilities: vec![],
                    dependencies: vec![],
                    min_framework_version: semver::Version::new(2, 0, 0),
                    max_framework_version: None,
                    config_schema: None,
                    tags: vec![],
                },
                state: PluginState::Uninitialized,
                initialized: Arc::new(AtomicBool::new(false)),
                started: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_dependency(mut self, dep_id: &str) -> Self {
            self.metadata.dependencies.push(PluginDependency {
                plugin_id: dep_id.to_string(),
                version_req: "^1.0".to_string(),
                optional: false,
            });
            self
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _context: &PluginContext) -> Result<()> {
            self.initialized.store(true, Ordering::Relaxed);
            self.state = PluginState::Stopped;
            Ok(())
        }

        async fn start(&mut self, _context: &PluginContext) -> Result<()> {
            self.started.store(true, Ordering::Relaxed);
            self.state = PluginState::Running;
            Ok(())
        }

        async fn stop(&mut self, _context: &PluginContext) -> Result<()> {
            self.started.store(false, Ordering::Relaxed);
            self.state = PluginState::Stopped;
            Ok(())
        }

        fn state(&self) -> PluginState {
            self.state
        }

        async fn health_check(&self) -> Result<PluginHealthStatus> {
            Ok(PluginHealthStatus {
                healthy: true,
                message: "Plugin is healthy".to_string(),
                last_check: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                details: HashMap::new(),
            })
        }

        async fn get_metrics(&self) -> Result<PluginMetrics> {
            Ok(PluginMetrics::default())
        }

        async fn configure(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<()> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_registration_and_lifecycle() {
        let config = LifecycleManagerConfig::default();
        let manager = PluginLifecycleManager::new(config, None);

        let plugin = TestPlugin::new("test_plugin");
        let context = PluginContext::new("test_plugin".to_string());

        // 注册插件
        manager.register_plugin(
            Arc::new(Mutex::new(plugin)),
            context
        ).await.unwrap();

        // 检查状态
        let state = manager.get_plugin_state("test_plugin").await.unwrap();
        assert_eq!(state.current_state, PluginState::Uninitialized);

        // 初始化插件
        manager.initialize_plugin("test_plugin").await.unwrap();
        let state = manager.get_plugin_state("test_plugin").await.unwrap();
        assert_eq!(state.current_state, PluginState::Stopped);

        // 启动插件
        manager.start_plugin("test_plugin").await.unwrap();
        let state = manager.get_plugin_state("test_plugin").await.unwrap();
        assert_eq!(state.current_state, PluginState::Running);

        // 停止插件
        manager.stop_plugin("test_plugin").await.unwrap();
        let state = manager.get_plugin_state("test_plugin").await.unwrap();
        assert_eq!(state.current_state, PluginState::Stopped);
    }

    #[tokio::test]
    async fn test_dependency_management() {
        let config = LifecycleManagerConfig::default();
        let manager = PluginLifecycleManager::new(config, None);

        // 注册依赖插件
        let dep_plugin = TestPlugin::new("dependency");
        let dep_context = PluginContext::new("dependency".to_string());
        manager.register_plugin(
            Arc::new(Mutex::new(dep_plugin)),
            dep_context
        ).await.unwrap();

        // 注册主插件（依赖于dependency）
        let main_plugin = TestPlugin::new("main").with_dependency("dependency");
        let main_context = PluginContext::new("main".to_string());
        manager.register_plugin(
            Arc::new(Mutex::new(main_plugin)),
            main_context
        ).await.unwrap();

        // 计算启动顺序
        let startup_order = manager.calculate_startup_order().await.unwrap();
        
        // dependency应该在第一批，main在第二批
        assert_eq!(startup_order.len(), 2);
        assert!(startup_order[0].contains(&"dependency".to_string()));
        assert!(startup_order[1].contains(&"main".to_string()));
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let config = LifecycleManagerConfig::default();
        let manager = PluginLifecycleManager::new(config, None);

        // 创建循环依赖：A -> B -> A
        let plugin_a = TestPlugin::new("plugin_a").with_dependency("plugin_b");
        let plugin_b = TestPlugin::new("plugin_b").with_dependency("plugin_a");

        let context_a = PluginContext::new("plugin_a".to_string());
        let context_b = PluginContext::new("plugin_b".to_string());

        manager.register_plugin(
            Arc::new(Mutex::new(plugin_a)),
            context_a
        ).await.unwrap();

        manager.register_plugin(
            Arc::new(Mutex::new(plugin_b)),
            context_b
        ).await.unwrap();

        // 应该检测到循环依赖
        let result = manager.calculate_startup_order().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MosesQuantError::CircularDependency));
    }
}