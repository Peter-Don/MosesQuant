//! 可插拔配置管理器
//! 
//! 基于分层配置架构的高性能配置管理系统，支持动态配置更新和多源配置融合

use crate::plugins::*;
use crate::types::*;
use crate::{Result, MosesQuantError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, broadcast};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 配置管理器配置
#[derive(Debug, Clone)]
pub struct ConfigManagerConfig {
    /// 配置文件根目录
    pub config_root_dir: PathBuf,
    /// 是否启用配置热重载
    pub enable_hot_reload: bool,
    /// 配置文件监控间隔
    pub watch_interval: Duration,
    /// 是否启用配置验证
    pub enable_validation: bool,
    /// 是否启用配置缓存
    pub enable_caching: bool,
    /// 缓存过期时间
    pub cache_expiry: Duration,
    /// 最大配置历史版本数
    pub max_history_versions: usize,
    /// 配置更新超时时间
    pub update_timeout: Duration,
    /// 是否启用配置加密
    pub enable_encryption: bool,
}

impl Default for ConfigManagerConfig {
    fn default() -> Self {
        Self {
            config_root_dir: PathBuf::from("./config"),
            enable_hot_reload: true,
            watch_interval: Duration::from_secs(5),
            enable_validation: true,
            enable_caching: true,
            cache_expiry: Duration::from_secs(300), // 5分钟
            max_history_versions: 10,
            update_timeout: Duration::from_secs(30),
            enable_encryption: false,
        }
    }
}

/// 配置源类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSourceType {
    /// 文件配置源
    File,
    /// 环境变量配置源
    Environment,
    /// 命令行参数配置源
    CommandLine,
    /// 数据库配置源
    Database,
    /// 远程配置服务
    Remote,
    /// 插件配置源
    Plugin,
}

/// 配置源优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigPriority {
    /// 最低优先级（框架默认）
    Lowest = 0,
    /// 低优先级（文件配置）
    Low = 1,
    /// 中等优先级（环境变量）
    Medium = 2,
    /// 高优先级（命令行参数）
    High = 3,
    /// 最高优先级（运行时覆盖）
    Highest = 4,
}

/// 配置源接口
#[async_trait]
pub trait ConfigSource: Send + Sync {
    /// 获取配置源类型
    fn source_type(&self) -> ConfigSourceType;
    
    /// 获取配置源优先级
    fn priority(&self) -> ConfigPriority;
    
    /// 加载配置数据
    async fn load_config(&self) -> Result<ConfigData>;
    
    /// 保存配置数据（如果支持）
    async fn save_config(&self, data: &ConfigData) -> Result<()>;
    
    /// 检查是否支持热重载
    fn supports_hot_reload(&self) -> bool;
    
    /// 开始监控配置变化
    async fn start_watching(&self) -> Result<broadcast::Receiver<ConfigChangeEvent>>;
    
    /// 停止监控配置变化
    async fn stop_watching(&self) -> Result<()>;
}

/// 配置数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    /// 配置版本
    pub version: String,
    /// 配置数据
    pub data: JsonValue,
    /// 最后更新时间
    pub last_updated: i64,
    /// 配置源信息
    pub source_info: ConfigSourceInfo,
    /// 配置元数据
    pub metadata: HashMap<String, String>,
}

/// 配置源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSourceInfo {
    /// 源类型
    pub source_type: String,
    /// 源标识符
    pub source_id: String,
    /// 源优先级
    pub priority: u8,
    /// 源描述
    pub description: String,
}

/// 配置变化事件
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// 变化类型
    pub change_type: ConfigChangeType,
    /// 配置路径
    pub config_path: String,
    /// 旧值
    pub old_value: Option<JsonValue>,
    /// 新值
    pub new_value: Option<JsonValue>,
    /// 事件时间戳
    pub timestamp: i64,
    /// 源信息
    pub source: ConfigSourceInfo,
}

/// 配置变化类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeType {
    /// 配置创建
    Created,
    /// 配置更新
    Updated,
    /// 配置删除
    Deleted,
    /// 配置重载
    Reloaded,
}

