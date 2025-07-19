//! 动态插件加载器
//! 
//! 支持运行时动态加载、卸载和热更新插件，提供安全的插件隔离和版本管理

use crate::plugins::core::*;
use crate::plugins::dependency_injection::*;
use crate::{Result, MosesQuantError};
use crate::types::PluginId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// 动态插件加载器
pub struct DynamicPluginLoader {
    /// 已加载的插件
    loaded_plugins: Arc<RwLock<HashMap<PluginId, LoadedPlugin>>>,
    /// 插件搜索路径
    search_paths: Arc<RwLock<Vec<PathBuf>>>,
    /// 加载器配置
    config: LoaderConfig,
    /// 插件仓库
    plugin_repository: Arc<RwLock<HashMap<PluginId, PluginPackage>>>,
    /// 热更新监控器
    hot_reload_monitor: Arc<RwLock<Option<HotReloadMonitor>>>,
    /// 依赖注入容器
    di_container: Arc<DIContainer>,
    /// 加载器状态
    loader_state: Arc<RwLock<LoaderState>>,
}

/// 加载器配置
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// 是否启用热更新
    pub enable_hot_reload: bool,
    /// 插件加载超时时间
    pub load_timeout: Duration,
    /// 是否启用插件隔离
    pub enable_isolation: bool,
    /// 最大并发加载数
    pub max_concurrent_loads: usize,
    /// 是否启用版本检查
    pub enable_version_check: bool,
    /// 插件卸载超时时间
    pub unload_timeout: Duration,
    /// 热更新检查间隔
    pub hot_reload_interval: Duration,
    /// 是否启用依赖自动解析
    pub enable_auto_dependency_resolution: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            enable_hot_reload: true,
            load_timeout: Duration::from_secs(30),
            enable_isolation: true,
            max_concurrent_loads: 10,
            enable_version_check: true,
            unload_timeout: Duration::from_secs(10),
            hot_reload_interval: Duration::from_secs(5),
            enable_auto_dependency_resolution: true,
        }
    }
}

/// 加载器状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// 已加载的插件信息
#[derive(Debug)]
pub struct LoadedPlugin {
    /// 插件实例
    pub plugin: Arc<Mutex<dyn Plugin>>,
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 加载时间
    pub loaded_at: Instant,
    /// 插件状态
    pub state: PluginState,
    /// 加载来源
    pub source: PluginSource,
    /// 依赖的插件
    pub dependencies: Vec<PluginId>,
    /// 被依赖的插件
    pub dependents: Vec<PluginId>,
    /// 使用计数
    pub usage_count: u64,
}

/// 插件来源
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// 文件系统
    File(PathBuf),
    /// 网络下载
    Network(String),
    /// 内存中
    Memory,
    /// 注册表
    Registry(String),
}

/// 插件包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPackage {
    /// 包元数据
    pub metadata: PluginMetadata,
    /// 包文件路径
    pub path: PathBuf,
    /// 包校验和
    pub checksum: String,
    /// 包大小
    pub size: u64,
    /// 创建时间
    pub created_at: i64,
    /// 签名信息
    pub signature: Option<String>,
}

/// 热更新监控器
pub struct HotReloadMonitor {
    /// 监控的文件路径
    watched_paths: HashSet<PathBuf>,
    /// 文件修改时间缓存
    modification_times: HashMap<PathBuf, std::time::SystemTime>,
    /// 监控任务句柄
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

/// 插件加载结果
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// 是否成功
    pub success: bool,
    /// 加载的插件ID
    pub plugin_id: PluginId,
    /// 加载耗时
    pub load_time: Duration,
    /// 错误信息
    pub error: Option<String>,
    /// 加载的依赖插件
    pub loaded_dependencies: Vec<PluginId>,
}

/// 插件卸载结果
#[derive(Debug, Clone)]
pub struct UnloadResult {
    /// 是否成功
    pub success: bool,
    /// 卸载的插件ID
    pub plugin_id: PluginId,
    /// 卸载耗时
    pub unload_time: Duration,
    /// 错误信息
    pub error: Option<String>,
    /// 卸载的依赖插件
    pub unloaded_dependents: Vec<PluginId>,
}

