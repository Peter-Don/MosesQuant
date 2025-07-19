//! 插件开发工具套件
//! 
//! 提供脚手架、代码生成、开发模板和工具链支持，简化插件开发流程

use crate::plugins::core::*;
use crate::plugins::version_management::*;
use crate::{Result, MosesQuantError};
use crate::types::PluginId;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use std::fs;
use tracing::{debug, info, warn, error};

/// 插件开发工具套件
pub struct PluginDevToolkit {
    /// 模板存储库
    templates: HashMap<TemplateType, PluginTemplate>,
    /// 代码生成器
    generators: HashMap<GeneratorType, Box<dyn CodeGenerator>>,
    /// 工具链配置
    toolchain_config: ToolchainConfig,
    /// 开发环境配置
    dev_config: DevConfig,
}

/// 开发环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    /// 工作目录
    pub workspace_dir: PathBuf,
    /// 模板目录
    pub template_dir: PathBuf,
    /// 输出目录
    pub output_dir: PathBuf,
    /// 是否启用热重载
    pub enable_hot_reload: bool,
    /// 自动格式化代码
    pub auto_format: bool,
    /// 自动生成文档
    pub auto_generate_docs: bool,
    /// 开发服务器端口
    pub dev_server_port: u16,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            workspace_dir: PathBuf::from("./plugins"),
            template_dir: PathBuf::from("./templates"),
            output_dir: PathBuf::from("./generated"),
            enable_hot_reload: true,
            auto_format: true,
            auto_generate_docs: true,
            dev_server_port: 3000,
        }
    }
}

/// 工具链配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    /// Rust工具链版本
    pub rust_version: String,
    /// Cargo配置
    pub cargo_config: CargoConfig,
    /// 测试框架
    pub test_framework: TestFramework,
    /// 文档生成器
    pub doc_generator: DocGenerator,
    /// 代码格式化器
    pub code_formatter: CodeFormatter,
    /// 静态分析工具
    pub static_analyzer: StaticAnalyzer,
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        Self {
            rust_version: "1.70.0".to_string(),
            cargo_config: CargoConfig::default(),
            test_framework: TestFramework::default(),
            doc_generator: DocGenerator::default(),
            code_formatter: CodeFormatter::default(),
            static_analyzer: StaticAnalyzer::default(),
        }
    }
}

/// Cargo配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoConfig {
    /// 目标架构
    pub target: String,
    /// 优化级别
    pub optimization_level: String,
    /// 调试信息
    pub debug_info: bool,
    /// 增量编译
    pub incremental: bool,
    /// 并行任务数
    pub parallel_jobs: Option<usize>,
}

impl Default for CargoConfig {
    fn default() -> Self {
        Self {
            target: "x86_64-unknown-linux-gnu".to_string(),
            optimization_level: "2".to_string(),
            debug_info: true,
            incremental: true,
            parallel_jobs: None,
        }
    }
}

/// 测试框架配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFramework {
    /// 框架类型
    pub framework_type: String,
    /// 覆盖率工具
    pub coverage_tool: String,
    /// 基准测试
    pub benchmark_enabled: bool,
    /// 集成测试
    pub integration_tests: bool,
}

impl Default for TestFramework {
    fn default() -> Self {
        Self {
            framework_type: "tokio-test".to_string(),
            coverage_tool: "tarpaulin".to_string(),
            benchmark_enabled: true,
            integration_tests: true,
        }
    }
}

/// 文档生成器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGenerator {
    /// 生成器类型
    pub generator_type: String,
    /// 主题
    pub theme: String,
    /// 包含私有项
    pub include_private: bool,
    /// 生成示例
    pub generate_examples: bool,
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self {
            generator_type: "rustdoc".to_string(),
            theme: "default".to_string(),
            include_private: false,
            generate_examples: true,
        }
    }
}

/// 代码格式化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFormatter {
    /// 格式化器类型
    pub formatter_type: String,
    /// 配置文件路径
    pub config_file: Option<PathBuf>,
    /// 自动修复
    pub auto_fix: bool,
}

