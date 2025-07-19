//! 插件元数据管理
//! 
//! 提供插件元数据的序列化、验证、版本管理和Schema定义

use super::core::*;
use crate::{Result, MosesQuantError};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, Map as JsonMap};
use semver::Version;
use std::collections::HashMap;
use jsonschema::{JSONSchema, ValidationError};

/// 元数据管理器
pub struct MetadataManager {
    /// 插件Schema缓存
    schema_cache: HashMap<String, JSONSchema>,
    /// 元数据验证规则
    validation_rules: ValidationRules,
}

/// 验证规则配置
#[derive(Debug, Clone)]
pub struct ValidationRules {
    /// 是否启用严格验证
    pub strict_validation: bool,
    /// 必需字段列表
    pub required_fields: Vec<String>,
    /// 禁止字段列表
    pub forbidden_fields: Vec<String>,
    /// 版本格式验证
    pub version_format: VersionFormat,
    /// 描述最小长度
    pub min_description_length: usize,
    /// 标签最大数量
    pub max_tags: usize,
    /// 依赖最大数量
    pub max_dependencies: usize,
}

/// 版本格式要求
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFormat {
    /// 语义化版本 (x.y.z)
    Semantic,
    /// 自定义格式
    Custom(String),
    /// 任意格式
    Any,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            strict_validation: true,
            required_fields: vec![
                "id".to_string(),
                "name".to_string(),
                "version".to_string(),
                "description".to_string(),
                "author".to_string(),
                "plugin_type".to_string(),
            ],
            forbidden_fields: vec![],
            version_format: VersionFormat::Semantic,
            min_description_length: 10,
            max_tags: 20,
            max_dependencies: 50,
        }
    }
}

/// 元数据验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否通过验证
    pub valid: bool,
    /// 验证错误列表
    pub errors: Vec<ValidationIssue>,
    /// 警告列表
    pub warnings: Vec<ValidationIssue>,
    /// 建议列表
    pub suggestions: Vec<String>,
}

/// 验证问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// 问题类型
    pub issue_type: IssueType,
    /// 字段路径
    pub field_path: String,
    /// 问题描述
    pub message: String,
    /// 严重程度
    pub severity: IssueSeverity,
    /// 修复建议
    pub fix_suggestion: Option<String>,
}

/// 问题类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    /// 缺失必需字段
    MissingRequiredField,
    /// 字段类型错误
    InvalidFieldType,
    /// 字段值无效
    InvalidFieldValue,
    /// 字段格式错误
    InvalidFieldFormat,
    /// 依赖问题
    DependencyIssue,
    /// 版本问题
    VersionIssue,
    /// Schema不匹配
    SchemaMismatch,
    /// 自定义验证错误
    CustomValidation,
}

/// 问题严重程度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 元数据构建器
pub struct MetadataBuilder {
    metadata: PluginMetadata,
    validation_enabled: bool,
}

/// 元数据模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTemplate {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 默认值
    pub defaults: JsonMap<String, JsonValue>,
    /// 可选字段
    pub optional_fields: Vec<String>,
    /// 字段描述
    pub field_descriptions: HashMap<String, String>,
    /// 示例值
    pub examples: JsonMap<String, JsonValue>,
}

impl MetadataManager {
    /// 创建新的元数据管理器
    pub fn new(validation_rules: ValidationRules) -> Self {
        Self {
            schema_cache: HashMap::new(),
            validation_rules,
        }
    }

    /// 验证插件元数据
    pub fn validate_metadata(&self, metadata: &PluginMetadata) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // 基础字段验证
        self.validate_basic_fields(metadata, &mut errors, &mut warnings);
        
        // 版本验证
        self.validate_version(metadata, &mut errors, &mut warnings);
        
        // 依赖验证
        self.validate_dependencies(metadata, &mut errors, &mut warnings);
        
        // 能力验证
        self.validate_capabilities(metadata, &mut warnings);
        
        // 标签验证
        self.validate_tags(metadata, &mut errors, &mut warnings);
        