/// 配置验证结果
#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    /// 验证是否通过
    pub valid: bool,
    /// 验证错误
    pub errors: Vec<String>,
    /// 验证警告
    pub warnings: Vec<String>,
    /// 配置路径
    pub config_path: String,
}

/// 配置管理器
pub struct ConfigManager {
    /// 配置源列表
    config_sources: Arc<RwLock<Vec<Box<dyn ConfigSource>>>>,
    /// 合并后的配置缓存
    config_cache: Arc<RwLock<HashMap<String, ConfigData>>>,
    /// 配置历史版本
    config_history: Arc<RwLock<HashMap<String, Vec<ConfigData>>>>,
    /// 配置管理器配置
    config: ConfigManagerConfig,
    /// 配置变化事件广播器
    change_sender: broadcast::Sender<ConfigChangeEvent>,
    /// 配置验证器
    validators: Arc<RwLock<HashMap<String, Box<dyn ConfigValidator>>>>,
    /// 配置转换器
    transformers: Arc<RwLock<HashMap<String, Box<dyn ConfigTransformer>>>>,
    /// 管理器状态
    manager_state: Arc<RwLock<ConfigManagerState>>,
    /// 插件注册表
    plugin_registry: Arc<PluginRegistry>,
}

/// 配置管理器状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigManagerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

/// 配置验证器接口
#[async_trait]
pub trait ConfigValidator: Send + Sync {
    /// 验证配置数据
    async fn validate(&self, config: &ConfigData) -> ConfigValidationResult;
    
    /// 获取验证器名称
    fn name(&self) -> &str;
    
    /// 获取支持的配置路径模式
    fn supported_paths(&self) -> Vec<String>;
}

/// 配置转换器接口
#[async_trait]
pub trait ConfigTransformer: Send + Sync {
    /// 转换配置数据
    async fn transform(&self, config: &ConfigData) -> Result<ConfigData>;
    
    /// 获取转换器名称
    fn name(&self) -> &str;
    
    /// 获取支持的配置路径模式
    fn supported_paths(&self) -> Vec<String>;
}

/// 文件配置源实现
pub struct FileConfigSource {
    /// 文件路径
    file_path: PathBuf,
    /// 优先级
    priority: ConfigPriority,
    /// 是否监控文件变化
    watch_enabled: bool,
    /// 文件格式
    format: ConfigFileFormat,
}

/// 配置文件格式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigFileFormat {
    Json,
    Yaml,
    Toml,
    Properties,
}

impl FileConfigSource {
    pub fn new(file_path: PathBuf, priority: ConfigPriority, format: ConfigFileFormat) -> Self {
        Self {
            file_path,
            priority,
            watch_enabled: true,
            format,
        }
    }
}

#[async_trait]
impl ConfigSource for FileConfigSource {
    fn source_type(&self) -> ConfigSourceType {
        ConfigSourceType::File
    }
    
    fn priority(&self) -> ConfigPriority {
        self.priority.clone()
    }
    