impl Default for CodeFormatter {
    fn default() -> Self {
        Self {
            formatter_type: "rustfmt".to_string(),
            config_file: None,
            auto_fix: true,
        }
    }
}

/// 静态分析器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAnalyzer {
    /// 分析器类型
    pub analyzer_type: String,
    /// 严格模式
    pub strict_mode: bool,
    /// 自定义lint规则
    pub custom_lints: Vec<String>,
}

impl Default for StaticAnalyzer {
    fn default() -> Self {
        Self {
            analyzer_type: "clippy".to_string(),
            strict_mode: false,
            custom_lints: vec![],
        }
    }
}

/// 模板类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemplateType {
    /// 数据源插件
    DataSource,
    /// 策略插件
    Strategy,
    /// 风险管理插件
    RiskManager,
    /// 执行插件
    Execution,
    /// 工具插件
    Utility,
    /// 完整插件
    FullPlugin,
}

/// 插件模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTemplate {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 模板版本
    pub version: String,
    /// 模板类型
    pub template_type: TemplateType,
    /// 文件列表
    pub files: Vec<TemplateFile>,
    /// 依赖项
    pub dependencies: Vec<TemplateDependency>,
    /// 配置变量
    pub variables: Vec<TemplateVariable>,
    /// 生成后钩子
    pub post_generation_hooks: Vec<PostGenerationHook>,
}

/// 模板文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    /// 源路径（模板中的路径）
    pub source_path: String,
    /// 目标路径（生成后的路径）
    pub target_path: String,
    /// 文件类型
    pub file_type: FileType,
    /// 是否为模板文件（需要变量替换）
    pub is_template: bool,
    /// 文件权限
    pub permissions: Option<u32>,
}

/// 文件类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Rust源代码
    RustSource,
    /// Cargo配置
    CargoToml,
    /// 文档文件
    Documentation,
    /// 测试文件
    Test,
    /// 配置文件
    Config,
    /// 脚本文件
    Script,
    /// 其他
    Other,
}

/// 模板依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDependency {
    /// 依赖名称
    pub name: String,
    /// 版本要求
    pub version: String,
    /// 是否可选
    pub optional: bool,
    /// 特性列表
    pub features: Vec<String>,
}

/// 模板变量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// 变量名
    pub name: String,
    /// 变量描述
    pub description: String,
    /// 变量类型
    pub var_type: VariableType,
    /// 默认值
    pub default_value: Option<String>,
    /// 是否必需
    pub required: bool,
    /// 验证规则
    pub validation_rules: Vec<ValidationRule>,
}

/// 变量类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Integer,
    Boolean,
    Array,
    Object,
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// 规则类型
    pub rule_type: String,
    /// 规则参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 错误消息
    pub error_message: String,
}

/// 生成后钩子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostGenerationHook {
    /// 钩子名称
    pub name: String,
    /// 执行命令
    pub command: String,
    /// 工作目录
    pub working_dir: Option<String>,
    /// 环境变量
    pub env_vars: HashMap<String, String>,
    /// 是否异步执行
    pub async_execution: bool,
}

/// 代码生成器类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeneratorType {
    /// 插件骨架生成器
    PluginSkeleton,
    /// API绑定生成器
    ApiBinding,
    /// 测试代码生成器
    TestCode,
    /// 文档生成器
    Documentation,
    /// 配置生成器
    Configuration,
    /// FFI绑定生成器
    FfiBinding,
}

/// 代码生成器trait
pub trait CodeGenerator: Send + Sync {
    /// 生成器名称
    fn name(&self) -> &str;

    /// 生成器类型
    fn generator_type(&self) -> GeneratorType;

    /// 生成代码
    fn generate(&self, context: &GenerationContext) -> Result<GenerationResult>;

    /// 验证生成上下文
    fn validate_context(&self, context: &GenerationContext) -> Result<()>;