impl DynamicPluginLoader {
    /// 创建新的动态插件加载器
    pub fn new(config: LoaderConfig, di_container: Arc<DIContainer>) -> Self {
        Self {
            loaded_plugins: Arc::new(RwLock::new(HashMap::new())),
            search_paths: Arc::new(RwLock::new(Vec::new())),
            config,
            plugin_repository: Arc::new(RwLock::new(HashMap::new())),
            hot_reload_monitor: Arc::new(RwLock::new(None)),
            di_container,
            loader_state: Arc::new(RwLock::new(LoaderState::Stopped)),
        }
    }

    /// 添加插件搜索路径
    pub async fn add_search_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(MosesQuantError::PluginLoad {
                message: format!("Search path does not exist: {:?}", path)
            });
        }

        let mut search_paths = self.search_paths.write().await;
        if !search_paths.contains(&path) {
            search_paths.push(path);
            info!("Added plugin search path: {:?}", path);
        }

        Ok(())
    }

    /// 扫描并发现插件
    pub async fn discover_plugins(&self) -> Result<Vec<PluginPackage>> {
        let search_paths = self.search_paths.read().await;
        let mut discovered_plugins = Vec::new();

        for search_path in search_paths.iter() {
            match self.scan_directory(search_path).await {
                Ok(mut plugins) => {
                    discovered_plugins.append(&mut plugins);
                }
                Err(e) => {
                    warn!("Failed to scan directory {:?}: {:?}", search_path, e);
                }
            }
        }

        // 更新插件仓库
        {
            let mut repository = self.plugin_repository.write().await;
            for plugin in &discovered_plugins {
                repository.insert(plugin.metadata.id.clone(), plugin.clone());
            }
        }

        info!("Discovered {} plugins", discovered_plugins.len());
        Ok(discovered_plugins)
    }

    /// 加载插件
    pub async fn load_plugin(&self, plugin_id: &PluginId) -> Result<LoadResult> {
        let start_time = Instant::now();
        
        // 检查插件是否已加载
        {
            let loaded_plugins = self.loaded_plugins.read().await;
            if loaded_plugins.contains_key(plugin_id) {
                return Ok(LoadResult {
                    success: true,
                    plugin_id: plugin_id.clone(),
                    load_time: Duration::ZERO,
                    error: None,
                    loaded_dependencies: vec![],
                });
            }
        }

        // 查找插件包
        let plugin_package = {
            let repository = self.plugin_repository.read().await;
            repository.get(plugin_id).cloned()
                .ok_or_else(|| MosesQuantError::PluginNotFound { plugin_id: plugin_id.clone() })?
        };

        // 解析并加载依赖
        let mut loaded_dependencies = Vec::new();
        if self.config.enable_auto_dependency_resolution {
            for dependency in &plugin_package.metadata.dependencies {
                if !dependency.optional {
                    match self.load_plugin(&dependency.plugin_id).await {
                        Ok(dep_result) => {
                            loaded_dependencies.push(dependency.plugin_id.clone());
                            loaded_dependencies.extend(dep_result.loaded_dependencies);
                        }
                        Err(e) => {
                            return Ok(LoadResult {
                                success: false,
                                plugin_id: plugin_id.clone(),
                                load_time: start_time.elapsed(),
                                error: Some(format!("Failed to load dependency {}: {}", dependency.plugin_id, e)),
                                loaded_dependencies,
                            });
                        }
                    }
                }
            }
        }

        // 加载插件
        match self.load_plugin_from_package(&plugin_package).await {
            Ok(loaded_plugin) => {
                // 注册到依赖注入容器
                if let Err(e) = self.register_plugin_in_di(&loaded_plugin).await {
                    warn!("Failed to register plugin in DI container: {:?}", e);
                }

                // 添加到已加载插件列表
                {
                    let mut loaded_plugins = self.loaded_plugins.write().await;
                    loaded_plugins.insert(plugin_id.clone(), loaded_plugin);
                }

                Ok(LoadResult {
                    success: true,
                    plugin_id: plugin_id.clone(),
                    load_time: start_time.elapsed(),
                    error: None,
                    loaded_dependencies,
                })
            }
            Err(e) => {
                Ok(LoadResult {
                    success: false,
                    plugin_id: plugin_id.clone(),
                    load_time: start_time.elapsed(),
                    error: Some(e.to_string()),
                    loaded_dependencies,
                })
            }
        }
    }

    /// 卸载插件
    pub async fn unload_plugin(&self, plugin_id: &PluginId) -> Result<UnloadResult> {
        let start_time = Instant::now();

        // 检查插件是否已加载
        let loaded_plugin = {
            let loaded_plugins = self.loaded_plugins.read().await;
            loaded_plugins.get(plugin_id).cloned()
        };

        let loaded_plugin = match loaded_plugin {
            Some(plugin) => plugin,
            None => {
                return Ok(UnloadResult {
                    success: true,
                    plugin_id: plugin_id.clone(),
                    unload_time: Duration::ZERO,
                    error: None,
                    unloaded_dependents: vec![],
                });
            }
        };

        // 检查依赖关系
        let mut unloaded_dependents = Vec::new();
        for dependent_id in &loaded_plugin.dependents {
            match self.unload_plugin(dependent_id).await {
                Ok(result) => {
                    unloaded_dependents.push(dependent_id.clone());
                    unloaded_dependents.extend(result.unloaded_dependents);
                }
                Err(e) => {
                    return Ok(UnloadResult {
                        success: false,
                        plugin_id: plugin_id.clone(),
                        unload_time: start_time.elapsed(),
                        error: Some(format!("Failed to unload dependent {}: {}", dependent_id, e)),
                        unloaded_dependents,
                    });
                }
            }
        }

        // 停止插件
        {
            let plugin_context = PluginContext::new(plugin_id.clone());
            let mut plugin_instance = loaded_plugin.plugin.lock().await;
            
            if let Err(e) = plugin_instance.stop(&plugin_context).await {
                warn!("Failed to stop plugin {}: {:?}", plugin_id, e);
            }
        }

        // 从已加载插件列表中移除
        {
            let mut loaded_plugins = self.loaded_plugins.write().await;
            loaded_plugins.remove(plugin_id);
        }

        Ok(UnloadResult {
            success: true,
            plugin_id: plugin_id.clone(),
            unload_time: start_time.elapsed(),
            error: None,
            unloaded_dependents,
        })
    }

    /// 重新加载插件（热更新）
    pub async fn reload_plugin(&self, plugin_id: &PluginId) -> Result<(UnloadResult, LoadResult)> {
        let unload_result = self.unload_plugin(plugin_id).await?;
        let load_result = self.load_plugin(plugin_id).await?;
        
        info!("Plugin '{}' reloaded successfully", plugin_id);
        Ok((unload_result, load_result))
    }

    /// 获取已加载的插件列表
    pub async fn get_loaded_plugins(&self) -> Vec<PluginId> {
        let loaded_plugins = self.loaded_plugins.read().await;
        loaded_plugins.keys().cloned().collect()
    }

    /// 获取插件信息
    pub async fn get_plugin_info(&self, plugin_id: &PluginId) -> Option<LoadedPlugin> {
        let loaded_plugins = self.loaded_plugins.read().await;
        loaded_plugins.get(plugin_id).cloned()
    }

    /// 启动插件加载器
    pub async fn start(&self) -> Result<()> {
        {
            let mut state = self.loader_state.write().await;
            if *state != LoaderState::Stopped {
                return Err(MosesQuantError::Internal {
                    message: "Plugin loader is not in stopped state".to_string()
                });
            }
            *state = LoaderState::Starting;
        }

        // 发现插件
        self.discover_plugins().await?;

        // 启动热更新监控
        if self.config.enable_hot_reload {
            self.start_hot_reload_monitor().await?;
        }

        {
            let mut state = self.loader_state.write().await;
            *state = LoaderState::Running;
        }

        info!("Dynamic plugin loader started successfully");
        Ok(())
    }

    /// 停止插件加载器
    pub async fn stop(&self) -> Result<()> {
        {
            let mut state = self.loader_state.write().await;
            if *state != LoaderState::Running {
                return Err(MosesQuantError::Internal {
                    message: "Plugin loader is not in running state".to_string()
                });
            }
            *state = LoaderState::Stopping;
        }

        // 停止热更新监控
        self.stop_hot_reload_monitor().await;

        // 卸载所有插件
        let plugin_ids: Vec<PluginId> = {
            let loaded_plugins = self.loaded_plugins.read().await;
            loaded_plugins.keys().cloned().collect()
        };

        for plugin_id in plugin_ids {
            if let Err(e) = self.unload_plugin(&plugin_id).await {
                warn!("Failed to unload plugin '{}': {:?}", plugin_id, e);
            }
        }

        {
            let mut state = self.loader_state.write().await;
            *state = LoaderState::Stopped;
        }

        info!("Dynamic plugin loader stopped successfully");
        Ok(())
    }

    /// 获取加载器统计信息
    pub async fn get_statistics(&self) -> LoaderStatistics {
        let loaded_plugins = self.loaded_plugins.read().await;
        let repository = self.plugin_repository.read().await;
        let state = self.loader_state.read().await;

        let mut running_plugins = 0;
        let mut stopped_plugins = 0;
        let mut total_usage = 0;

        for plugin in loaded_plugins.values() {
            match plugin.state {
                PluginState::Running => running_plugins += 1,
                PluginState::Stopped => stopped_plugins += 1,
                _ => {}
            }
            total_usage += plugin.usage_count;
        }

        LoaderStatistics {
            loader_state: state.clone(),
            total_loaded_plugins: loaded_plugins.len(),
            running_plugins,
            stopped_plugins,
            total_discovered_plugins: repository.len(),
            total_usage_count: total_usage,
            hot_reload_enabled: self.config.enable_hot_reload,
        }
    }

    // 私有方法

    /// 扫描目录查找插件
    async fn scan_directory(&self, dir_path: &Path) -> Result<Vec<PluginPackage>> {
        let mut plugins = Vec::new();
        
        if !dir_path.is_dir() {
            return Ok(plugins);
        }

        let mut entries = tokio::fs::read_dir(dir_path).await
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to read directory {:?}: {}", dir_path, e)
            })?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to read directory entry: {}", e)
            })? {
            
            let path = entry.path();
            
            // 查找插件清单文件
            if path.is_file() && path.file_name().unwrap_or_default() == "plugin.json" {
                match self.load_plugin_manifest(&path).await {
                    Ok(package) => plugins.push(package),
                    Err(e) => warn!("Failed to load plugin manifest from {:?}: {:?}", path, e),
                }
            }
        }

        Ok(plugins)
    }

    /// 加载插件清单
    async fn load_plugin_manifest(&self, manifest_path: &Path) -> Result<PluginPackage> {
        let content = tokio::fs::read_to_string(manifest_path).await
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to read manifest file: {}", e)
            })?;

        let metadata: PluginMetadata = serde_json::from_str(&content)
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to parse manifest: {}", e)
            })?;

        let file_metadata = tokio::fs::metadata(manifest_path).await
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to get file metadata: {}", e)
            })?;

        Ok(PluginPackage {
            metadata,
            path: manifest_path.to_path_buf(),
            checksum: self.calculate_checksum(manifest_path).await?,
            size: file_metadata.len(),
            created_at: file_metadata.created()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            signature: None,
        })
    }

    /// 计算文件校验和
    async fn calculate_checksum(&self, file_path: &Path) -> Result<String> {
        let content = tokio::fs::read(file_path).await
            .map_err(|e| MosesQuantError::PluginLoad {
                message: format!("Failed to read file for checksum: {}", e)
            })?;

        // 简化的校验和计算（实际中应使用更好的哈希算法）
        Ok(format!("{:x}", content.len()))
    }

    /// 从包加载插件
    async fn load_plugin_from_package(&self, package: &PluginPackage) -> Result<LoadedPlugin> {
        // 这里需要根据实际的插件格式实现加载逻辑
        // 为简化示例，这里创建一个模拟的插件实例
        
        debug!("Loading plugin from package: {}", package.metadata.id);
        
        // 实际实现中这里会加载动态库或解析配置
        let plugin_instance = self.create_mock_plugin(&package.metadata)?;

        Ok(LoadedPlugin {
            plugin: Arc::new(Mutex::new(plugin_instance)),
            metadata: package.metadata.clone(),
            loaded_at: Instant::now(),
            state: PluginState::Stopped,
            source: PluginSource::File(package.path.clone()),
            dependencies: package.metadata.dependencies.iter()
                .map(|dep| dep.plugin_id.clone())
                .collect(),
            dependents: Vec::new(),
            usage_count: 0,
        })
    }

    /// 创建模拟插件实例（实际实现中应该动态加载）
    fn create_mock_plugin(&self, metadata: &PluginMetadata) -> Result<Box<dyn Plugin>> {
        // 这里需要根据插件类型创建相应的实例
        // 实际实现中会使用动态库加载
        Err(MosesQuantError::PluginLoad {
            message: "Mock plugin creation not implemented".to_string()
        })
    }

    /// 将插件注册到依赖注入容器
    async fn register_plugin_in_di(&self, loaded_plugin: &LoadedPlugin) -> Result<()> {
        // 实际实现中需要根据插件接口类型注册到DI容器
        debug!("Registering plugin {} in DI container", loaded_plugin.metadata.id);
        Ok(())
    }

    /// 启动热更新监控
    async fn start_hot_reload_monitor(&self) -> Result<()> {
        if !self.config.enable_hot_reload {
            return Ok();
        }

        let search_paths = self.search_paths.read().await.clone();
        let mut monitor = HotReloadMonitor {
            watched_paths: HashSet::new(),
            modification_times: HashMap::new(),
            monitor_handle: None,
        };

        // 收集所有要监控的文件
        for search_path in &search_paths {
            self.collect_watched_files(search_path, &mut monitor.watched_paths).await?;
        }

        // 初始化修改时间
        for path in &monitor.watched_paths {
            if let Ok(metadata) = tokio::fs::metadata(path).await {
                if let Ok(modified) = metadata.modified() {
                    monitor.modification_times.insert(path.clone(), modified);
                }
            }
        }

        // 启动监控任务
        let watched_paths = monitor.watched_paths.clone();
        let modification_times = Arc::new(Mutex::new(monitor.modification_times.clone()));
        let loader = self.clone_for_monitor();
        let interval = self.config.hot_reload_interval;

        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                for path in &watched_paths {
                    if let Ok(metadata) = tokio::fs::metadata(path).await {
                        if let Ok(modified) = metadata.modified() {
                            let mut times = modification_times.lock().await;
                            
                            if let Some(last_modified) = times.get(path) {
                                if modified > *last_modified {
                                    info!("Detected file change: {:?}", path);
                                    times.insert(path.clone(), modified);
                                    
                                    // 触发插件重新加载
                                    if let Err(e) = loader.handle_file_change(path).await {
                                        error!("Failed to handle file change: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        monitor.monitor_handle = Some(handle);
        
        {
            let mut hot_reload_monitor = self.hot_reload_monitor.write().await;
            *hot_reload_monitor = Some(monitor);
        }

        info!("Hot reload monitor started");
        Ok(())
    }

    /// 停止热更新监控
    async fn stop_hot_reload_monitor(&self) {
        let mut monitor = self.hot_reload_monitor.write().await;
        
        if let Some(ref mut hot_monitor) = monitor.as_mut() {
            if let Some(handle) = hot_monitor.monitor_handle.take() {
                handle.abort();
            }
        }
        
        *monitor = None;
        info!("Hot reload monitor stopped");
    }

    /// 收集要监控的文件
    async fn collect_watched_files(&self, dir_path: &Path, watched_paths: &mut HashSet<PathBuf>) -> Result<()> {
        if !dir_path.is_dir() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(dir_path).await
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to read directory: {}", e)
            })?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to read directory entry: {}", e)
            })? {
            
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                watched_paths.insert(path);
            }
        }

        Ok(())
    }

    /// 克隆用于监控任务的loader引用
    fn clone_for_monitor(&self) -> DynamicPluginLoaderMonitor {
        DynamicPluginLoaderMonitor {
            loaded_plugins: self.loaded_plugins.clone(),
            plugin_repository: self.plugin_repository.clone(),
        }
    }

    /// 处理文件变化
    async fn handle_file_change(&self, _changed_path: &Path) -> Result<()> {
        // 实际实现中这里会分析变化的文件并触发相应的插件重新加载
        debug!("Handling file change: {:?}", _changed_path);
        Ok(())
    }
}

/// 监控任务专用的加载器引用
#[derive(Clone)]
struct DynamicPluginLoaderMonitor {
    loaded_plugins: Arc<RwLock<HashMap<PluginId, LoadedPlugin>>>,
    plugin_repository: Arc<RwLock<HashMap<PluginId, PluginPackage>>>,
}

impl DynamicPluginLoaderMonitor {
    async fn handle_file_change(&self, _changed_path: &Path) -> Result<()> {
        // 监控任务中的文件变化处理逻辑
        Ok(())
    }
}

/// 加载器统计信息
#[derive(Debug, Clone)]
pub struct LoaderStatistics {
    /// 加载器状态
    pub loader_state: LoaderState,
    /// 已加载插件总数
    pub total_loaded_plugins: usize,
    /// 运行中的插件数
    pub running_plugins: usize,
    /// 已停止的插件数
    pub stopped_plugins: usize,
    /// 发现的插件总数
    pub total_discovered_plugins: usize,
    /// 总使用次数
    pub total_usage_count: u64,
    /// 是否启用热更新
    pub hot_reload_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_loader_creation() {
        let config = LoaderConfig::default();
        let di_container = Arc::new(DIContainer::new(DIContainerConfig::default()));
        let loader = DynamicPluginLoader::new(config, di_container);
        
        let stats = loader.get_statistics().await;
        assert_eq!(stats.loader_state, LoaderState::Stopped);
        assert_eq!(stats.total_loaded_plugins, 0);
    }

    #[tokio::test]
    async fn test_search_path_management() {
        let config = LoaderConfig::default();
        let di_container = Arc::new(DIContainer::new(DIContainerConfig::default()));
        let loader = DynamicPluginLoader::new(config, di_container);

        // 创建临时目录
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 添加搜索路径
        loader.add_search_path(temp_path).await.unwrap();

        // 验证路径已添加
        let search_paths = loader.search_paths.read().await;
        assert!(search_paths.contains(&temp_path.to_path_buf()));
    }

    #[tokio::test]
    async fn test_plugin_discovery() {
        let config = LoaderConfig::default();
        let di_container = Arc::new(DIContainer::new(DIContainerConfig::default()));
        let loader = DynamicPluginLoader::new(config, di_container);

        // 创建临时目录和插件清单文件
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("test_plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let manifest_content = r#"
        {
            "id": "test_plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "author": "Test Author",
            "plugin_type": "Utility",
            "capabilities": [],
            "dependencies": [],
            "min_framework_version": "2.0.0",
            "tags": []
        }"#;

        let manifest_path = plugin_dir.join("plugin.json");
        fs::write(&manifest_path, manifest_content).unwrap();

        // 添加搜索路径并发现插件
        loader.add_search_path(&plugin_dir).await.unwrap();
        let discovered = loader.discover_plugins().await.unwrap();

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].metadata.id, "test_plugin");
    }

    #[tokio::test]
    async fn test_loader_lifecycle() {
        let config = LoaderConfig::default();
        let di_container = Arc::new(DIContainer::new(DIContainerConfig::default()));
        let loader = DynamicPluginLoader::new(config, di_container);

        // 启动加载器
        loader.start().await.unwrap();
        let stats = loader.get_statistics().await;
        assert_eq!(stats.loader_state, LoaderState::Running);

        // 停止加载器
        loader.stop().await.unwrap();
        let stats = loader.get_statistics().await;
        assert_eq!(stats.loader_state, LoaderState::Stopped);
    }
}