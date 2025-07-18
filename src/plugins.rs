//! 策略插件系统
//! 
//! 提供动态加载用户自定义策略的能力，支持插件化架构

use crate::{
    strategy::{AlphaModel, PortfolioConstructor, RiskManager, UniverseSelector, ExecutionAlgorithm, AlphaModelConfig},
    types::{Symbol, Insight},
    Result, CzscError,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;

/// 策略插件接口
/// 
/// 用户实现此接口来创建自定义策略组件
#[async_trait]
pub trait StrategyPlugin: Send + Sync {
    /// 获取插件名称
    fn name(&self) -> &str;
    
    /// 获取插件版本
    fn version(&self) -> &str;
    
    /// 获取插件描述
    fn description(&self) -> &str;
    
    /// 获取插件作者
    fn author(&self) -> &str;
    
    /// 插件初始化
    async fn initialize(&mut self) -> Result<()>;
    
    /// 插件清理
    async fn cleanup(&mut self) -> Result<()>;
    
    /// 创建Alpha模型
    fn create_alpha_model(&self, config: &AlphaModelConfig) -> Result<Box<dyn AlphaModel>>;
    
    /// 创建投资组合构建器（可选）
    fn create_portfolio_constructor(&self, _config: &HashMap<String, String>) -> Result<Option<Box<dyn PortfolioConstructor>>> {
        Ok(None)
    }
    
    /// 创建风险管理器（可选）
    fn create_risk_manager(&self, _config: &HashMap<String, String>) -> Result<Option<Box<dyn RiskManager>>> {
        Ok(None)
    }
    
    /// 创建标的选择器（可选）
    fn create_universe_selector(&self, _config: &HashMap<String, String>) -> Result<Option<Box<dyn UniverseSelector>>> {
        Ok(None)
    }
    
    /// 创建执行算法（可选）
    fn create_execution_algorithm(&self, _config: &HashMap<String, String>) -> Result<Option<Box<dyn ExecutionAlgorithm>>> {
        Ok(None)
    }
}

/// 插件元数据
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub file_path: String,
    pub loaded: bool,
}

/// 插件注册表
/// 
/// 管理所有已注册的策略插件
pub struct PluginRegistry {
    /// 已注册的插件
    plugins: HashMap<String, Arc<dyn StrategyPlugin>>,
    /// 插件元数据
    metadata: HashMap<String, PluginMetadata>,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// 注册插件
    pub async fn register_plugin(&mut self, plugin: Arc<dyn StrategyPlugin>) -> Result<()> {
        let name = plugin.name().to_string();
        
        // 检查是否已存在同名插件
        if self.plugins.contains_key(&name) {
            return Err(CzscError::strategy(&format!("Plugin '{}' is already registered", name)));
        }
        
        // 创建元数据
        let metadata = PluginMetadata {
            name: name.clone(),
            version: plugin.version().to_string(),
            description: plugin.description().to_string(),
            author: plugin.author().to_string(),
            file_path: "memory".to_string(), // 内存中注册的插件
            loaded: true,
        };
        
        // 注册插件
        self.plugins.insert(name.clone(), plugin);
        self.metadata.insert(name, metadata);
        
        Ok(())
    }
    
    /// 卸载插件
    pub async fn unregister_plugin(&mut self, name: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.remove(name) {
            // 清理插件
            let mut plugin = Arc::try_unwrap(plugin)
                .map_err(|_| CzscError::strategy("Plugin is still in use"))?;
            plugin.cleanup().await?;
            
            // 移除元数据
            self.metadata.remove(name);
            
            Ok(())
        } else {
            Err(CzscError::strategy(&format!("Plugin '{}' not found", name)))
        }
    }
    
    /// 获取插件
    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn StrategyPlugin>> {
        self.plugins.get(name).cloned()
    }
    
    /// 列出所有插件
    pub fn list_plugins(&self) -> Vec<&PluginMetadata> {
        self.metadata.values().collect()
    }
    