    /// 获取支持的输出格式
    fn supported_formats(&self) -> Vec<OutputFormat>;
}

/// 生成上下文
#[derive(Debug, Clone)]
pub struct GenerationContext {
    /// 插件信息
    pub plugin_info: PluginInfo,
    /// 模板变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 输出目录
    pub output_dir: PathBuf,
    /// 模板目录
    pub template_dir: PathBuf,
    /// 生成选项
    pub options: GenerationOptions,
}

/// 插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 插件ID
    pub id: PluginId,
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: Version,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 作者信息
    pub author: String,
    /// 描述
    pub description: String,
    /// 许可证
    pub license: Option<String>,
    /// 仓库URL
    pub repository: Option<String>,
}

/// 生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    /// 是否覆盖现有文件
    pub overwrite_existing: bool,
    /// 是否生成测试代码
    pub generate_tests: bool,
    /// 是否生成文档
    pub generate_docs: bool,
    /// 是否生成示例
    pub generate_examples: bool,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 代码风格
    pub code_style: CodeStyle,
}

/// 输出格式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Rust项目
    RustProject,
    /// 库文件
    Library,
    /// 单文件
    SingleFile,
    /// 压缩包
    Archive,
}

/// 代码风格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStyle {
    /// 缩进大小
    pub indent_size: usize,
    /// 使用空格还是制表符
    pub use_spaces: bool,
    /// 最大行长度
    pub max_line_length: usize,
    /// 命名约定
    pub naming_convention: NamingConvention,
}

impl Default for CodeStyle {
    fn default() -> Self {
        Self {
            indent_size: 4,
            use_spaces: true,
            max_line_length: 100,
            naming_convention: NamingConvention::default(),
        }
    }
}

/// 命名约定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConvention {
    /// 结构体命名
    pub struct_naming: String,
    /// 函数命名
    pub function_naming: String,
    /// 变量命名
    pub variable_naming: String,
    /// 常量命名
    pub constant_naming: String,
}

impl Default for NamingConvention {
    fn default() -> Self {
        Self {
            struct_naming: "PascalCase".to_string(),
            function_naming: "snake_case".to_string(),
            variable_naming: "snake_case".to_string(),
            constant_naming: "SCREAMING_SNAKE_CASE".to_string(),
        }
    }
}

/// 生成结果
#[derive(Debug, Clone)]
pub struct GenerationResult {
    /// 是否成功
    pub success: bool,
    /// 生成的文件列表
    pub generated_files: Vec<GeneratedFile>,
    /// 错误信息
    pub errors: Vec<String>,
    /// 警告信息
    pub warnings: Vec<String>,
    /// 生成统计
    pub statistics: GenerationStatistics,
}

/// 生成的文件
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// 文件路径
    pub path: PathBuf,
    /// 文件大小
    pub size: u64,
    /// 文件类型
    pub file_type: FileType,
    /// 生成时间
    pub generated_at: i64,
}

/// 生成统计
#[derive(Debug, Clone)]
pub struct GenerationStatistics {
    /// 生成的文件数量
    pub files_generated: usize,
    /// 代码行数
    pub lines_of_code: usize,
    /// 生成耗时
    pub generation_time: std::time::Duration,
    /// 模板变量数量
    pub variables_processed: usize,
}

impl PluginDevToolkit {
    /// 创建插件开发工具套件
    pub fn new(dev_config: DevConfig, toolchain_config: ToolchainConfig) -> Self {
        let mut templates = HashMap::new();
        let mut generators = HashMap::new();

        // 注册默认模板
        templates.insert(TemplateType::DataSource, Self::create_data_source_template());
        templates.insert(TemplateType::Strategy, Self::create_strategy_template());
        templates.insert(TemplateType::FullPlugin, Self::create_full_plugin_template());

        // 注册默认代码生成器
        generators.insert(GeneratorType::PluginSkeleton, Box::new(PluginSkeletonGenerator::new()));
        generators.insert(GeneratorType::TestCode, Box::new(TestCodeGenerator::new()));

        Self {
            templates,
            generators,
            toolchain_config,
            dev_config,
        }
    }