        // 生成建议
        self.generate_suggestions(metadata, &mut suggestions);

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }

    /// 验证JSON Schema
    pub fn validate_config_schema(&mut self, schema: &JsonValue) -> Result<()> {
        match JSONSchema::compile(schema) {
            Ok(compiled_schema) => {
                self.schema_cache.insert("temp_schema".to_string(), compiled_schema);
                Ok(())
            }
            Err(e) => Err(MosesQuantError::ConfigValidation {
                message: format!("Invalid JSON Schema: {}", e)
            })
        }
    }

    /// 验证配置是否符合Schema
    pub fn validate_config_against_schema(
        &self, 
        config: &JsonValue, 
        schema: &JsonValue
    ) -> Result<Vec<String>> {
        let compiled_schema = JSONSchema::compile(schema)
            .map_err(|e| MosesQuantError::ConfigValidation {
                message: format!("Schema compilation failed: {}", e)
            })?;

        let validation_result = compiled_schema.validate(config);
        match validation_result {
            Ok(_) => Ok(Vec::new()),
            Err(errors) => Ok(errors.map(|e| e.to_string()).collect()),
        }
    }

    /// 序列化元数据
    pub fn serialize_metadata(&self, metadata: &PluginMetadata, format: SerializationFormat) -> Result<String> {
        match format {
            SerializationFormat::Json => {
                serde_json::to_string_pretty(metadata)
                    .map_err(|e| MosesQuantError::Serialization(e))
            }
            SerializationFormat::Yaml => {
                serde_yaml::to_string(metadata)
                    .map_err(|e| MosesQuantError::YamlSerialization(e))
            }
            SerializationFormat::Toml => {
                toml::to_string_pretty(metadata)
                    .map_err(|e| MosesQuantError::Internal {
                        message: format!("TOML serialization error: {}", e)
                    })
            }
        }
    }

    /// 反序列化元数据
    pub fn deserialize_metadata(&self, data: &str, format: SerializationFormat) -> Result<PluginMetadata> {
        let metadata = match format {
            SerializationFormat::Json => {
                serde_json::from_str(data)
                    .map_err(|e| MosesQuantError::Serialization(e))?
            }
            SerializationFormat::Yaml => {
                serde_yaml::from_str(data)
                    .map_err(|e| MosesQuantError::YamlSerialization(e))?
            }
            SerializationFormat::Toml => {
                toml::from_str(data)
                    .map_err(|e| MosesQuantError::Internal {
                        message: format!("TOML deserialization error: {}", e)
                    })?
            }
        };

        // 验证反序列化的元数据
        let validation_result = self.validate_metadata(&metadata);
        if !validation_result.valid && self.validation_rules.strict_validation {
            return Err(MosesQuantError::ConfigValidation {
                message: format!("Metadata validation failed: {:?}", validation_result.errors)
            });
        }

        Ok(metadata)
    }

    /// 生成元数据Schema
    pub fn generate_metadata_schema(&self) -> JsonValue {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Plugin Metadata Schema",
            "type": "object",
            "required": self.validation_rules.required_fields,
            "properties": {
                "id": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9_-]+$",
                    "minLength": 1,
                    "maxLength": 100,
                    "description": "Unique plugin identifier"
                },
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 200,
                    "description": "Human-readable plugin name"
                },
                "version": {
                    "type": "string",
                    "pattern": "^\\d+\\.\\d+\\.\\d+",
                    "description": "Plugin version in semantic versioning format"
                },
                "description": {
                    "type": "string",
                    "minLength": self.validation_rules.min_description_length,
                    "maxLength": 1000,
                    "description": "Plugin description"
                },
                "author": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 100,
                    "description": "Plugin author"
                },
                "plugin_type": {
                    "type": "string",
                    "enum": [
                        "DataSource",
                        "Strategy", 
                        "RiskManager",
                        "Execution",
                        "Analytics",
                        "Notification",
                        "Utility"
                    ],
                    "description": "Plugin type category"
                },
                "capabilities": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "List of plugin capabilities"
                },
                "dependencies": {
                    "type": "array",
                    "maxItems": self.validation_rules.max_dependencies,
                    "items": {
                        "type": "object",
                        "required": ["plugin_id", "version_req"],
                        "properties": {
                            "plugin_id": {
                                "type": "string"
                            },
                            "version_req": {
                                "type": "string"
                            },
                            "optional": {
                                "type": "boolean",
                                "default": false
                            }
                        }
                    },
                    "description": "Plugin dependencies"
                },
                "min_framework_version": {
                    "type": "string",
                    "pattern": "^\\d+\\.\\d+\\.\\d+",
                    "description": "Minimum required framework version"
                },
                "max_framework_version": {
                    "type": ["string", "null"],
                    "pattern": "^\\d+\\.\\d+\\.\\d+",
                    "description": "Maximum supported framework version"
                },
                "config_schema": {
                    "type": ["object", "null"],
                    "description": "JSON Schema for plugin configuration"
                },
                "tags": {
                    "type": "array",
                    "maxItems": self.validation_rules.max_tags,
                    "items": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    },
                    "description": "Plugin tags for categorization"
                }
            },
            "additionalProperties": false
        })
    }

    /// 创建元数据模板
    pub fn create_template(&self, template_type: PluginType) -> MetadataTemplate {
        let (name, description, defaults) = match template_type {
            PluginType::DataSource => (
                "Data Source Template".to_string(),
                "Template for data source plugins".to_string(),
                serde_json::json!({
                    "plugin_type": "DataSource",
                    "capabilities": ["RealTimeData", "HistoricalData"],
                    "tags": ["data", "source"]
                })
            ),
            PluginType::Strategy => (
                "Strategy Template".to_string(),
                "Template for trading strategy plugins".to_string(),
                serde_json::json!({
                    "plugin_type": "Strategy",
                    "capabilities": ["SignalGeneration"],
                    "tags": ["strategy", "trading"]
                })
            ),
            PluginType::RiskManager => (
                "Risk Manager Template".to_string(),
                "Template for risk management plugins".to_string(),
                serde_json::json!({
                    "plugin_type": "RiskManager",
                    "capabilities": ["RiskCalculation"],
                    "tags": ["risk", "management"]
                })
            ),
            _ => (
                "Generic Template".to_string(),
                "Generic plugin template".to_string(),
                serde_json::json!({
                    "plugin_type": format!("{:?}", template_type),
                    "capabilities": [],
                    "tags": []
                })
            ),
        };

        MetadataTemplate {
            name,
            description,
            defaults: defaults.as_object().unwrap().clone(),
            optional_fields: vec![
                "max_framework_version".to_string(),
                "config_schema".to_string(),
            ],
            field_descriptions: [
                ("id".to_string(), "Unique identifier for the plugin".to_string()),
                ("name".to_string(), "Display name of the plugin".to_string()),
                ("version".to_string(), "Plugin version (semantic versioning)".to_string()),
                ("description".to_string(), "Detailed description of plugin functionality".to_string()),
                ("author".to_string(), "Plugin author or organization".to_string()),
                ("capabilities".to_string(), "List of capabilities provided by the plugin".to_string()),
                ("dependencies".to_string(), "Other plugins this plugin depends on".to_string()),
                ("tags".to_string(), "Tags for categorization and discovery".to_string()),
            ].iter().cloned().collect(),
            examples: serde_json::json!({
                "id": "my_awesome_plugin",
                "name": "My Awesome Plugin",
                "version": "1.0.0",
                "description": "This plugin provides awesome functionality for trading",
                "author": "John Doe <john@example.com>",
                "tags": ["awesome", "trading", "utility"]
            }).as_object().unwrap().clone(),
        }
    }

    // 私有验证方法

    fn validate_basic_fields(
        &self,
        metadata: &PluginMetadata,
        errors: &mut Vec<ValidationIssue>,
        warnings: &mut Vec<ValidationIssue>,
    ) {
        // ID验证
        if metadata.id.is_empty() {
            errors.push(ValidationIssue {
                issue_type: IssueType::MissingRequiredField,
                field_path: "id".to_string(),
                message: "Plugin ID cannot be empty".to_string(),
                severity: IssueSeverity::Critical,
                fix_suggestion: Some("Provide a unique plugin identifier".to_string()),
            });
        } else if !metadata.id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            errors.push(ValidationIssue {
                issue_type: IssueType::InvalidFieldFormat,
                field_path: "id".to_string(),
                message: "Plugin ID contains invalid characters".to_string(),
                severity: IssueSeverity::Error,
                fix_suggestion: Some("Use only alphanumeric characters, underscores, and hyphens".to_string()),
            });
        }

        // 名称验证
        if metadata.name.is_empty() {
            errors.push(ValidationIssue {
                issue_type: IssueType::MissingRequiredField,
                field_path: "name".to_string(),
                message: "Plugin name cannot be empty".to_string(),
                severity: IssueSeverity::Error,
                fix_suggestion: Some("Provide a descriptive plugin name".to_string()),
            });
        }

        // 描述验证
        if metadata.description.len() < self.validation_rules.min_description_length {
            warnings.push(ValidationIssue {
                issue_type: IssueType::InvalidFieldValue,
                field_path: "description".to_string(),
                message: format!("Description is too short (minimum {} characters)", 
                                self.validation_rules.min_description_length),
                severity: IssueSeverity::Warning,
                fix_suggestion: Some("Provide a more detailed description".to_string()),
            });
        }

        // 作者验证
        if metadata.author.is_empty() {
            warnings.push(ValidationIssue {
                issue_type: IssueType::MissingRequiredField,
                field_path: "author".to_string(),
                message: "Author field is empty".to_string(),
                severity: IssueSeverity::Warning,
                fix_suggestion: Some("Specify the plugin author".to_string()),
            });
        }
    }

    fn validate_version(
        &self,
        metadata: &PluginMetadata,
        errors: &mut Vec<ValidationIssue>,
        _warnings: &mut Vec<ValidationIssue>,
    ) {
        if self.validation_rules.version_format == VersionFormat::Semantic {
            // 语义化版本验证已在类型级别完成（使用semver::Version）
            // 这里可以添加额外的版本规则检查
            if metadata.version.major == 0 && metadata.version.minor == 0 && metadata.version.patch == 0 {
                errors.push(ValidationIssue {
                    issue_type: IssueType::VersionIssue,
                    field_path: "version".to_string(),
                    message: "Version 0.0.0 is not recommended".to_string(),
                    severity: IssueSeverity::Warning,
                    fix_suggestion: Some("Use a meaningful version number".to_string()),
                });
            }
        }

        // 框架版本兼容性检查
        if let Some(ref max_version) = metadata.max_framework_version {
            if max_version < &metadata.min_framework_version {
                errors.push(ValidationIssue {
                    issue_type: IssueType::VersionIssue,
                    field_path: "max_framework_version".to_string(),
                    message: "Maximum framework version is less than minimum version".to_string(),
                    severity: IssueSeverity::Error,
                    fix_suggestion: Some("Ensure max_framework_version >= min_framework_version".to_string()),
                });
            }
        }
    }

    fn validate_dependencies(
        &self,
        metadata: &PluginMetadata,
        errors: &mut Vec<ValidationIssue>,
        warnings: &mut Vec<ValidationIssue>,
    ) {
        if metadata.dependencies.len() > self.validation_rules.max_dependencies {
            errors.push(ValidationIssue {
                issue_type: IssueType::DependencyIssue,
                field_path: "dependencies".to_string(),
                message: format!("Too many dependencies (maximum {})", 
                               self.validation_rules.max_dependencies),
                severity: IssueSeverity::Error,
                fix_suggestion: Some("Reduce the number of dependencies".to_string()),
            });
        }

        // 检查循环依赖（简化版本）
        for dep in &metadata.dependencies {
            if dep.plugin_id == metadata.id {
                errors.push(ValidationIssue {
                    issue_type: IssueType::DependencyIssue,
                    field_path: "dependencies".to_string(),
                    message: "Plugin cannot depend on itself".to_string(),
                    severity: IssueSeverity::Critical,
                    fix_suggestion: Some("Remove self-dependency".to_string()),
                });
            }

            // 版本要求格式验证
            if semver::VersionReq::parse(&dep.version_req).is_err() {
                warnings.push(ValidationIssue {
                    issue_type: IssueType::InvalidFieldFormat,
                    field_path: format!("dependencies[{}].version_req", dep.plugin_id),
                    message: "Invalid version requirement format".to_string(),
                    severity: IssueSeverity::Warning,
                    fix_suggestion: Some("Use valid semver requirement format (e.g., '^1.0', '>=2.0.0')".to_string()),
                });
            }
        }
    }

    fn validate_capabilities(
        &self,
        metadata: &PluginMetadata,
        warnings: &mut Vec<ValidationIssue>,
    ) {
        if metadata.capabilities.is_empty() {
            warnings.push(ValidationIssue {
                issue_type: IssueType::InvalidFieldValue,
                field_path: "capabilities".to_string(),
                message: "Plugin has no declared capabilities".to_string(),
                severity: IssueSeverity::Warning,
                fix_suggestion: Some("Declare at least one capability".to_string()),
            });
        }

        // 检查能力与插件类型的匹配度
        let expected_capabilities = match metadata.plugin_type {
            PluginType::DataSource => vec![PluginCapability::RealTimeData, PluginCapability::HistoricalData],
            PluginType::Strategy => vec![PluginCapability::RealTimeData],
            PluginType::RiskManager => vec![PluginCapability::RiskCalculation],
            _ => vec![],
        };

        if !expected_capabilities.is_empty() {
            let has_expected = expected_capabilities.iter()
                .any(|cap| metadata.capabilities.contains(cap));
            
            if !has_expected {
                warnings.push(ValidationIssue {
                    issue_type: IssueType::InvalidFieldValue,
                    field_path: "capabilities".to_string(),
                    message: format!("Plugin type {:?} typically requires specific capabilities", metadata.plugin_type),
                    severity: IssueSeverity::Info,
                    fix_suggestion: Some(format!("Consider adding capabilities: {:?}", expected_capabilities)),
                });
            }
        }
    }

    fn validate_tags(
        &self,
        metadata: &PluginMetadata,
        errors: &mut Vec<ValidationIssue>,
        warnings: &mut Vec<ValidationIssue>,
    ) {
        if metadata.tags.len() > self.validation_rules.max_tags {
            errors.push(ValidationIssue {
                issue_type: IssueType::InvalidFieldValue,
                field_path: "tags".to_string(),
                message: format!("Too many tags (maximum {})", self.validation_rules.max_tags),
                severity: IssueSeverity::Error,
                fix_suggestion: Some("Reduce the number of tags".to_string()),
            });
        }

        for (i, tag) in metadata.tags.iter().enumerate() {
            if tag.is_empty() {
                warnings.push(ValidationIssue {
                    issue_type: IssueType::InvalidFieldValue,
                    field_path: format!("tags[{}]", i),
                    message: "Empty tag".to_string(),
                    severity: IssueSeverity::Warning,
                    fix_suggestion: Some("Remove empty tags".to_string()),
                });
            } else if !tag.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                warnings.push(ValidationIssue {
                    issue_type: IssueType::InvalidFieldFormat,
                    field_path: format!("tags[{}]", i),
                    message: "Tag contains invalid characters".to_string(),
                    severity: IssueSeverity::Warning,
                    fix_suggestion: Some("Use only alphanumeric characters, underscores, and hyphens in tags".to_string()),
                });
            }
        }
    }

    fn generate_suggestions(&self, metadata: &PluginMetadata, suggestions: &mut Vec<String>) {
        // 建议添加标签
        if metadata.tags.is_empty() {
            suggestions.push("Consider adding tags to improve plugin discoverability".to_string());
        }

        // 建议添加配置Schema
        if metadata.config_schema.is_none() {
            suggestions.push("Consider adding a config_schema to validate plugin configuration".to_string());
        }

        // 建议版本策略
        if metadata.version.major == 0 {
            suggestions.push("Consider releasing a stable 1.0.0 version when the plugin is ready for production".to_string());
        }

        // 建议文档改进
        if metadata.description.len() < 50 {
            suggestions.push("Consider providing a more detailed description for better user understanding".to_string());
        }
    }
}