    /// 获取插件元数据
    pub fn get_metadata(&self, name: &str) -> Option<&PluginMetadata> {
        self.metadata.get(name)
    }
    
    /// 创建Alpha模型
    pub fn create_alpha_model(&self, plugin_name: &str, config: &AlphaModelConfig) -> Result<Box<dyn AlphaModel>> {
        if let Some(plugin) = self.plugins.get(plugin_name) {
            plugin.create_alpha_model(config)
        } else {
            Err(CzscError::strategy(&format!("Plugin '{}' not found", plugin_name)))
        }
    }
    
    /// 搜索插件
    pub fn search_plugins(&self, query: &str) -> Vec<&PluginMetadata> {
        self.metadata.values()
            .filter(|metadata| {
                metadata.name.contains(query) ||
                metadata.description.contains(query) ||
                metadata.author.contains(query)
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 插件管理器
/// 
/// 提供插件的加载、卸载、管理功能
pub struct PluginManager {
    /// 插件注册表
    registry: PluginRegistry,
    /// 插件目录
    plugin_directories: Vec<String>,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            plugin_directories: vec![
                "./plugins".to_string(),
                "./strategies".to_string(),
                "./user_strategies".to_string(),
            ],
        }
    }
    
    /// 添加插件目录
    pub fn add_plugin_directory(&mut self, path: &str) {
        self.plugin_directories.push(path.to_string());
    }
    
    /// 注册插件
    pub async fn register_plugin(&mut self, plugin: Arc<dyn StrategyPlugin>) -> Result<()> {
        self.registry.register_plugin(plugin).await
    }
    
    /// 卸载插件
    pub async fn unregister_plugin(&mut self, name: &str) -> Result<()> {
        self.registry.unregister_plugin(name).await
    }
    
    /// 获取插件注册表
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
    
    /// 获取可变插件注册表
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }
    
    /// 扫描并加载插件目录中的插件
    pub async fn scan_and_load_plugins(&mut self) -> Result<usize> {
        let mut loaded_count = 0;
        
        for directory in &self.plugin_directories.clone() {
            if let Ok(loaded) = self.scan_directory(directory).await {
                loaded_count += loaded;
            }
        }
        
        Ok(loaded_count)
    }
    
    /// 扫描目录中的插件
    async fn scan_directory(&mut self, directory: &str) -> Result<usize> {
        // 简化实现：在实际项目中，这里会扫描目录中的动态库文件
        // 并使用libloading等库加载插件
        
        tracing::info!("Scanning plugin directory: {}", directory);
        
        // 对于演示目的，我们返回0
        // 在真实实现中，这里会：
        // 1. 扫描目录中的.so/.dll/.dylib文件
        // 2. 加载动态库
        // 3. 查找插件入口点
        // 4. 创建插件实例
        // 5. 注册到registry中
        
        Ok(0)
    }
    
    /// 重新加载所有插件
    pub async fn reload_all_plugins(&mut self) -> Result<()> {
        // 清理所有现有插件
        let plugin_names: Vec<String> = self.registry.plugins.keys().cloned().collect();
        for name in plugin_names {
            self.unregister_plugin(&name).await?;
        }
        
        // 重新扫描和加载
        self.scan_and_load_plugins().await?;
        
        Ok(())
    }
    
    /// 获取插件统计信息
    pub fn get_statistics(&self) -> PluginStatistics {
        PluginStatistics {
            total_plugins: self.registry.plugins.len(),
            loaded_plugins: self.registry.metadata.values().filter(|m| m.loaded).count(),
            plugin_directories: self.plugin_directories.len(),
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 插件统计信息
#[derive(Debug, Clone)]
pub struct PluginStatistics {
    pub total_plugins: usize,
    pub loaded_plugins: usize,
    pub plugin_directories: usize,
}

/// 示例插件：简单RSI策略
pub struct SimpleRSIPlugin {
    name: String,
    version: String,
}

impl SimpleRSIPlugin {
    pub fn new() -> Self {
        Self {
            name: "SimpleRSI".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

#[async_trait]
impl StrategyPlugin for SimpleRSIPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        "Simple RSI-based trading strategy plugin"
    }
    
    fn author(&self) -> &str {
        "MosesQuant Team"
    }
    
    async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing SimpleRSI plugin");
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up SimpleRSI plugin");
        Ok(())
    }
    
    fn create_alpha_model(&self, config: &AlphaModelConfig) -> Result<Box<dyn AlphaModel>> {
        // 这里应该返回实际的RSI Alpha模型实现
        // 为了简化，我们返回一个错误，表示需要实现
        Err(CzscError::strategy("SimpleRSI AlphaModel implementation needed"))
    }
}

/// 插件配置
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// 插件名称
    pub name: String,
    /// 插件参数
    pub parameters: HashMap<String, String>,
    /// 是否启用
    pub enabled: bool,
    /// 优先级
    pub priority: u32,
}

impl PluginConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parameters: HashMap::new(),
            enabled: true,
            priority: 100,
        }
    }
    
    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        
        // 创建示例插件
        let plugin = Arc::new(SimpleRSIPlugin::new());
        