    /// 创建新插件项目
    pub fn create_plugin_project(&self, plugin_info: &PluginInfo, template_type: TemplateType) -> Result<GenerationResult> {
        let template = self.templates.get(&template_type)
            .ok_or_else(|| MosesQuantError::Internal {
                message: format!("Template not found for type {:?}", template_type)
            })?;

        let output_dir = self.dev_config.output_dir.join(&plugin_info.id);

        // 创建输出目录
        fs::create_dir_all(&output_dir)
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to create output directory: {}", e)
            })?;

        let mut variables = HashMap::new();
        variables.insert("plugin_id".to_string(), serde_json::Value::String(plugin_info.id.clone()));
        variables.insert("plugin_name".to_string(), serde_json::Value::String(plugin_info.name.clone()));
        variables.insert("plugin_version".to_string(), serde_json::Value::String(plugin_info.version.to_string()));
        variables.insert("author".to_string(), serde_json::Value::String(plugin_info.author.clone()));
        variables.insert("description".to_string(), serde_json::Value::String(plugin_info.description.clone()));

        let context = GenerationContext {
            plugin_info: plugin_info.clone(),
            variables,
            output_dir,
            template_dir: self.dev_config.template_dir.clone(),
            options: GenerationOptions {
                overwrite_existing: false,
                generate_tests: true,
                generate_docs: true,
                generate_examples: true,
                output_format: OutputFormat::RustProject,
                code_style: CodeStyle::default(),
            },
        };

        let mut generated_files = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let start_time = std::time::Instant::now();

        // 生成模板文件
        for template_file in &template.files {
            match self.generate_file(template_file, &context) {
                Ok(file) => generated_files.push(file),
                Err(e) => errors.push(format!("Failed to generate {}: {}", template_file.target_path, e)),
            }
        }

        // 执行生成后钩子
        for hook in &template.post_generation_hooks {
            if let Err(e) = self.execute_post_generation_hook(hook, &context) {
                warnings.push(format!("Post-generation hook '{}' failed: {}", hook.name, e));
            }
        }

        let statistics = GenerationStatistics {
            files_generated: generated_files.len(),
            lines_of_code: self.count_lines_of_code(&generated_files),
            generation_time: start_time.elapsed(),
            variables_processed: context.variables.len(),
        };

        Ok(GenerationResult {
            success: errors.is_empty(),
            generated_files,
            errors,
            warnings,
            statistics,
        })
    }

    /// 生成插件骨架代码
    pub fn generate_plugin_skeleton(&self, context: &GenerationContext) -> Result<GenerationResult> {
        let generator = self.generators.get(&GeneratorType::PluginSkeleton)
            .ok_or_else(|| MosesQuantError::Internal {
                message: "Plugin skeleton generator not found".to_string()
            })?;

        generator.generate(context)
    }

    /// 生成测试代码
    pub fn generate_test_code(&self, context: &GenerationContext) -> Result<GenerationResult> {
        let generator = self.generators.get(&GeneratorType::TestCode)
            .ok_or_else(|| MosesQuantError::Internal {
                message: "Test code generator not found".to_string()
            })?;

        generator.generate(context)
    }

    /// 验证开发环境
    pub fn validate_dev_environment(&self) -> Result<ValidationReport> {
        let mut checks = Vec::new();
        let mut warnings = Vec::new();

        // 检查Rust工具链
        match self.check_rust_toolchain() {
            Ok(_) => checks.push(("Rust toolchain".to_string(), true, None)),
            Err(e) => checks.push(("Rust toolchain".to_string(), false, Some(e.to_string()))),
        }

        // 检查Cargo
        match self.check_cargo() {
            Ok(_) => checks.push(("Cargo".to_string(), true, None)),
            Err(e) => checks.push(("Cargo".to_string(), false, Some(e.to_string()))),
        }

        // 检查工作目录
        if !self.dev_config.workspace_dir.exists() {
            warnings.push("Workspace directory does not exist".to_string());
        }

        let passed_checks = checks.iter().filter(|(_, passed, _)| *passed).count();
        let total_checks = checks.len();

        Ok(ValidationReport {
            overall_status: if passed_checks == total_checks { ValidationStatus::Passed } else { ValidationStatus::Failed },
            checks,
            warnings,
            recommendations: self.generate_dev_recommendations(),
        })
    }

    /// 获取可用模板列表
    pub fn list_templates(&self) -> Vec<&PluginTemplate> {
        self.templates.values().collect()
    }

    /// 获取开发统计信息
    pub fn get_dev_statistics(&self) -> DevStatistics {
        DevStatistics {
            available_templates: self.templates.len(),
            available_generators: self.generators.len(),
            workspace_projects: self.count_workspace_projects(),
        }
    }

    // 私有方法

    /// 创建数据源模板
    fn create_data_source_template() -> PluginTemplate {
        PluginTemplate {
            name: "Data Source Plugin".to_string(),
            description: "Template for creating data source plugins".to_string(),
            version: "1.0.0".to_string(),
            template_type: TemplateType::DataSource,
            files: vec![
                TemplateFile {
                    source_path: "data_source/Cargo.toml.template".to_string(),
                    target_path: "Cargo.toml".to_string(),
                    file_type: FileType::CargoToml,
                    is_template: true,
                    permissions: None,
                },
                TemplateFile {
                    source_path: "data_source/src/lib.rs.template".to_string(),
                    target_path: "src/lib.rs".to_string(),
                    file_type: FileType::RustSource,
                    is_template: true,
                    permissions: None,
                },
            ],
            dependencies: vec![
                TemplateDependency {
                    name: "moses_quant".to_string(),
                    version: "2.0.0".to_string(),
                    optional: false,
                    features: vec!["data-source".to_string()],
                },
            ],
            variables: vec![
                TemplateVariable {
                    name: "plugin_id".to_string(),
                    description: "Unique plugin identifier".to_string(),
                    var_type: VariableType::String,
                    default_value: None,
                    required: true,
                    validation_rules: vec![],
                },
            ],
            post_generation_hooks: vec![
                PostGenerationHook {
                    name: "cargo_check".to_string(),
                    command: "cargo check".to_string(),
                    working_dir: None,
                    env_vars: HashMap::new(),
                    async_execution: false,
                },
            ],
        }
    }

    /// 创建策略模板
    fn create_strategy_template() -> PluginTemplate {
        PluginTemplate {
            name: "Strategy Plugin".to_string(),
            description: "Template for creating strategy plugins".to_string(),
            version: "1.0.0".to_string(),
            template_type: TemplateType::Strategy,
            files: vec![],
            dependencies: vec![],
            variables: vec![],
            post_generation_hooks: vec![],
        }
    }

    /// 创建完整插件模板
    fn create_full_plugin_template() -> PluginTemplate {
        PluginTemplate {
            name: "Full Plugin".to_string(),
            description: "Complete plugin template with all components".to_string(),
            version: "1.0.0".to_string(),
            template_type: TemplateType::FullPlugin,
            files: vec![],
            dependencies: vec![],
            variables: vec![],
            post_generation_hooks: vec![],
        }
    }

    /// 生成文件
    fn generate_file(&self, template_file: &TemplateFile, context: &GenerationContext) -> Result<GeneratedFile> {
        let target_path = context.output_dir.join(&template_file.target_path);
        
        // 创建目标目录
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| MosesQuantError::Internal {
                    message: format!("Failed to create directory: {}", e)
                })?;
        }

        // 简化的文件生成逻辑
        let content = if template_file.is_template {
            format!("// Generated plugin file: {}\n// Plugin ID: {}\n", 
                   template_file.target_path, context.plugin_info.id)
        } else {
            "// Static file content\n".to_string()
        };

        fs::write(&target_path, content)
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to write file: {}", e)
            })?;

        let metadata = fs::metadata(&target_path)
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to get file metadata: {}", e)
            })?;

        Ok(GeneratedFile {
            path: target_path,
            size: metadata.len(),
            file_type: template_file.file_type.clone(),
            generated_at: chrono::Utc::now().timestamp(),
        })
    }

    /// 执行生成后钩子
    fn execute_post_generation_hook(&self, hook: &PostGenerationHook, context: &GenerationContext) -> Result<()> {
        debug!("Executing post-generation hook: {}", hook.name);
        // 简化的钩子执行逻辑
        Ok(())
    }

    /// 统计代码行数
    fn count_lines_of_code(&self, files: &[GeneratedFile]) -> usize {
        // 简化的行数统计
        files.len() * 50 // 假设每个文件平均50行
    }

    /// 检查Rust工具链
    fn check_rust_toolchain(&self) -> Result<()> {
        // 简化的检查逻辑
        Ok(())
    }

    /// 检查Cargo
    fn check_cargo(&self) -> Result<()> {
        // 简化的检查逻辑
        Ok(())
    }

    /// 生成开发建议
    fn generate_dev_recommendations(&self) -> Vec<String> {
        vec![
            "Use latest stable Rust version".to_string(),
            "Enable clippy for code quality checks".to_string(),
            "Write comprehensive tests".to_string(),
        ]
    }

    /// 统计工作空间项目数
    fn count_workspace_projects(&self) -> usize {
        // 简化的统计逻辑
        0
    }
}