    async fn load_config(&self) -> Result<ConfigData> {
        let content = tokio::fs::read_to_string(&self.file_path).await
            .map_err(|e| MosesQuantError::ConfigIO {
                message: format!("Failed to read config file {:?}: {}", self.file_path, e)
            })?;

        let data = match self.format {
            ConfigFileFormat::Json => {
                serde_json::from_str(&content)
                    .map_err(|e| MosesQuantError::ConfigParsing {
                        message: format!("Failed to parse JSON config: {}", e)
                    })?
            }
            ConfigFileFormat::Yaml => {
                serde_yaml::from_str(&content)
                    .map_err(|e| MosesQuantError::ConfigParsing {
                        message: format!("Failed to parse YAML config: {}", e)
                    })?
            }
            ConfigFileFormat::Toml => {
                let toml_value: toml::Value = content.parse()
                    .map_err(|e| MosesQuantError::ConfigParsing {
                        message: format!("Failed to parse TOML config: {}", e)
                    })?;
                
                serde_json::to_value(toml_value)
                    .map_err(|e| MosesQuantError::ConfigParsing {
                        message: format!("Failed to convert TOML to JSON: {}", e)
                    })?
            }
            ConfigFileFormat::Properties => {
                // 简化的properties文件解析
                let mut map = serde_json::Map::new();
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Some((key, value)) = line.split_once('=') {
                            map.insert(key.trim().to_string(), JsonValue::String(value.trim().to_string()));
                        }
                    }
                }
                JsonValue::Object(map)
            }
        };

        let metadata = tokio::fs::metadata(&self.file_path).await
            .map_err(|e| MosesQuantError::ConfigIO {
                message: format!("Failed to get file metadata: {}", e)
            })?;

        let last_updated = metadata.modified()
            .map_err(|e| MosesQuantError::ConfigIO {
                message: format!("Failed to get file modification time: {}", e)
            })?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(ConfigData {
            version: format!("file-{}", last_updated),
            data,
            last_updated,
            source_info: ConfigSourceInfo {
                source_type: "file".to_string(),
                source_id: self.file_path.to_string_lossy().to_string(),
                priority: self.priority.clone() as u8,
                description: format!("File config source: {:?}", self.file_path),
            },
            metadata: HashMap::new(),
        })
    }
    
    async fn save_config(&self, data: &ConfigData) -> Result<()> {
        let content = match self.format {
            ConfigFileFormat::Json => {
                serde_json::to_string_pretty(&data.data)
                    .map_err(|e| MosesQuantError::ConfigSerialization {
                        message: format!("Failed to serialize to JSON: {}", e)
                    })?
            }
            ConfigFileFormat::Yaml => {
                serde_yaml::to_string(&data.data)
                    .map_err(|e| MosesQuantError::ConfigSerialization {
                        message: format!("Failed to serialize to YAML: {}", e)
                    })?
            }
            ConfigFileFormat::Toml => {
                let toml_value: toml::Value = serde_json::from_value(data.data.clone())
                    .map_err(|e| MosesQuantError::ConfigSerialization {
                        message: format!("Failed to convert to TOML: {}", e)
                    })?;
                
                toml::to_string_pretty(&toml_value)
                    .map_err(|e| MosesQuantError::ConfigSerialization {
                        message: format!("Failed to serialize TOML: {}", e)
                    })?
            }
            ConfigFileFormat::Properties => {
                if let JsonValue::Object(map) = &data.data {
                    let mut content = String::new();
                    for (key, value) in map {
                        if let JsonValue::String(str_val) = value {
                            content.push_str(&format!("{}={}\n", key, str_val));
                        }
                    }
                    content
                } else {
                    return Err(MosesQuantError::ConfigSerialization {
                        message: "Properties format only supports string key-value pairs".to_string()
                    });
                }
            }
        };

        // 确保目录存在
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| MosesQuantError::ConfigIO {
                    message: format!("Failed to create config directory: {}", e)
                })?;
        }

        tokio::fs::write(&self.file_path, content).await
            .map_err(|e| MosesQuantError::ConfigIO {
                message: format!("Failed to write config file: {}", e)
            })?;

        Ok(())
    }
    
    fn supports_hot_reload(&self) -> bool {
        self.watch_enabled
    }
    
    async fn start_watching(&self) -> Result<broadcast::Receiver<ConfigChangeEvent>> {
        let (sender, receiver) = broadcast::channel(100);
        
        if self.watch_enabled {
            // 实际实现中这里会启动文件监控
            // 为简化示例，这里只是创建一个空的接收器
            debug!("Started watching config file: {:?}", self.file_path);
        }
        
        Ok(receiver)
    }
    
    async fn stop_watching(&self) -> Result<()> {
        if self.watch_enabled {
            debug!("Stopped watching config file: {:?}", self.file_path);
        }
        Ok(())
    }
}

/// 环境变量配置源
pub struct EnvironmentConfigSource {
    /// 环境变量前缀
    prefix: String,
    /// 优先级
    priority: ConfigPriority,
}

impl EnvironmentConfigSource {
    pub fn new(prefix: String, priority: ConfigPriority) -> Self {
        Self { prefix, priority }
    }
}

#[async_trait]
impl ConfigSource for EnvironmentConfigSource {
    fn source_type(&self) -> ConfigSourceType {
        ConfigSourceType::Environment
    }
    
    fn priority(&self) -> ConfigPriority {
        self.priority.clone()
    }
    