/// 序列化格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    Json,
    Yaml,
    Toml,
}

impl MetadataBuilder {
    /// 创建新的元数据构建器
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: String::new(),
                name: String::new(),
                version: Version::new(0, 1, 0),
                description: String::new(),
                author: String::new(),
                plugin_type: PluginType::Utility,
                capabilities: Vec::new(),
                dependencies: Vec::new(),
                min_framework_version: Version::new(2, 0, 0),
                max_framework_version: None,
                config_schema: None,
                tags: Vec::new(),
            },
            validation_enabled: true,
        }
    }

    /// 设置插件ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.metadata.id = id.into();
        self
    }

    /// 设置插件名称
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.metadata.name = name.into();
        self
    }

    /// 设置版本
    pub fn version(mut self, version: Version) -> Self {
        self.metadata.version = version;
        self
    }

    /// 设置描述
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    /// 设置作者
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = author.into();
        self
    }

    /// 设置插件类型
    pub fn plugin_type(mut self, plugin_type: PluginType) -> Self {
        self.metadata.plugin_type = plugin_type;
        self
    }

    /// 添加能力
    pub fn capability(mut self, capability: PluginCapability) -> Self {
        if !self.metadata.capabilities.contains(&capability) {
            self.metadata.capabilities.push(capability);
        }
        self
    }

    /// 添加依赖
    pub fn dependency(mut self, plugin_id: impl Into<String>, version_req: impl Into<String>) -> Self {
        self.metadata.dependencies.push(PluginDependency {
            plugin_id: plugin_id.into(),
            version_req: version_req.into(),
            optional: false,
        });
        self
    }

    /// 添加可选依赖
    pub fn optional_dependency(mut self, plugin_id: impl Into<String>, version_req: impl Into<String>) -> Self {
        self.metadata.dependencies.push(PluginDependency {
            plugin_id: plugin_id.into(),
            version_req: version_req.into(),
            optional: true,
        });
        self
    }

    /// 添加标签
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        let tag = tag.into();
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
        self
    }

    /// 设置配置Schema
    pub fn config_schema(mut self, schema: serde_json::Value) -> Self {
        self.metadata.config_schema = Some(schema);
        self
    }

    /// 设置最小框架版本
    pub fn min_framework_version(mut self, version: Version) -> Self {
        self.metadata.min_framework_version = version;
        self
    }

    /// 设置最大框架版本
    pub fn max_framework_version(mut self, version: Version) -> Self {
        self.metadata.max_framework_version = Some(version);
        self
    }

    /// 禁用验证
    pub fn disable_validation(mut self) -> Self {
        self.validation_enabled = false;
        self
    }

    /// 构建元数据
    pub fn build(self) -> Result<PluginMetadata> {
        if self.validation_enabled {
            let manager = MetadataManager::new(ValidationRules::default());
            let validation_result = manager.validate_metadata(&self.metadata);
            
            if !validation_result.valid {
                return Err(MosesQuantError::ConfigValidation {
                    message: format!("Metadata validation failed: {:?}", validation_result.errors)
                });
            }
        }
        
        Ok(self.metadata)
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_validation() {
        let rules = ValidationRules::default();
        let manager = MetadataManager::new(rules);

        // 创建有效的元数据
        let valid_metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            description: "This is a test plugin with sufficient description length".to_string(),
            author: "Test Author".to_string(),
            plugin_type: PluginType::Utility,
            capabilities: vec![PluginCapability::Custom("test".to_string())],
            dependencies: vec![],
            min_framework_version: Version::new(2, 0, 0),
            max_framework_version: None,
            config_schema: None,
            tags: vec!["test".to_string()],
        };

        let result = manager.validate_metadata(&valid_metadata);
        assert!(result.valid);
        assert!(result.errors.is_empty());

        // 创建无效的元数据
        let invalid_metadata = PluginMetadata {
            id: "".to_string(), // 空ID
            name: "".to_string(), // 空名称
            description: "short".to_string(), // 描述太短
            ..valid_metadata.clone()
        };

        let result = manager.validate_metadata(&invalid_metadata);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_metadata_serialization() {
        let manager = MetadataManager::new(ValidationRules::default());
        
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: Version::new(1, 0, 0),
            description: "This is a test plugin".to_string(),
            author: "Test Author".to_string(),
            plugin_type: PluginType::Utility,
            capabilities: vec![],
            dependencies: vec![],
            min_framework_version: Version::new(2, 0, 0),
            max_framework_version: None,
            config_schema: None,
            tags: vec![],
        };

        // JSON序列化和反序列化
        let json_str = manager.serialize_metadata(&metadata, SerializationFormat::Json).unwrap();
        let deserialized = manager.deserialize_metadata(&json_str, SerializationFormat::Json).unwrap();
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.version, deserialized.version);

        // YAML序列化和反序列化
        let yaml_str = manager.serialize_metadata(&metadata, SerializationFormat::Yaml).unwrap();
        let deserialized = manager.deserialize_metadata(&yaml_str, SerializationFormat::Yaml).unwrap();
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.version, deserialized.version);
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = MetadataBuilder::new()
            .id("awesome_plugin")
            .name("Awesome Plugin")
            .version(Version::new(1, 2, 3))
            .description("This is an awesome plugin that does amazing things in the trading system")
            .author("John Doe")
            .plugin_type(PluginType::Strategy)
            .capability(PluginCapability::RealTimeData)
            .capability(PluginCapability::MachineLearning)
            .dependency("data_source_plugin", "^1.0")
            .optional_dependency("analytics_plugin", ">=2.0")
            .tag("strategy")
            .tag("machine-learning")
            .build()
            .unwrap();

        assert_eq!(metadata.id, "awesome_plugin");
        assert_eq!(metadata.name, "Awesome Plugin");
        assert_eq!(metadata.version, Version::new(1, 2, 3));
        assert_eq!(metadata.plugin_type, PluginType::Strategy);
        assert_eq!(metadata.capabilities.len(), 2);
        assert_eq!(metadata.dependencies.len(), 2);
        assert_eq!(metadata.tags.len(), 2);
        assert!(!metadata.dependencies[1].optional); // 第一个依赖不是可选的
        assert!(metadata.dependencies[1].optional); // 第二个依赖是可选的
    }

    #[test]
    fn test_schema_generation() {
        let manager = MetadataManager::new(ValidationRules::default());
        let schema = manager.generate_metadata_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
        
        // 验证生成的Schema本身是有效的
        let validation_result = manager.validate_config_schema(&schema);
        assert!(validation_result.is_ok());
    }

    #[test]
    fn test_template_creation() {
        let manager = MetadataManager::new(ValidationRules::default());
        
        let template = manager.create_template(PluginType::DataSource);
        assert_eq!(template.name, "Data Source Template");
        assert!(template.defaults.contains_key("plugin_type"));
        assert!(template.defaults.contains_key("capabilities"));
        
        let template = manager.create_template(PluginType::Strategy);
        assert_eq!(template.name, "Strategy Template");
        assert!(template.defaults.contains_key("plugin_type"));
    }

    #[test]
    fn test_config_schema_validation() {
        let mut manager = MetadataManager::new(ValidationRules::default());
        
        // 有效的JSON Schema
        let valid_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "api_key": {
                    "type": "string"
                },
                "timeout": {
                    "type": "number",
                    "default": 30
                }
            },
            "required": ["api_key"]
        });
        
        assert!(manager.validate_config_schema(&valid_schema).is_ok());
        
        // 验证配置是否符合Schema
        let valid_config = serde_json::json!({
            "api_key": "secret_key",
            "timeout": 60
        });
        
        let errors = manager.validate_config_against_schema(&valid_config, &valid_schema).unwrap();
        assert!(errors.is_empty());
        
        let invalid_config = serde_json::json!({
            "timeout": 60
            // 缺少必需的api_key
        });
        
        let errors = manager.validate_config_against_schema(&invalid_config, &valid_schema).unwrap();
        assert!(!errors.is_empty());
    }
}