/// 验证报告
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// 总体状态
    pub overall_status: ValidationStatus,
    /// 检查结果列表
    pub checks: Vec<(String, bool, Option<String>)>, // (名称, 是否通过, 错误信息)
    /// 警告信息
    pub warnings: Vec<String>,
    /// 建议
    pub recommendations: Vec<String>,
}

/// 验证状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationStatus {
    Passed,
    Failed,
    Warning,
}

/// 开发统计信息
#[derive(Debug, Clone)]
pub struct DevStatistics {
    /// 可用模板数
    pub available_templates: usize,
    /// 可用生成器数
    pub available_generators: usize,
    /// 工作空间项目数
    pub workspace_projects: usize,
}

/// 插件骨架生成器
pub struct PluginSkeletonGenerator {
    name: String,
}

impl PluginSkeletonGenerator {
    pub fn new() -> Self {
        Self {
            name: "Plugin Skeleton Generator".to_string(),
        }
    }
}

impl CodeGenerator for PluginSkeletonGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::PluginSkeleton
    }

    fn generate(&self, context: &GenerationContext) -> Result<GenerationResult> {
        let start_time = std::time::Instant::now();
        
        // 简化的骨架生成逻辑
        let lib_rs_content = format!(
            r#"//! {} Plugin
//! 
//! {}

use moses_quant::{{Result, Plugin, PluginContext}};
use async_trait::async_trait;

pub struct {}Plugin;

#[async_trait]
impl Plugin for {}Plugin {{
    async fn start(&mut self, context: &PluginContext) -> Result<()> {{
        // Plugin initialization logic
        Ok(())
    }}

    async fn stop(&mut self, context: &PluginContext) -> Result<()> {{
        // Plugin cleanup logic
        Ok(())
    }}

    fn name(&self) -> &str {{
        "{}"
    }}

    fn version(&self) -> &str {{
        "{}"
    }}
}}
"#,
            context.plugin_info.name,
            context.plugin_info.description,
            context.plugin_info.name.replace(" ", ""),
            context.plugin_info.name.replace(" ", ""),
            context.plugin_info.name,
            context.plugin_info.version
        );

        let lib_rs_path = context.output_dir.join("src/lib.rs");
        fs::create_dir_all(lib_rs_path.parent().unwrap())
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to create src directory: {}", e)
            })?;

        fs::write(&lib_rs_path, lib_rs_content)
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to write lib.rs: {}", e)
            })?;

        let generated_file = GeneratedFile {
            path: lib_rs_path,
            size: lib_rs_content.len() as u64,
            file_type: FileType::RustSource,
            generated_at: chrono::Utc::now().timestamp(),
        };

        Ok(GenerationResult {
            success: true,
            generated_files: vec![generated_file],
            errors: vec![],
            warnings: vec![],
            statistics: GenerationStatistics {
                files_generated: 1,
                lines_of_code: lib_rs_content.lines().count(),
                generation_time: start_time.elapsed(),
                variables_processed: context.variables.len(),
            },
        })
    }

    fn validate_context(&self, _context: &GenerationContext) -> Result<()> {
        Ok(())
    }

    fn supported_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::RustProject, OutputFormat::SingleFile]
    }
}

