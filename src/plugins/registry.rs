//! 插件注册表和发现系统
//! 
//! 提供插件的注册、发现、版本管理和依赖解析功能

use super::core::*;
use crate::{Result, MosesQuantError};
use crate::types::PluginId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use semver::{Version, VersionReq};
use std::path::{Path, PathBuf};
use tracing::{info, warn, error, debug};

/// 插件注册表
pub struct PluginRegistry {
    /// 已注册的插件元数据
    plugins: Arc<RwLock<HashMap<PluginId, RegisteredPlugin>>>,
    /// 插件类型索引
    type_index: Arc<RwLock<HashMap<PluginType, Vec<PluginId>>>>,
    /// 能力索引
    capability_index: Arc<RwLock<HashMap<PluginCapability, Vec<PluginId>>>>,
    /// 插件搜索路径
    search_paths: Arc<RwLock<Vec<PathBuf>>>,
    /// 注册表配置
    config: RegistryConfig,
}

/// 已注册的插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPlugin {
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 注册时间
    pub registered_at: crate::TimestampNs,
    /// 插件状态
    pub status: PluginRegistrationStatus,
    /// 插件位置
    pub location: PluginLocation,
    /// 验证信息
    pub verification: PluginVerification,
    /// 使用统计
    pub usage_stats: PluginUsageStats,
}

/// 插件注册状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginRegistrationStatus {
    /// 已注册
    Registered,
    /// 已验证
    Verified,
    /// 已禁用
    Disabled,
    /// 已弃用
    Deprecated,
    /// 有问题
    Problematic,
}

/// 插件位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginLocation {
    /// 本地文件路径
    LocalPath(PathBuf),
    /// 远程URL
    RemoteUrl(String),
    /// 内嵌插件
    Embedded,
    /// 动态库
    DynamicLibrary(PathBuf),
}

/// 插件验证信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVerification {
    /// 是否已验证
    pub verified: bool,
    /// 数字签名
    pub signature: Option<String>,
    /// 校验和
    pub checksum: Option<String>,
    /// 验证时间
    pub verified_at: Option<crate::TimestampNs>,
    /// 验证者
    pub verified_by: Option<String>,
    /// 信任级别
    pub trust_level: TrustLevel,
}

/// 信任级别
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// 不可信
    Untrusted,
    /// 低信任
    Low,
    /// 中等信任
    Medium,
    /// 高信任
    High,
    /// 完全信任
    Trusted,
}

/// 插件使用统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginUsageStats {
    /// 安装次数
    pub install_count: u64,
    /// 启动次数
    pub launch_count: u64,
    /// 总运行时间（纳秒）
    pub total_runtime_ns: i64,
    /// 崩溃次数
    pub crash_count: u64,
    /// 最后使用时间
    pub last_used_at: Option<crate::TimestampNs>,
    /// 平均性能评分
    pub performance_score: f64,
    /// 用户评分
    pub user_rating: Option<f64>,
}

/// 注册表配置
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// 是否启用自动发现
    pub auto_discovery: bool,
    /// 发现间隔
    pub discovery_interval: std::time::Duration,
    /// 最大插件数量
    pub max_plugins: usize,
    /// 是否允许重复注册
    pub allow_duplicates: bool,
    /// 是否启用验证
    pub enable_verification: bool,
    /// 默认信任级别
    pub default_trust_level: TrustLevel,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            auto_discovery: false,
            discovery_interval: std::time::Duration::from_secs(300),
            max_plugins: 1000,
            allow_duplicates: false,
            enable_verification: true,
            default_trust_level: TrustLevel::Low,
        }
    }
}

/// 插件查询条件
#[derive(Debug, Clone, Default)]
pub struct PluginQuery {
    /// 插件类型过滤
    pub plugin_type: Option<PluginType>,
    /// 能力要求
    pub required_capabilities: Vec<PluginCapability>,
    /// 版本要求
    pub version_req: Option<VersionReq>,
    /// 最小信任级别
    pub min_trust_level: Option<TrustLevel>,
    /// 状态过滤
    pub status_filter: Option<Vec<PluginRegistrationStatus>>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 作者过滤
    pub author: Option<String>,
}