    async fn load_config(&self) -> Result<ConfigData> {
        let mut config_map = serde_json::Map::new();
        
        for (key, value) in std::env::vars() {
            if key.starts_with(&self.prefix) {
                let config_key = key.strip_prefix(&self.prefix)
                    .unwrap_or(&key)
                    .trim_start_matches('_')
                    .to_lowercase()
                    .replace('_', ".");
                
                config_map.insert(config_key, JsonValue::String(value));
            }
        }

        Ok(ConfigData {
            version: "env-latest".to_string(),
            data: JsonValue::Object(config_map),
            last_updated: chrono::Utc::now().timestamp(),
            source_info: ConfigSourceInfo {
                source_type: "environment".to_string(),
                source_id: format!("env-{}", self.prefix),
                priority: self.priority.clone() as u8,
                description: format!("Environment variables with prefix: {}", self.prefix),
            },
            metadata: HashMap::new(),
        })
    }
    
    async fn save_config(&self, _data: &ConfigData) -> Result<()> {
        Err(MosesQuantError::ConfigOperation {
            message: "Environment variables are read-only".to_string()
        })
    }
    
    fn supports_hot_reload(&self) -> bool {
        false
    }
    
    async fn start_watching(&self) -> Result<broadcast::Receiver<ConfigChangeEvent>> {
        let (_, receiver) = broadcast::channel(1);
        Ok(receiver)
    }
    
    async fn stop_watching(&self) -> Result<()> {
        Ok(())
    }
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(
        config: ConfigManagerConfig,
        plugin_registry: Arc<PluginRegistry>,
    ) -> Self {
        let (change_sender, _) = broadcast::channel(1000);
        
        Self {
            config_sources: Arc::new(RwLock::new(Vec::new())),
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            config_history: Arc::new(RwLock::new(HashMap::new())),
            config,
            change_sender,
            validators: Arc::new(RwLock::new(HashMap::new())),
            transformers: Arc::new(RwLock::new(HashMap::new())),
            manager_state: Arc::new(RwLock::new(ConfigManagerState::Stopped)),
            plugin_registry,
        }
    }

    /// 添加配置源
    pub async fn add_config_source(&self, source: Box<dyn ConfigSource>) -> Result<()> {
        let mut sources = self.config_sources.write().await;
        sources.push(source);
        
        // 按优先级排序
        sources.sort_by(|a, b| b.priority().cmp(&a.priority()));
        
        info!("Added config source with priority: {:?}", sources.last().unwrap().priority());
        Ok(())
    }

    /// 添加文件配置源
    pub async fn add_file_source<P: AsRef<Path>>(
        &self, 
        path: P, 
        priority: ConfigPriority,
        format: ConfigFileFormat
    ) -> Result<()> {
        let source = Box::new(FileConfigSource::new(
            path.as_ref().to_path_buf(),
            priority,
            format
        ));
        self.add_config_source(source).await
    }

    /// 添加环境变量配置源
    pub async fn add_env_source(&self, prefix: String, priority: ConfigPriority) -> Result<()> {
        let source = Box::new(EnvironmentConfigSource::new(prefix, priority));
        self.add_config_source(source).await
    }

    /// 获取配置值
    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let merged_config = self.get_merged_config().await?;
        
        let value = self.get_value_by_path(&merged_config.data, path)
            .ok_or_else(|| MosesQuantError::ConfigKeyNotFound {
                key: path.to_string()
            })?;