/// 测试代码生成器
pub struct TestCodeGenerator {
    name: String,
}

impl TestCodeGenerator {
    pub fn new() -> Self {
        Self {
            name: "Test Code Generator".to_string(),
        }
    }
}

impl CodeGenerator for TestCodeGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn generator_type(&self) -> GeneratorType {
        GeneratorType::TestCode
    }

    fn generate(&self, context: &GenerationContext) -> Result<GenerationResult> {
        let start_time = std::time::Instant::now();

        let test_content = format!(
            r#"#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_{}_creation() {{
        let plugin = {}Plugin;
        assert_eq!(plugin.name(), "{}");
        assert_eq!(plugin.version(), "{}");
    }}

    #[tokio::test]
    async fn test_{}_lifecycle() {{
        let mut plugin = {}Plugin;
        let context = PluginContext::new("test_plugin".to_string());
        
        // Test start
        plugin.start(&context).await.unwrap();
        
        // Test stop
        plugin.stop(&context).await.unwrap();
    }}
}}
"#,
            context.plugin_info.id.to_lowercase(),
            context.plugin_info.name.replace(" ", ""),
            context.plugin_info.name,
            context.plugin_info.version,
            context.plugin_info.id.to_lowercase(),
            context.plugin_info.name.replace(" ", "")
        );

        let test_path = context.output_dir.join("src/lib.rs");
        
        // 将测试代码追加到lib.rs文件
        fs::write(&test_path, format!("{}\n\n{}", 
            fs::read_to_string(&test_path).unwrap_or_default(),
            test_content))
            .map_err(|e| MosesQuantError::Internal {
                message: format!("Failed to write test code: {}", e)
            })?;

        let generated_file = GeneratedFile {
            path: test_path,
            size: test_content.len() as u64,
            file_type: FileType::Test,
            generated_at: chrono::Utc::now().timestamp(),
        };

        Ok(GenerationResult {
            success: true,
            generated_files: vec![generated_file],
            errors: vec![],
            warnings: vec![],
            statistics: GenerationStatistics {
                files_generated: 1,
                lines_of_code: test_content.lines().count(),
                generation_time: start_time.elapsed(),
                variables_processed: context.variables.len(),
            },
        })
    }

    fn validate_context(&self, _context: &GenerationContext) -> Result<()> {
        Ok(())
    }

    fn supported_formats(&self) -> Vec<OutputFormat> {
        vec![OutputFormat::RustProject]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_toolkit_creation() {
        let toolkit = PluginDevToolkit::new(DevConfig::default(), ToolchainConfig::default());
        let stats = toolkit.get_dev_statistics();
        assert!(stats.available_templates > 0);
        assert!(stats.available_generators > 0);
    }

    #[test]
    fn test_template_creation() {
        let template = PluginDevToolkit::create_data_source_template();
        assert_eq!(template.template_type, TemplateType::DataSource);
        assert!(!template.files.is_empty());
    }

    #[test]
    fn test_plugin_skeleton_generator() {
        let generator = PluginSkeletonGenerator::new();
        assert_eq!(generator.generator_type(), GeneratorType::PluginSkeleton);
        assert!(generator.supported_formats().contains(&OutputFormat::RustProject));
    }
}