/// 依赖解析结果
#[derive(Debug, Clone)]
pub struct DependencyResolution {
    /// 主插件
    pub target_plugin: PluginId,
    /// 解析后的依赖链
    pub dependency_chain: Vec<PluginId>,
    /// 缺失的依赖
    pub missing_dependencies: Vec<PluginDependency>,
    /// 版本冲突
    pub version_conflicts: Vec<VersionConflict>,
    /// 是否可解析
    pub resolvable: bool,
}

/// 版本冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// 插件ID
    pub plugin_id: PluginId,
    /// 要求的版本
    pub required_version: String,
    /// 可用的版本
    pub available_version: Version,
    /// 冲突来源
    pub conflicting_requester: PluginId,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            type_index: Arc::new(RwLock::new(HashMap::new())),
            capability_index: Arc::new(RwLock::new(HashMap::new())),
            search_paths: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// 注册插件
    pub async fn register_plugin(&self, metadata: PluginMetadata, location: PluginLocation) -> Result<()> {
        // 检查是否已达到最大插件数量
        {
            let plugins = self.plugins.read().await;
            if plugins.len() >= self.config.max_plugins {
                return Err(MosesQuantError::Internal {
                    message: "Maximum number of plugins reached".to_string()
                });
            }
        }

        // 检查是否重复注册
        if !self.config.allow_duplicates {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&metadata.id) {
                return Err(MosesQuantError::PluginAlreadyRegistered { 
                    plugin_id: metadata.id.clone() 
                });
            }
        }

        // 验证插件
        let verification = if self.config.enable_verification {
            self.verify_plugin(&metadata, &location).await?
        } else {
            PluginVerification {
                verified: false,
                signature: None,
                checksum: None,
                verified_at: None,
                verified_by: None,
                trust_level: self.config.default_trust_level.clone(),
            }
        };

        let registered_plugin = RegisteredPlugin {
            metadata: metadata.clone(),
            registered_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            status: PluginRegistrationStatus::Registered,
            location,
            verification,
            usage_stats: PluginUsageStats::default(),
        };

        // 注册插件
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(metadata.id.clone(), registered_plugin);
        }

        // 更新索引
        self.update_indexes(&metadata).await;

        info!("Plugin '{}' registered successfully", metadata.id);
        Ok(())
    }

    /// 注销插件
    pub async fn unregister_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        let metadata = {
            let mut plugins = self.plugins.write().await;
            if let Some(registered_plugin) = plugins.remove(plugin_id) {
                registered_plugin.metadata
            } else {
                return Err(MosesQuantError::PluginNotFound { 
                    plugin_id: plugin_id.clone() 
                });
            }
        };

        // 更新索引
        self.remove_from_indexes(&metadata).await;

        info!("Plugin '{}' unregistered successfully", plugin_id);
        Ok(())
    }

    /// 查找插件
    pub async fn find_plugins(&self, query: &PluginQuery) -> Result<Vec<RegisteredPlugin>> {
        let plugins = self.plugins.read().await;
        let mut results = Vec::new();

        for plugin in plugins.values() {
            if self.matches_query(plugin, query) {
                results.push(plugin.clone());
            }
        }

        // 按评分和信任级别排序
        results.sort_by(|a, b| {
            b.verification.trust_level.cmp(&a.verification.trust_level)
                .then_with(|| b.usage_stats.performance_score.partial_cmp(&a.usage_stats.performance_score).unwrap_or(std::cmp::Ordering::Equal))
        });

        Ok(results)
    }

    /// 按类型查找插件
    pub async fn find_by_type(&self, plugin_type: &PluginType) -> Result<Vec<RegisteredPlugin>> {
        let query = PluginQuery {
            plugin_type: Some(plugin_type.clone()),
            ..Default::default()
        };
        self.find_plugins(&query).await
    }

    /// 按能力查找插件
    pub async fn find_by_capability(&self, capability: &PluginCapability) -> Result<Vec<RegisteredPlugin>> {
        let query = PluginQuery {
            required_capabilities: vec![capability.clone()],
            ..Default::default()
        };
        self.find_plugins(&query).await
    }

    /// 获取插件详情
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Result<RegisteredPlugin> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id)
            .cloned()
            .ok_or_else(|| MosesQuantError::PluginNotFound { 
                plugin_id: plugin_id.clone() 
            })
    }

    /// 解析插件依赖
    pub async fn resolve_dependencies(&self, plugin_id: &PluginId) -> Result<DependencyResolution> {
        let plugin = self.get_plugin(plugin_id).await?;
        let mut dependency_chain = Vec::new();
        let mut missing_dependencies = Vec::new();
        let mut version_conflicts = Vec::new();
        let mut visited = HashSet::new();

        let resolvable = self.resolve_dependencies_recursive(
            &plugin.metadata,
            &mut dependency_chain,
            &mut missing_dependencies,
            &mut version_conflicts,
            &mut visited,
        ).await;

        Ok(DependencyResolution {
            target_plugin: plugin_id.clone(),
            dependency_chain,
            missing_dependencies,
            version_conflicts,
            resolvable,
        })
    }

    /// 获取插件统计信息
    pub async fn get_registry_stats(&self) -> RegistryStats {
        let plugins = self.plugins.read().await;
        
        let mut stats = RegistryStats {
            total_plugins: plugins.len(),
            by_type: HashMap::new(),
            by_status: HashMap::new(),
            by_trust_level: HashMap::new(),
            average_performance_score: 0.0,
        };

        let mut total_score = 0.0;
        let mut score_count = 0;

        for plugin in plugins.values() {
            // 按类型统计
            *stats.by_type.entry(plugin.metadata.plugin_type.clone()).or_insert(0) += 1;
            
            // 按状态统计
            *stats.by_status.entry(plugin.status.clone()).or_insert(0) += 1;
            
            // 按信任级别统计
            *stats.by_trust_level.entry(plugin.verification.trust_level.clone()).or_insert(0) += 1;
            
            // 计算平均性能评分
            total_score += plugin.usage_stats.performance_score;
            score_count += 1;
        }

        if score_count > 0 {
            stats.average_performance_score = total_score / score_count as f64;
        }

        stats
    }

    /// 添加搜索路径
    pub async fn add_search_path(&self, path: PathBuf) -> Result<()> {
        let mut search_paths = self.search_paths.write().await;
        if !search_paths.contains(&path) {
            search_paths.push(path.clone());
            info!("Added plugin search path: {:?}", path);
        }
        Ok(())
    }

    /// 自动发现插件
    pub async fn discover_plugins(&self) -> Result<Vec<PluginMetadata>> {
        let search_paths = self.search_paths.read().await.clone();
        let mut discovered = Vec::new();

        for path in search_paths {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = self.try_load_plugin_metadata(&entry.path()).await {
                        discovered.push(metadata);
                    }
                }
            }
        }

        info!("Discovered {} plugins", discovered.len());
        Ok(discovered)
    }

    /// 更新插件状态
    pub async fn update_plugin_status(&self, plugin_id: &PluginId, status: PluginRegistrationStatus) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.status = status;
            info!("Updated plugin '{}' status to {:?}", plugin_id, plugin.status);
            Ok(())
        } else {
            Err(MosesQuantError::PluginNotFound { 
                plugin_id: plugin_id.clone() 
            })
        }
    }

    /// 更新插件使用统计
    pub async fn update_usage_stats(&self, plugin_id: &PluginId, update: UsageStatsUpdate) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            match update {
                UsageStatsUpdate::Launch => {
                    plugin.usage_stats.launch_count += 1;
                    plugin.usage_stats.last_used_at = Some(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                }
                UsageStatsUpdate::Crash => {
                    plugin.usage_stats.crash_count += 1;
                }
                UsageStatsUpdate::Runtime(duration_ns) => {
                    plugin.usage_stats.total_runtime_ns += duration_ns;
                }
                UsageStatsUpdate::PerformanceScore(score) => {
                    plugin.usage_stats.performance_score = score;
                }
                UsageStatsUpdate::UserRating(rating) => {
                    plugin.usage_stats.user_rating = Some(rating);
                }
            }
            Ok(())
        } else {
            Err(MosesQuantError::PluginNotFound { 
                plugin_id: plugin_id.clone() 
            })
        }
    }

    // 私有辅助方法

    async fn verify_plugin(&self, _metadata: &PluginMetadata, _location: &PluginLocation) -> Result<PluginVerification> {
        // 简化的验证实现
        Ok(PluginVerification {
            verified: true,
            signature: None,
            checksum: None,
            verified_at: Some(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            verified_by: Some("system".to_string()),
            trust_level: self.config.default_trust_level.clone(),
        })
    }

    async fn update_indexes(&self, metadata: &PluginMetadata) {
        // 更新类型索引
        {
            let mut type_index = self.type_index.write().await;
            type_index.entry(metadata.plugin_type.clone())
                .or_insert_with(Vec::new)
                .push(metadata.id.clone());
        }

        // 更新能力索引
        {
            let mut capability_index = self.capability_index.write().await;
            for capability in &metadata.capabilities {
                capability_index.entry(capability.clone())
                    .or_insert_with(Vec::new)
                    .push(metadata.id.clone());
            }
        }
    }

    async fn remove_from_indexes(&self, metadata: &PluginMetadata) {
        // 从类型索引中移除
        {
            let mut type_index = self.type_index.write().await;
            if let Some(plugins) = type_index.get_mut(&metadata.plugin_type) {
                plugins.retain(|id| id != &metadata.id);
            }
        }

        // 从能力索引中移除
        {
            let mut capability_index = self.capability_index.write().await;
            for capability in &metadata.capabilities {
                if let Some(plugins) = capability_index.get_mut(capability) {
                    plugins.retain(|id| id != &metadata.id);
                }
            }
        }
    }

    fn matches_query(&self, plugin: &RegisteredPlugin, query: &PluginQuery) -> bool {
        // 类型过滤
        if let Some(ref required_type) = query.plugin_type {
            if &plugin.metadata.plugin_type != required_type {
                return false;
            }
        }

        // 能力要求
        for required_capability in &query.required_capabilities {
            if !plugin.metadata.capabilities.contains(required_capability) {
                return false;
            }
        }

        // 版本要求
        if let Some(ref version_req) = query.version_req {
            if !version_req.matches(&plugin.metadata.version) {
                return false;
            }
        }

        // 信任级别
        if let Some(ref min_trust_level) = query.min_trust_level {
            if plugin.verification.trust_level < *min_trust_level {
                return false;
            }
        }

        // 状态过滤
        if let Some(ref status_filter) = query.status_filter {
            if !status_filter.contains(&plugin.status) {
                return false;
            }
        }

        // 标签过滤
        if let Some(ref required_tags) = query.tags {
            for required_tag in required_tags {
                if !plugin.metadata.tags.contains(required_tag) {
                    return false;
                }
            }
        }

        // 作者过滤
        if let Some(ref required_author) = query.author {
            if &plugin.metadata.author != required_author {
                return false;
            }
        }

        true
    }

    async fn resolve_dependencies_recursive(
        &self,
        plugin_metadata: &PluginMetadata,
        dependency_chain: &mut Vec<PluginId>,
        missing_dependencies: &mut Vec<PluginDependency>,
        version_conflicts: &mut Vec<VersionConflict>,
        visited: &mut HashSet<PluginId>,
    ) -> bool {
        if visited.contains(&plugin_metadata.id) {
            // 检测到循环依赖
            return false;
        }

        visited.insert(plugin_metadata.id.clone());
        let mut all_resolved = true;

        for dependency in &plugin_metadata.dependencies {
            if dependency.optional {
                continue; // 跳过可选依赖
            }

            let plugins = self.plugins.read().await;
            if let Some(dep_plugin) = plugins.get(&dependency.plugin_id) {
                // 检查版本兼容性
                let version_req = VersionReq::parse(&dependency.version_req);
                match version_req {
                    Ok(req) => {
                        if !req.matches(&dep_plugin.metadata.version) {
                            version_conflicts.push(VersionConflict {
                                plugin_id: dependency.plugin_id.clone(),
                                required_version: dependency.version_req.clone(),
                                available_version: dep_plugin.metadata.version.clone(),
                                conflicting_requester: plugin_metadata.id.clone(),
                            });
                            all_resolved = false;
                        } else {
                            dependency_chain.push(dependency.plugin_id.clone());
                            
                            // 递归解析依赖的依赖
                            if !self.resolve_dependencies_recursive(
                                &dep_plugin.metadata,
                                dependency_chain,
                                missing_dependencies,
                                version_conflicts,
                                visited,
                            ).await {
                                all_resolved = false;
                            }
                        }
                    }
                    Err(_) => {
                        // 版本要求格式错误
                        all_resolved = false;
                    }
                }
            } else {
                // 缺失依赖
                missing_dependencies.push(dependency.clone());
                all_resolved = false;
            }
        }

        visited.remove(&plugin_metadata.id);
        all_resolved
    }

    async fn try_load_plugin_metadata(&self, path: &Path) -> Result<PluginMetadata> {
        // 简化的元数据加载实现
        // 在实际应用中，这里会根据文件类型和格式解析插件元数据
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = std::fs::read_to_string(path)?;
            let metadata: PluginMetadata = serde_json::from_str(&content)?;
            Ok(metadata)
        } else {
            Err(MosesQuantError::Internal {
                message: "Unsupported plugin metadata format".to_string()
            })
        }
    }
}