        serde_json::from_value(value)
            .map_err(|e| MosesQuantError::ConfigDeserialization {
                message: format!("Failed to deserialize config value at '{}': {}", path, e)
            })
    }

    /// 设置配置值
    pub async fn set<T>(&self, path: &str, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| MosesQuantError::ConfigSerialization {
                message: format!("Failed to serialize config value: {}", e)
            })?;

        self.set_value(path, json_value).await
    }

    /// 重新加载所有配置
    pub async fn reload_all(&self) -> Result<()> {
        {
            let mut state = self.manager_state.write().await;
            if *state == ConfigManagerState::Stopped {
                return Err(MosesQuantError::ConfigOperation {
                    message: "Config manager is not running".to_string()
                });
            }
        }

        let sources = self.config_sources.read().await;
        let mut new_configs = HashMap::new();

        for source in sources.iter() {
            match source.load_config().await {
                Ok(config_data) => {
                    let source_id = config_data.source_info.source_id.clone();
                    new_configs.insert(source_id, config_data);
                }
                Err(e) => {
                    warn!("Failed to reload config from source: {:?}", e);
                }
            }
        }

        // 更新缓存
        {
            let mut cache = self.config_cache.write().await;
            *cache = new_configs;
        }

        // 广播重载事件
        let event = ConfigChangeEvent {
            change_type: ConfigChangeType::Reloaded,
            config_path: "*".to_string(),
            old_value: None,
            new_value: None,
            timestamp: chrono::Utc::now().timestamp(),
            source: ConfigSourceInfo {
                source_type: "manager".to_string(),
                source_id: "config_manager".to_string(),
                priority: ConfigPriority::Highest as u8,
                description: "Config manager reload".to_string(),
            },
        };

        if let Err(_) = self.change_sender.send(event) {
            debug!("No subscribers for config change events");
        }

        info!("All configurations reloaded successfully");
        Ok(())
    }

    /// 获取配置变化事件订阅器
    pub fn subscribe_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_sender.subscribe()
    }

    /// 添加配置验证器
    pub async fn add_validator(&self, path_pattern: String, validator: Box<dyn ConfigValidator>) -> Result<()> {
        let mut validators = self.validators.write().await;
        validators.insert(path_pattern, validator);
        Ok(())
    }

    /// 添加配置转换器
    pub async fn add_transformer(&self, path_pattern: String, transformer: Box<dyn ConfigTransformer>) -> Result<()> {
        let mut transformers = self.transformers.write().await;
        transformers.insert(path_pattern, transformer);
        Ok(())
    }

    /// 验证配置
    pub async fn validate_config(&self, config_path: &str) -> Result<ConfigValidationResult> {
        let merged_config = self.get_merged_config().await?;
        
        let validators = self.validators.read().await;
        for (pattern, validator) in validators.iter() {
            if self.path_matches_pattern(config_path, pattern) {
                return Ok(validator.validate(&merged_config).await);
            }
        }

        // 默认验证通过
        Ok(ConfigValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            config_path: config_path.to_string(),
        })
    }

    /// 启动配置管理器
    pub async fn start(&self) -> Result<()> {
        {
            let mut state = self.manager_state.write().await;
            if *state != ConfigManagerState::Stopped {
                return Err(MosesQuantError::ConfigOperation {
                    message: "Config manager is already running".to_string()
                });
            }
            *state = ConfigManagerState::Starting;
        }

        // 初始加载所有配置
        self.reload_all().await?;

        // 启动热重载监控
        if self.config.enable_hot_reload {
            self.start_hot_reload_monitoring().await?;
        }

        {
            let mut state = self.manager_state.write().await;
            *state = ConfigManagerState::Running;
        }

        info!("Config manager started successfully");
        Ok(())
    }

    /// 停止配置管理器
    pub async fn stop(&self) -> Result<()> {
        {
            let mut state = self.manager_state.write().await;
            if *state != ConfigManagerState::Running {
                return Err(MosesQuantError::ConfigOperation {
                    message: "Config manager is not running".to_string()
                });
            }
            *state = ConfigManagerState::Stopping;
        }

        // 停止所有配置源的监控
        let sources = self.config_sources.read().await;
        for source in sources.iter() {
            if source.supports_hot_reload() {
                if let Err(e) = source.stop_watching().await {
                    warn!("Failed to stop watching config source: {:?}", e);
                }
            }
        }

        {
            let mut state = self.manager_state.write().await;
            *state = ConfigManagerState::Stopped;
        }

        info!("Config manager stopped successfully");
        Ok(())
    }

    /// 获取管理器状态
    pub async fn get_state(&self) -> ConfigManagerState {
        self.manager_state.read().await.clone()
    }

    // 私有方法

    /// 获取合并后的配置
    async fn get_merged_config(&self) -> Result<ConfigData> {
        let cache = self.config_cache.read().await;
        
        if cache.is_empty() {
            return Err(MosesQuantError::ConfigOperation {
                message: "No configuration loaded".to_string()
            });
        }

        // 按优先级合并配置
        let mut merged_data = JsonValue::Object(serde_json::Map::new());
        let mut latest_timestamp = 0i64;

        // 从低优先级到高优先级合并
        let mut configs: Vec<_> = cache.values().collect();
        configs.sort_by_key(|config| config.source_info.priority);

        for config in configs {
            self.merge_json_values(&mut merged_data, &config.data);
            latest_timestamp = latest_timestamp.max(config.last_updated);
        }

        Ok(ConfigData {
            version: format!("merged-{}", latest_timestamp),
            data: merged_data,
            last_updated: latest_timestamp,
            source_info: ConfigSourceInfo {
                source_type: "merged".to_string(),
                source_id: "config_manager".to_string(),
                priority: ConfigPriority::Highest as u8,
                description: "Merged configuration from all sources".to_string(),
            },
            metadata: HashMap::new(),
        })
    }

    /// 通过路径获取配置值
    fn get_value_by_path(&self, data: &JsonValue, path: &str) -> Option<JsonValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;

        for part in parts {
            match current {
                JsonValue::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }

    /// 设置配置值
    async fn set_value(&self, path: &str, value: JsonValue) -> Result<()> {
        // 这里可以实现将值写回到适当的配置源
        // 为简化实现，这里只是广播变化事件
        
        let event = ConfigChangeEvent {
            change_type: ConfigChangeType::Updated,
            config_path: path.to_string(),
            old_value: None, // 在实际实现中应该获取旧值
            new_value: Some(value),
            timestamp: chrono::Utc::now().timestamp(),
            source: ConfigSourceInfo {
                source_type: "runtime".to_string(),
                source_id: "config_manager".to_string(),
                priority: ConfigPriority::Highest as u8,
                description: "Runtime configuration update".to_string(),
            },
        };

        if let Err(_) = self.change_sender.send(event) {
            debug!("No subscribers for config change events");
        }

        Ok(())
    }

    /// 合并JSON值
    fn merge_json_values(&self, target: &mut JsonValue, source: &JsonValue) {
        match (target, source) {
            (JsonValue::Object(target_map), JsonValue::Object(source_map)) => {
                for (key, value) in source_map {
                    match target_map.get_mut(key) {
                        Some(existing_value) => {
                            self.merge_json_values(existing_value, value);
                        }
                        None => {
                            target_map.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
            _ => {
                *target = source.clone();
            }
        }
    }

    /// 检查路径是否匹配模式
    fn path_matches_pattern(&self, path: &str, pattern: &str) -> bool {
        // 简化的通配符匹配实现
        if pattern == "*" {
            return true;
        }
        
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return path.starts_with(prefix);
        }
        
        path == pattern
    }

    /// 启动热重载监控
    async fn start_hot_reload_monitoring(&self) -> Result<()> {
        if !self.config.enable_hot_reload {
            return Ok();
        }

        // 为每个支持热重载的配置源启动监控
        let sources = self.config_sources.read().await;
        for source in sources.iter() {
            if source.supports_hot_reload() {
                // 在实际实现中，这里会启动文件监控线程
                debug!("Started hot reload monitoring for source: {:?}", source.source_type());
            }
        }

        Ok(())
    }

    /// 获取配置管理器统计信息
    pub async fn get_stats(&self) -> ConfigManagerStats {
        let cache = self.config_cache.read().await;
        let sources = self.config_sources.read().await;
        let validators = self.validators.read().await;
        let transformers = self.transformers.read().await;
        let state = self.manager_state.read().await;

        ConfigManagerStats {
            manager_state: state.clone(),
            total_sources: sources.len(),
            loaded_configs: cache.len(),
            total_validators: validators.len(),
            total_transformers: transformers.len(),
            hot_reload_enabled: self.config.enable_hot_reload,
            validation_enabled: self.config.enable_validation,
            caching_enabled: self.config.enable_caching,
        }
    }
}

/// 配置管理器统计信息
#[derive(Debug, Clone)]
pub struct ConfigManagerStats {
    /// 管理器状态
    pub manager_state: ConfigManagerState,
    /// 总配置源数量
    pub total_sources: usize,
    /// 已加载配置数量
    pub loaded_configs: usize,
    /// 验证器数量
    pub total_validators: usize,
    /// 转换器数量
    pub total_transformers: usize,
    /// 是否启用热重载
    pub hot_reload_enabled: bool,
    /// 是否启用验证
    pub validation_enabled: bool,
    /// 是否启用缓存
    pub caching_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let config = ConfigManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        
        let manager = ConfigManager::new(config, registry);
        let stats = manager.get_stats().await;
        
        assert_eq!(stats.manager_state, ConfigManagerState::Stopped);
        assert_eq!(stats.total_sources, 0);
    }

    #[tokio::test]
    async fn test_file_config_source() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"{"test_key": "test_value", "number": 42}"#).unwrap();
        
        let source = FileConfigSource::new(
            temp_file.path().to_path_buf(),
            ConfigPriority::Low,
            ConfigFileFormat::Json
        );
        
        let config_data = source.load_config().await.unwrap();
        assert_eq!(config_data.source_info.source_type, "file");
        
        if let JsonValue::Object(map) = &config_data.data {
            assert_eq!(map.get("test_key"), Some(&JsonValue::String("test_value".to_string())));
            assert_eq!(map.get("number"), Some(&JsonValue::Number(serde_json::Number::from(42))));
        } else {
            panic!("Expected JSON object");
        }
    }

    #[tokio::test]
    async fn test_environment_config_source() {
        std::env::set_var("TEST_CONFIG_KEY", "test_value");
        std::env::set_var("TEST_CONFIG_NUMBER", "123");
        
        let source = EnvironmentConfigSource::new("TEST_CONFIG".to_string(), ConfigPriority::Medium);
        let config_data = source.load_config().await.unwrap();
        
        if let JsonValue::Object(map) = &config_data.data {
            assert_eq!(map.get("key"), Some(&JsonValue::String("test_value".to_string())));
            assert_eq!(map.get("number"), Some(&JsonValue::String("123".to_string())));
        } else {
            panic!("Expected JSON object");
        }
        
        std::env::remove_var("TEST_CONFIG_KEY");
        std::env::remove_var("TEST_CONFIG_NUMBER");
    }

    #[tokio::test]
    async fn test_config_manager_with_multiple_sources() {
        let config = ConfigManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let manager = ConfigManager::new(config, registry);

        // 添加环境变量源
        std::env::set_var("TEST_APP_NAME", "MosesQuant");
        std::env::set_var("TEST_APP_VERSION", "2.0.0");
        manager.add_env_source("TEST_APP".to_string(), ConfigPriority::Medium).await.unwrap();

        // 添加文件源
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"{"name": "DefaultApp", "debug": true}"#).unwrap();
        manager.add_file_source(
            temp_file.path(),
            ConfigPriority::Low,
            ConfigFileFormat::Json
        ).await.unwrap();

        // 启动管理器并加载配置
        manager.start().await.unwrap();

        // 测试获取配置值
        let name: String = manager.get("name").await.unwrap();
        assert_eq!(name, "MosesQuant"); // 环境变量优先级更高

        let debug: bool = manager.get("debug").await.unwrap();
        assert!(debug); // 来自文件配置

        let version: String = manager.get("version").await.unwrap();
        assert_eq!(version, "2.0.0"); // 来自环境变量

        manager.stop().await.unwrap();
        
        std::env::remove_var("TEST_APP_NAME");
        std::env::remove_var("TEST_APP_VERSION");
    }

    #[tokio::test]
    async fn test_config_change_events() {
        let config = ConfigManagerConfig::default();
        let registry = Arc::new(PluginRegistry::new(crate::plugins::RegistryConfig::default()));
        let manager = ConfigManager::new(config, registry);

        let mut receiver = manager.subscribe_changes();
        
        manager.start().await.unwrap();

        // 触发配置重载
        manager.reload_all().await.unwrap();

        // 检查是否收到重载事件
        let event = tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await;
        match event {
            Ok(Ok(change_event)) => {
                assert_eq!(change_event.change_type, ConfigChangeType::Reloaded);
            }
            _ => panic!("Expected config reload event"),
        }

        manager.stop().await.unwrap();
    }
}