        // 注册插件
        registry.register_plugin(plugin.clone()).await.unwrap();
        
        // 检查插件是否注册成功
        assert!(registry.get_plugin("SimpleRSI").is_some());
        assert_eq!(registry.list_plugins().len(), 1);
        
        // 获取元数据
        let metadata = registry.get_metadata("SimpleRSI").unwrap();
        assert_eq!(metadata.name, "SimpleRSI");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.loaded);
        
        // 卸载插件
        registry.unregister_plugin("SimpleRSI").await.unwrap();
        assert!(registry.get_plugin("SimpleRSI").is_none());
        assert_eq!(registry.list_plugins().len(), 0);
    }
    
    #[tokio::test]
    async fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        
        // 添加插件目录
        manager.add_plugin_directory("./test_plugins");
        
        // 创建并注册插件
        let plugin = Arc::new(SimpleRSIPlugin::new());
        manager.register_plugin(plugin).await.unwrap();
        
        // 检查统计信息
        let stats = manager.get_statistics();
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.loaded_plugins, 1);
        assert_eq!(stats.plugin_directories, 4); // 3 default + 1 added
        
        // 检查插件存在
        assert!(manager.registry().get_plugin("SimpleRSI").is_some());
    }
    
    #[test]
    fn test_plugin_config() {
        let config = PluginConfig::new("TestPlugin")
            .with_parameter("period", "14")
            .with_parameter("threshold", "70")
            .with_priority(200)
            .enabled(true);
        
        assert_eq!(config.name, "TestPlugin");
        assert_eq!(config.parameters.get("period"), Some(&"14".to_string()));
        assert_eq!(config.parameters.get("threshold"), Some(&"70".to_string()));
        assert_eq!(config.priority, 200);
        assert!(config.enabled);
    }
    
    #[test]
    fn test_plugin_search() {
        let mut registry = PluginRegistry::new();
        
        // 手动添加一些测试元数据
        let metadata = PluginMetadata {
            name: "RSIStrategy".to_string(),
            version: "1.0.0".to_string(),
            description: "RSI-based trading strategy".to_string(),
            author: "TestAuthor".to_string(),
            file_path: "test.so".to_string(),
            loaded: true,
        };
        registry.metadata.insert("RSIStrategy".to_string(), metadata);
        
        // 搜索测试
        let results = registry.search_plugins("RSI");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "RSIStrategy");
        
        let results = registry.search_plugins("trading");
        assert_eq!(results.len(), 1);
        
        let results = registry.search_plugins("nonexistent");
        assert_eq!(results.len(), 0);
    }
}