/// 注册表统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// 总插件数量
    pub total_plugins: usize,
    /// 按类型分组的统计
    pub by_type: HashMap<PluginType, usize>,
    /// 按状态分组的统计
    pub by_status: HashMap<PluginRegistrationStatus, usize>,
    /// 按信任级别分组的统计
    pub by_trust_level: HashMap<TrustLevel, usize>,
    /// 平均性能评分
    pub average_performance_score: f64,
}

/// 使用统计更新
#[derive(Debug, Clone)]
pub enum UsageStatsUpdate {
    Launch,
    Crash,
    Runtime(i64),
    PerformanceScore(f64),
    UserRating(f64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_metadata(id: &str) -> PluginMetadata {
        PluginMetadata {
            id: id.to_string(),
            name: format!("Test Plugin {}", id),
            version: Version::new(1, 0, 0),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            plugin_type: PluginType::Utility,
            capabilities: vec![PluginCapability::Custom("test".to_string())],
            dependencies: vec![],
            min_framework_version: Version::new(2, 0, 0),
            max_framework_version: None,
            config_schema: None,
            tags: vec!["test".to_string()],
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        let metadata = create_test_metadata("test_plugin");
        let location = PluginLocation::Embedded;

        // 注册插件
        registry.register_plugin(metadata.clone(), location).await.unwrap();

        // 获取插件
        let registered = registry.get_plugin("test_plugin").await.unwrap();
        assert_eq!(registered.metadata.id, "test_plugin");
        assert_eq!(registered.status, PluginRegistrationStatus::Registered);

        // 检查重复注册
        let location2 = PluginLocation::Embedded;
        let result = registry.register_plugin(metadata, location2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plugin_query() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        // 注册多个插件
        let mut metadata1 = create_test_metadata("plugin1");
        metadata1.plugin_type = PluginType::DataSource;
        metadata1.capabilities = vec![PluginCapability::RealTimeData];

        let mut metadata2 = create_test_metadata("plugin2");
        metadata2.plugin_type = PluginType::Strategy;
        metadata2.capabilities = vec![PluginCapability::MachineLearning];

        registry.register_plugin(metadata1, PluginLocation::Embedded).await.unwrap();
        registry.register_plugin(metadata2, PluginLocation::Embedded).await.unwrap();

        // 按类型查询
        let data_sources = registry.find_by_type(&PluginType::DataSource).await.unwrap();
        assert_eq!(data_sources.len(), 1);
        assert_eq!(data_sources[0].metadata.id, "plugin1");

        // 按能力查询
        let ml_plugins = registry.find_by_capability(&PluginCapability::MachineLearning).await.unwrap();
        assert_eq!(ml_plugins.len(), 1);
        assert_eq!(ml_plugins[0].metadata.id, "plugin2");

        // 复合查询
        let query = PluginQuery {
            plugin_type: Some(PluginType::DataSource),
            required_capabilities: vec![PluginCapability::RealTimeData],
            ..Default::default()
        };
        let results = registry.find_plugins(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.id, "plugin1");
    }

    #[tokio::test]
    async fn test_dependency_resolution() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        // 创建有依赖关系的插件
        let mut base_metadata = create_test_metadata("base_plugin");
        base_metadata.dependencies = vec![];

        let mut dependent_metadata = create_test_metadata("dependent_plugin");
        dependent_metadata.dependencies = vec![
            PluginDependency {
                plugin_id: "base_plugin".to_string(),
                version_req: "^1.0".to_string(),
                optional: false,
            }
        ];

        registry.register_plugin(base_metadata, PluginLocation::Embedded).await.unwrap();
        registry.register_plugin(dependent_metadata, PluginLocation::Embedded).await.unwrap();

        // 解析依赖
        let resolution = registry.resolve_dependencies("dependent_plugin").await.unwrap();
        assert!(resolution.resolvable);
        assert!(resolution.dependency_chain.contains(&"base_plugin".to_string()));
        assert!(resolution.missing_dependencies.is_empty());
        assert!(resolution.version_conflicts.is_empty());
    }

    #[tokio::test]
    async fn test_usage_stats_update() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        let metadata = create_test_metadata("test_plugin");
        registry.register_plugin(metadata, PluginLocation::Embedded).await.unwrap();

        // 更新使用统计
        registry.update_usage_stats("test_plugin", UsageStatsUpdate::Launch).await.unwrap();
        registry.update_usage_stats("test_plugin", UsageStatsUpdate::Runtime(1000000)).await.unwrap();
        registry.update_usage_stats("test_plugin", UsageStatsUpdate::PerformanceScore(0.95)).await.unwrap();

        // 检查统计
        let plugin = registry.get_plugin("test_plugin").await.unwrap();
        assert_eq!(plugin.usage_stats.launch_count, 1);
        assert_eq!(plugin.usage_stats.total_runtime_ns, 1000000);
        assert_eq!(plugin.usage_stats.performance_score, 0.95);
    }

    #[tokio::test]
    async fn test_plugin_discovery() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        // 创建临时目录和测试文件
        let temp_dir = TempDir::new().unwrap();
        let plugin_file = temp_dir.path().join("test_plugin.json");

        let metadata = create_test_metadata("discovered_plugin");
        let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
        std::fs::write(&plugin_file, metadata_json).unwrap();

        // 添加搜索路径
        registry.add_search_path(temp_dir.path().to_path_buf()).await.unwrap();

        // 发现插件
        let discovered = registry.discover_plugins().await.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].id, "discovered_plugin");
    }

    #[tokio::test]
    async fn test_registry_stats() {
        let config = RegistryConfig::default();
        let registry = PluginRegistry::new(config);

        // 注册不同类型的插件
        let mut metadata1 = create_test_metadata("plugin1");
        metadata1.plugin_type = PluginType::DataSource;

        let mut metadata2 = create_test_metadata("plugin2");
        metadata2.plugin_type = PluginType::Strategy;

        let mut metadata3 = create_test_metadata("plugin3");
        metadata3.plugin_type = PluginType::DataSource;

        registry.register_plugin(metadata1, PluginLocation::Embedded).await.unwrap();
        registry.register_plugin(metadata2, PluginLocation::Embedded).await.unwrap();
        registry.register_plugin(metadata3, PluginLocation::Embedded).await.unwrap();

        // 获取统计信息
        let stats = registry.get_registry_stats().await;
        assert_eq!(stats.total_plugins, 3);
        assert_eq!(stats.by_type.get(&PluginType::DataSource), Some(&2));
        assert_eq!(stats.by_type.get(&PluginType::Strategy), Some(&1));
    }
}