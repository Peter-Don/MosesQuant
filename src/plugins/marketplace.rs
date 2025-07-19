//! 插件市场平台原型
//! 
//! 提供插件分发、发现、安装和管理功能，构建完整的插件生态系统

use crate::plugins::core::*;
use crate::plugins::version_management::*;
use crate::plugins::quality_assurance::*;
use crate::{Result, MosesQuantError};
use crate::types::PluginId;
use std::collections::{HashMap, BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use tracing::{debug, info, warn, error};
use std::time::{Duration, Instant};

/// 插件市场管理器
pub struct PluginMarketplace {
    /// 插件注册表
    registry: Arc<RwLock<HashMap<PluginId, MarketplaceEntry>>>,
    /// 分类系统
    categories: Arc<RwLock<BTreeMap<CategoryId, Category>>>,
    /// 用户管理
    user_manager: Arc<UserManager>,
    /// 发布管理
    publisher_manager: Arc<PublisherManager>,
    /// 下载管理
    download_manager: Arc<DownloadManager>,
    /// 评级系统
    rating_system: Arc<RatingSystem>,
    /// 搜索引擎
    search_engine: Arc<SearchEngine>,
    /// 市场配置
    config: MarketplaceConfig,
    /// 统计数据
    statistics: Arc<RwLock<MarketplaceStatistics>>,
}

/// 市场配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    /// 是否启用用户认证
    pub enable_authentication: bool,
    /// 是否启用付费插件
    pub enable_paid_plugins: bool,
    /// 最大文件大小(MB)
    pub max_file_size: u64,
    /// 支持的文件格式
    pub supported_formats: Vec<String>,
    /// 审核模式
    pub review_mode: ReviewMode,
    /// 缓存配置
    pub cache_config: CacheConfig,
    /// API限制
    pub rate_limits: RateLimits,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            enable_authentication: true,
            enable_paid_plugins: false,
            max_file_size: 100, // 100MB
            supported_formats: vec!["tar.gz".to_string(), "zip".to_string()],
            review_mode: ReviewMode::Automatic,
            cache_config: CacheConfig {
                enabled: true,
                key: "marketplace".to_string(),
                paths: vec!["./cache/marketplace".to_string()],
                restore_keys: vec![],
                ttl: Duration::from_secs(3600),
            },
            rate_limits: RateLimits {
                requests_per_minute: 60,
                downloads_per_hour: 100,
                uploads_per_day: 10,
            },
        }
    }
}

/// 审核模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewMode {
    /// 自动审核
    Automatic,
    /// 手动审核
    Manual,
    /// 混合模式
    Hybrid,
}

/// API限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// 每分钟请求数
    pub requests_per_minute: u32,
    /// 每小时下载数
    pub downloads_per_hour: u32,
    /// 每天上传数
    pub uploads_per_day: u32,
}

/// 市场条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// 插件ID
    pub plugin_id: PluginId,
    /// 条目ID
    pub entry_id: String,
    /// 插件信息
    pub plugin_info: PluginInfo,
    /// 发布者信息
    pub publisher: PublisherInfo,
    /// 版本历史
    pub versions: Vec<PluginVersionInfo>,
    /// 分类
    pub categories: Vec<CategoryId>,
    /// 标签
    pub tags: Vec<String>,
    /// 许可证
    pub license: LicenseInfo,
    /// 定价信息
    pub pricing: PricingInfo,
    /// 评级信息
    pub rating: RatingInfo,
    /// 下载统计
    pub download_stats: DownloadStats,
    /// 发布状态
    pub status: PublishStatus,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 插件信息（扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 插件ID
    pub id: PluginId,
    /// 名称
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 描述
    pub description: String,
    /// 详细描述
    pub long_description: String,
    /// 版本
    pub version: Version,
    /// 作者
    pub author: String,
    /// 主页
    pub homepage: Option<String>,
    /// 仓库地址
    pub repository: Option<String>,
    /// 文档地址
    pub documentation: Option<String>,
    /// 截图
    pub screenshots: Vec<String>,
    /// 演示视频
    pub demo_videos: Vec<String>,
    /// 依赖
    pub dependencies: Vec<DependencyInfo>,
    /// 支持的平台
    pub supported_platforms: Vec<Platform>,
    /// 最小框架版本
    pub min_framework_version: Version,
}

/// 依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// 依赖插件ID
    pub plugin_id: PluginId,
    /// 版本要求
    pub version_requirement: String,
    /// 是否可选
    pub optional: bool,
}

/// 支持平台
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
    Android,
    iOS,
    Web,
}

/// 发布者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    /// 发布者ID
    pub publisher_id: String,
    /// 名称
    pub name: String,
    /// 邮箱
    pub email: String,
    /// 网站
    pub website: Option<String>,
    /// 验证状态
    pub verified: bool,
    /// 声誉评分
    pub reputation_score: f64,
    /// 发布数量
    pub published_count: u32,
}

/// 插件版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersionInfo {
    /// 版本号
    pub version: Version,
    /// 发布时间
    pub release_date: i64,
    /// 变更日志
    pub changelog: String,
    /// 下载URL
    pub download_url: String,
    /// 文件大小
    pub file_size: u64,
    /// 校验和
    pub checksum: String,
    /// 签名
    pub signature: Option<String>,
    /// 发布状态
    pub status: VersionStatus,
}

/// 分类ID
pub type CategoryId = String;

/// 分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    /// 分类ID
    pub id: CategoryId,
    /// 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 父分类
    pub parent: Option<CategoryId>,
    /// 子分类
    pub children: Vec<CategoryId>,
    /// 图标
    pub icon: Option<String>,
    /// 排序权重
    pub sort_order: i32,
    /// 插件数量
    pub plugin_count: u32,
}

/// 许可证信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    /// 许可证类型
    pub license_type: LicenseType,
    /// 许可证文本
    pub license_text: Option<String>,
    /// 许可证URL
    pub license_url: Option<String>,
}

/// 许可证类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseType {
    MIT,
    Apache2,
    GPL3,
    BSD3,
    Commercial,
    Proprietary,
    Custom(String),
}

/// 定价信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    /// 价格模型
    pub pricing_model: PricingModel,
    /// 价格
    pub price: f64,
    /// 货币
    pub currency: String,
    /// 试用期(天)
    pub trial_period: Option<u32>,
    /// 订阅周期
    pub subscription_period: Option<SubscriptionPeriod>,
}

/// 价格模型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingModel {
    /// 免费
    Free,
    /// 一次性购买
    OneTime,
    /// 订阅
    Subscription,
    /// 按使用量付费
    PayPerUse,
    /// 企业定制
    Enterprise,
}

/// 订阅周期
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionPeriod {
    Monthly,
    Quarterly,
    Yearly,
}

/// 评级信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingInfo {
    /// 平均评分 (1-5)
    pub average_rating: f64,
    /// 评分数量
    pub rating_count: u32,
    /// 评分分布
    pub rating_distribution: HashMap<u8, u32>, // 星级 -> 数量
    /// 最新评论
    pub recent_reviews: Vec<ReviewSummary>,
}

/// 评论摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSummary {
    /// 评论ID
    pub review_id: String,
    /// 用户名
    pub username: String,
    /// 评分
    pub rating: u8,
    /// 评论摘要
    pub summary: String,
    /// 评论时间
    pub created_at: i64,
}

/// 下载统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStats {
    /// 总下载量
    pub total_downloads: u64,
    /// 本月下载量
    pub monthly_downloads: u64,
    /// 本周下载量
    pub weekly_downloads: u64,
    /// 今日下载量
    pub daily_downloads: u64,
    /// 下载趋势
    pub download_trend: Vec<DownloadDataPoint>,
}

/// 下载数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadDataPoint {
    /// 日期
    pub date: String,
    /// 下载量
    pub downloads: u32,
}

/// 发布状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishStatus {
    /// 草稿
    Draft,
    /// 待审核
    PendingReview,
    /// 已发布
    Published,
    /// 已下架
    Unpublished,
    /// 被拒绝
    Rejected,
    /// 已暂停
    Suspended,
}

/// 用户管理器
pub struct UserManager {
    /// 用户信息
    users: Arc<RwLock<HashMap<UserId, UserInfo>>>,
    /// 用户会话
    sessions: Arc<RwLock<HashMap<SessionId, UserSession>>>,
}

/// 用户ID
pub type UserId = String;
/// 会话ID
pub type SessionId = String;

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// 用户ID
    pub user_id: UserId,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 头像
    pub avatar: Option<String>,
    /// 注册时间
    pub registered_at: i64,
    /// 最后登录时间
    pub last_login: Option<i64>,
    /// 用户角色
    pub role: UserRole,
    /// 个人资料
    pub profile: UserProfile,
    /// 偏好设置
    pub preferences: UserPreferences,
}

/// 用户角色
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// 普通用户
    User,
    /// 开发者
    Developer,
    /// 发布者
    Publisher,
    /// 管理员
    Admin,
    /// 超级管理员
    SuperAdmin,
}

/// 用户资料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// 显示名称
    pub display_name: String,
    /// 个人简介
    pub bio: Option<String>,
    /// 所在地
    pub location: Option<String>,
    /// 网站
    pub website: Option<String>,
    /// 社交媒体链接
    pub social_links: HashMap<String, String>,
}

/// 用户偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// 语言
    pub language: String,
    /// 时区
    pub timezone: String,
    /// 通知设置
    pub notifications: NotificationPreferences,
    /// 隐私设置
    pub privacy: PrivacySettings,
}

/// 通知偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    /// 邮件通知
    pub email_notifications: bool,
    /// 应用内通知
    pub in_app_notifications: bool,
    /// 更新通知
    pub update_notifications: bool,
    /// 安全通知
    pub security_notifications: bool,
}

/// 隐私设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// 公开个人资料
    pub public_profile: bool,
    /// 显示邮箱
    pub show_email: bool,
    /// 数据收集
    pub allow_analytics: bool,
}

/// 用户会话
#[derive(Debug, Clone)]
pub struct UserSession {
    /// 会话ID
    pub session_id: SessionId,
    /// 用户ID
    pub user_id: UserId,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间
    pub expires_at: Instant,
    /// IP地址
    pub ip_address: String,
    /// 用户代理
    pub user_agent: String,
}

/// 发布管理器
pub struct PublisherManager {
    /// 发布请求
    publish_requests: Arc<RwLock<HashMap<String, PublishRequest>>>,
    /// 审核队列
    review_queue: Arc<RwLock<VecDeque<String>>>,
}

/// 发布请求
#[derive(Debug, Clone)]
pub struct PublishRequest {
    /// 请求ID
    pub request_id: String,
    /// 插件包
    pub plugin_package: PluginPackage,
    /// 发布者ID
    pub publisher_id: String,
    /// 请求状态
    pub status: PublishRequestStatus,
    /// 审核结果
    pub review_result: Option<ReviewResult>,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 发布请求状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishRequestStatus {
    /// 提交中
    Submitting,
    /// 待审核
    PendingReview,
    /// 审核中
    InReview,
    /// 审核通过
    Approved,
    /// 审核拒绝
    Rejected,
    /// 已发布
    Published,
}

/// 审核结果
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// 审核员ID
    pub reviewer_id: String,
    /// 审核决定
    pub decision: ReviewDecision,
    /// 审核评论
    pub comments: String,
    /// 质量评分
    pub quality_score: f64,
    /// 安全评分
    pub security_score: f64,
    /// 审核时间
    pub reviewed_at: i64,
}

/// 审核决定
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewDecision {
    /// 批准
    Approve,
    /// 拒绝
    Reject,
    /// 需要修改
    RequestChanges,
}

/// 下载管理器
pub struct DownloadManager {
    /// 下载记录
    downloads: Arc<RwLock<Vec<DownloadRecord>>>,
    /// 活跃下载
    active_downloads: Arc<RwLock<HashMap<String, DownloadSession>>>,
}

/// 下载记录
#[derive(Debug, Clone)]
pub struct DownloadRecord {
    /// 下载ID
    pub download_id: String,
    /// 插件ID
    pub plugin_id: PluginId,
    /// 版本
    pub version: Version,
    /// 用户ID
    pub user_id: Option<UserId>,
    /// IP地址
    pub ip_address: String,
    /// 用户代理
    pub user_agent: String,
    /// 下载时间
    pub downloaded_at: i64,
    /// 文件大小
    pub file_size: u64,
    /// 下载来源
    pub source: DownloadSource,
}

/// 下载来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSource {
    /// 网页
    Web,
    /// API
    Api,
    /// CLI工具
    Cli,
    /// IDE插件
    Ide,
}

/// 下载会话
#[derive(Debug, Clone)]
pub struct DownloadSession {
    /// 会话ID
    pub session_id: String,
    /// 开始时间
    pub start_time: Instant,
    /// 进度百分比
    pub progress: f64,
    /// 下载速度(字节/秒)
    pub speed: u64,
}

/// 评级系统
pub struct RatingSystem {
    /// 评级记录
    ratings: Arc<RwLock<HashMap<PluginId, Vec<Rating>>>>,
    /// 评论记录
    reviews: Arc<RwLock<HashMap<String, Review>>>,
}

/// 评级
#[derive(Debug, Clone)]
pub struct Rating {
    /// 评级ID
    pub rating_id: String,
    /// 插件ID
    pub plugin_id: PluginId,
    /// 用户ID
    pub user_id: UserId,
    /// 评分 (1-5)
    pub score: u8,
    /// 创建时间
    pub created_at: i64,
    /// 版本
    pub version: Option<Version>,
}

/// 评论
#[derive(Debug, Clone)]
pub struct Review {
    /// 评论ID
    pub review_id: String,
    /// 插件ID
    pub plugin_id: PluginId,
    /// 用户ID
    pub user_id: UserId,
    /// 评分
    pub rating: u8,
    /// 标题
    pub title: String,
    /// 评论内容
    pub content: String,
    /// 优点
    pub pros: Vec<String>,
    /// 缺点
    pub cons: Vec<String>,
    /// 推荐度
    pub recommend: bool,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: Option<i64>,
    /// 有用投票数
    pub helpful_votes: u32,
    /// 总投票数
    pub total_votes: u32,
}

/// 搜索引擎
pub struct SearchEngine {
    /// 搜索索引
    search_index: Arc<RwLock<SearchIndex>>,
    /// 搜索历史
    search_history: Arc<RwLock<Vec<SearchQuery>>>,
}

/// 搜索索引
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// 文本索引
    pub text_index: HashMap<String, HashSet<PluginId>>,
    /// 分类索引
    pub category_index: HashMap<CategoryId, HashSet<PluginId>>,
    /// 标签索引
    pub tag_index: HashMap<String, HashSet<PluginId>>,
    /// 作者索引
    pub author_index: HashMap<String, HashSet<PluginId>>,
}

/// 搜索查询
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// 查询ID
    pub query_id: String,
    /// 查询文本
    pub query_text: String,
    /// 搜索参数
    pub parameters: SearchParameters,
    /// 用户ID
    pub user_id: Option<UserId>,
    /// IP地址
    pub ip_address: String,
    /// 查询时间
    pub queried_at: i64,
    /// 结果数量
    pub result_count: u32,
}

/// 搜索参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParameters {
    /// 查询文本
    pub query: String,
    /// 分类过滤
    pub categories: Vec<CategoryId>,
    /// 标签过滤
    pub tags: Vec<String>,
    /// 作者过滤
    pub authors: Vec<String>,
    /// 许可证过滤
    pub licenses: Vec<LicenseType>,
    /// 价格过滤
    pub pricing: Option<PriceRange>,
    /// 评分过滤
    pub min_rating: Option<f64>,
    /// 排序方式
    pub sort_by: SortOption,
    /// 排序顺序
    pub sort_order: SortOrder,
    /// 页面大小
    pub page_size: u32,
    /// 页面偏移
    pub page_offset: u32,
}

/// 价格范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRange {
    /// 最小价格
    pub min_price: f64,
    /// 最大价格
    pub max_price: f64,
    /// 货币
    pub currency: String,
}

/// 排序选项
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOption {
    /// 相关性
    Relevance,
    /// 下载量
    Downloads,
    /// 评分
    Rating,
    /// 更新时间
    UpdatedAt,
    /// 创建时间
    CreatedAt,
    /// 名称
    Name,
    /// 价格
    Price,
}

/// 排序顺序
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    /// 升序
    Ascending,
    /// 降序
    Descending,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 查询文本
    pub query: String,
    /// 结果列表
    pub results: Vec<SearchResultItem>,
    /// 总结果数
    pub total_count: u32,
    /// 分页信息
    pub pagination: PaginationInfo,
    /// 搜索耗时
    pub search_time: Duration,
    /// 建议查询
    pub suggestions: Vec<String>,
    /// 分面统计
    pub facets: SearchFacets,
}

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// 市场条目
    pub entry: MarketplaceEntry,
    /// 相关性评分
    pub relevance_score: f64,
    /// 高亮文本
    pub highlights: Vec<String>,
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// 当前页
    pub current_page: u32,
    /// 页面大小
    pub page_size: u32,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

/// 搜索分面
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    /// 分类统计
    pub categories: HashMap<CategoryId, u32>,
    /// 标签统计
    pub tags: HashMap<String, u32>,
    /// 作者统计
    pub authors: HashMap<String, u32>,
    /// 许可证统计
    pub licenses: HashMap<LicenseType, u32>,
    /// 价格范围统计
    pub price_ranges: HashMap<String, u32>,
}

/// 市场统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStatistics {
    /// 总插件数
    pub total_plugins: u32,
    /// 已发布插件数
    pub published_plugins: u32,
    /// 总用户数
    pub total_users: u32,
    /// 活跃用户数
    pub active_users: u32,
    /// 总下载量
    pub total_downloads: u64,
    /// 今日下载量
    pub daily_downloads: u32,
    /// 总发布者数
    pub total_publishers: u32,
    /// 验证发布者数
    pub verified_publishers: u32,
    /// 平均评分
    pub average_rating: f64,
    /// 最受欢迎插件
    pub popular_plugins: Vec<PluginId>,
    /// 最新插件
    pub recent_plugins: Vec<PluginId>,
}

impl PluginMarketplace {
    /// 创建插件市场
    pub fn new(config: MarketplaceConfig) -> Self {
        let user_manager = Arc::new(UserManager {
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let publisher_manager = Arc::new(PublisherManager {
            publish_requests: Arc::new(RwLock::new(HashMap::new())),
            review_queue: Arc::new(RwLock::new(VecDeque::new())),
        });

        let download_manager = Arc::new(DownloadManager {
            downloads: Arc::new(RwLock::new(Vec::new())),
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
        });

        let rating_system = Arc::new(RatingSystem {
            ratings: Arc::new(RwLock::new(HashMap::new())),
            reviews: Arc::new(RwLock::new(HashMap::new())),
        });

        let search_engine = Arc::new(SearchEngine {
            search_index: Arc::new(RwLock::new(SearchIndex {
                text_index: HashMap::new(),
                category_index: HashMap::new(),
                tag_index: HashMap::new(),
                author_index: HashMap::new(),
            })),
            search_history: Arc::new(RwLock::new(Vec::new())),
        });

        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(BTreeMap::new())),
            user_manager,
            publisher_manager,
            download_manager,
            rating_system,
            search_engine,
            config,
            statistics: Arc::new(RwLock::new(MarketplaceStatistics {
                total_plugins: 0,
                published_plugins: 0,
                total_users: 0,
                active_users: 0,
                total_downloads: 0,
                daily_downloads: 0,
                total_publishers: 0,
                verified_publishers: 0,
                average_rating: 0.0,
                popular_plugins: vec![],
                recent_plugins: vec![],
            })),
        }
    }

    /// 发布插件
    pub async fn publish_plugin(&self, plugin_info: PluginInfo, publisher_id: String, package_data: Vec<u8>) -> Result<String> {
        // 验证发布者权限
        if !self.verify_publisher_permission(&publisher_id).await? {
            return Err(MosesQuantError::Internal {
                message: "Publisher not authorized".to_string()
            });
        }

        // 验证插件包
        self.validate_plugin_package(&plugin_info, &package_data).await?;

        // 创建发布请求
        let request_id = uuid::Uuid::new_v4().to_string();
        let publish_request = PublishRequest {
            request_id: request_id.clone(),
            plugin_package: PluginPackage {
                metadata: PluginMetadata {
                    id: plugin_info.id.clone(),
                    name: plugin_info.name.clone(),
                    version: plugin_info.version.clone(),
                    description: plugin_info.description.clone(),
                    author: plugin_info.author.clone(),
                    plugin_type: PluginType::Utility, // 简化处理
                    capabilities: vec![],
                    dependencies: vec![],
                    min_framework_version: plugin_info.min_framework_version.clone(),
                    tags: vec![],
                },
                path: PathBuf::from(""),
                checksum: "".to_string(),
                size: package_data.len() as u64,
                created_at: chrono::Utc::now().timestamp(),
                signature: None,
            },
            publisher_id,
            status: PublishRequestStatus::Submitting,
            review_result: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        // 添加到发布队列
        {
            let mut requests = self.publisher_manager.publish_requests.write().await;
            requests.insert(request_id.clone(), publish_request);
        }

        // 根据审核模式处理
        match self.config.review_mode {
            ReviewMode::Automatic => {
                self.auto_review_plugin(&request_id).await?;
            }
            ReviewMode::Manual => {
                self.queue_for_manual_review(&request_id).await?;
            }
            ReviewMode::Hybrid => {
                if self.should_auto_review(&plugin_info).await {
                    self.auto_review_plugin(&request_id).await?;
                } else {
                    self.queue_for_manual_review(&request_id).await?;
                }
            }
        }

        info!("Plugin publish request created: {}", request_id);
        Ok(request_id)
    }

    /// 搜索插件
    pub async fn search_plugins(&self, parameters: SearchParameters) -> Result<SearchResult> {
        let start_time = Instant::now();
        
        // 记录搜索查询
        let query_id = uuid::Uuid::new_v4().to_string();
        let search_query = SearchQuery {
            query_id: query_id.clone(),
            query_text: parameters.query.clone(),
            parameters: parameters.clone(),
            user_id: None, // 简化处理
            ip_address: "127.0.0.1".to_string(),
            queried_at: chrono::Utc::now().timestamp(),
            result_count: 0,
        };

        {
            let mut history = self.search_engine.search_history.write().await;
            history.push(search_query);
        }

        // 执行搜索
        let registry = self.registry.read().await;
        let mut results = Vec::new();

        for entry in registry.values() {
            if entry.status == PublishStatus::Published {
                let relevance_score = self.calculate_relevance_score(entry, &parameters);
                if relevance_score > 0.0 {
                    results.push(SearchResultItem {
                        entry: entry.clone(),
                        relevance_score,
                        highlights: self.generate_highlights(entry, &parameters.query),
                    });
                }
            }
        }

        // 排序结果
        self.sort_search_results(&mut results, &parameters);

        // 分页
        let total_count = results.len() as u32;
        let start_index = (parameters.page_offset * parameters.page_size) as usize;
        let end_index = std::cmp::min(start_index + parameters.page_size as usize, results.len());
        let paged_results = if start_index < results.len() {
            results[start_index..end_index].to_vec()
        } else {
            vec![]
        };

        let pagination = PaginationInfo {
            current_page: parameters.page_offset + 1,
            page_size: parameters.page_size,
            total_pages: (total_count + parameters.page_size - 1) / parameters.page_size,
            has_next: end_index < results.len(),
            has_prev: parameters.page_offset > 0,
        };

        // 生成分面统计
        let facets = self.generate_search_facets(&results).await;

        Ok(SearchResult {
            query: parameters.query,
            results: paged_results,
            total_count,
            pagination,
            search_time: start_time.elapsed(),
            suggestions: self.generate_search_suggestions(&parameters.query).await,
            facets,
        })
    }

    /// 安装插件
    pub async fn install_plugin(&self, plugin_id: &PluginId, version: Option<Version>, user_id: Option<UserId>) -> Result<String> {
        // 查找插件
        let entry = {
            let registry = self.registry.read().await;
            registry.get(plugin_id).cloned()
                .ok_or_else(|| MosesQuantError::Internal {
                    message: format!("Plugin not found: {}", plugin_id)
                })?
        };

        if entry.status != PublishStatus::Published {
            return Err(MosesQuantError::Internal {
                message: "Plugin is not available for installation".to_string()
            });
        }

        // 确定安装版本
        let target_version = version.unwrap_or_else(|| {
            entry.versions.iter()
                .filter(|v| v.status == VersionStatus::Stable)
                .max_by(|a, b| a.version.cmp(&b.version))
                .map(|v| v.version.clone())
                .unwrap_or(entry.plugin_info.version.clone())
        });

        // 生成下载ID
        let download_id = uuid::Uuid::new_v4().to_string();

        // 记录下载
        let download_record = DownloadRecord {
            download_id: download_id.clone(),
            plugin_id: plugin_id.clone(),
            version: target_version,
            user_id,
            ip_address: "127.0.0.1".to_string(),
            user_agent: "MosesQuant CLI".to_string(),
            downloaded_at: chrono::Utc::now().timestamp(),
            file_size: entry.versions.first().map(|v| v.file_size).unwrap_or(0),
            source: DownloadSource::Api,
        };

        {
            let mut downloads = self.download_manager.downloads.write().await;
            downloads.push(download_record);
        }

        // 更新下载统计
        self.update_download_statistics(plugin_id).await;

        info!("Plugin installation started: {} (download_id: {})", plugin_id, download_id);
        Ok(download_id)
    }

    /// 提交评分和评论
    pub async fn submit_review(&self, plugin_id: &PluginId, user_id: UserId, rating: u8, review_content: Option<String>) -> Result<String> {
        if rating < 1 || rating > 5 {
            return Err(MosesQuantError::Internal {
                message: "Rating must be between 1 and 5".to_string()
            });
        }

        let review_id = uuid::Uuid::new_v4().to_string();

        // 创建评级记录
        let rating_record = Rating {
            rating_id: review_id.clone(),
            plugin_id: plugin_id.clone(),
            user_id: user_id.clone(),
            score: rating,
            created_at: chrono::Utc::now().timestamp(),
            version: None,
        };

        {
            let mut ratings = self.rating_system.ratings.write().await;
            ratings.entry(plugin_id.clone()).or_insert_with(Vec::new).push(rating_record);
        }

        // 如果有评论内容，创建评论记录
        if let Some(content) = review_content {
            let review = Review {
                review_id: review_id.clone(),
                plugin_id: plugin_id.clone(),
                user_id,
                rating,
                title: "User Review".to_string(),
                content,
                pros: vec![],
                cons: vec![],
                recommend: rating >= 4,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: None,
                helpful_votes: 0,
                total_votes: 0,
            };

            let mut reviews = self.rating_system.reviews.write().await;
            reviews.insert(review_id.clone(), review);
        }

        // 更新插件评级信息
        self.update_plugin_rating(plugin_id).await;

        info!("Review submitted for plugin {}: rating {}", plugin_id, rating);
        Ok(review_id)
    }

    /// 获取市场统计信息
    pub async fn get_statistics(&self) -> MarketplaceStatistics {
        let mut stats = self.statistics.write().await;
        
        // 更新统计数据
        let registry = self.registry.read().await;
        stats.total_plugins = registry.len() as u32;
        stats.published_plugins = registry.values()
            .filter(|entry| entry.status == PublishStatus::Published)
            .count() as u32;

        let downloads = self.download_manager.downloads.read().await;
        stats.total_downloads = downloads.len() as u64;

        let today = chrono::Utc::now().date_naive();
        stats.daily_downloads = downloads.iter()
            .filter(|record| {
                let record_date = chrono::DateTime::from_timestamp(record.downloaded_at, 0)
                    .map(|dt| dt.date_naive())
                    .unwrap_or_default();
                record_date == today
            })
            .count() as u32;

        // 计算平均评分
        let ratings = self.rating_system.ratings.read().await;
        let total_ratings: u32 = ratings.values().map(|r| r.len() as u32).sum();
        let total_score: u32 = ratings.values()
            .flat_map(|r| r.iter())
            .map(|rating| rating.score as u32)
            .sum();
        
        stats.average_rating = if total_ratings > 0 {
            total_score as f64 / total_ratings as f64
        } else {
            0.0
        };

        stats.clone()
    }

    // 私有方法

    /// 验证发布者权限
    async fn verify_publisher_permission(&self, publisher_id: &str) -> Result<bool> {
        // 简化的权限验证
        debug!("Verifying publisher permission for: {}", publisher_id);
        Ok(true)
    }

    /// 验证插件包
    async fn validate_plugin_package(&self, plugin_info: &PluginInfo, package_data: &[u8]) -> Result<()> {
        // 检查文件大小
        if package_data.len() > (self.config.max_file_size * 1024 * 1024) as usize {
            return Err(MosesQuantError::Internal {
                message: "Package size exceeds maximum limit".to_string()
            });
        }

        // 验证插件信息
        if plugin_info.name.is_empty() || plugin_info.description.is_empty() {
            return Err(MosesQuantError::Internal {
                message: "Plugin name and description are required".to_string()
            });
        }

        Ok(())
    }

    /// 自动审核插件
    async fn auto_review_plugin(&self, request_id: &str) -> Result<()> {
        debug!("Auto-reviewing plugin request: {}", request_id);
        
        // 简化的自动审核逻辑
        let review_result = ReviewResult {
            reviewer_id: "system".to_string(),
            decision: ReviewDecision::Approve,
            comments: "Automated review passed".to_string(),
            quality_score: 0.8,
            security_score: 0.9,
            reviewed_at: chrono::Utc::now().timestamp(),
        };

        // 更新发布请求状态
        let mut requests = self.publisher_manager.publish_requests.write().await;
        if let Some(request) = requests.get_mut(request_id) {
            request.status = PublishRequestStatus::Approved;
            request.review_result = Some(review_result);
            request.updated_at = chrono::Utc::now().timestamp();
        }

        Ok(())
    }

    /// 加入手动审核队列
    async fn queue_for_manual_review(&self, request_id: &str) -> Result<()> {
        let mut queue = self.publisher_manager.review_queue.write().await;
        queue.push_back(request_id.to_string());
        
        let mut requests = self.publisher_manager.publish_requests.write().await;
        if let Some(request) = requests.get_mut(request_id) {
            request.status = PublishRequestStatus::PendingReview;
            request.updated_at = chrono::Utc::now().timestamp();
        }

        debug!("Plugin request queued for manual review: {}", request_id);
        Ok(())
    }

    /// 判断是否应该自动审核
    async fn should_auto_review(&self, plugin_info: &PluginInfo) -> bool {
        // 简化的判断逻辑
        plugin_info.name.len() > 5 && plugin_info.description.len() > 20
    }

    /// 计算相关性评分
    fn calculate_relevance_score(&self, entry: &MarketplaceEntry, parameters: &SearchParameters) -> f64 {
        let mut score = 0.0;
        let query_lower = parameters.query.to_lowercase();

        // 名称匹配
        if entry.plugin_info.name.to_lowercase().contains(&query_lower) {
            score += 10.0;
        }

        // 描述匹配
        if entry.plugin_info.description.to_lowercase().contains(&query_lower) {
            score += 5.0;
        }

        // 标签匹配
        for tag in &entry.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 3.0;
            }
        }

        // 分类匹配
        if !parameters.categories.is_empty() {
            for category in &parameters.categories {
                if entry.categories.contains(category) {
                    score += 8.0;
                }
            }
        }

        // 评分加成
        score += entry.rating.average_rating;

        // 下载量加成
        score += (entry.download_stats.total_downloads as f64).log10();

        score
    }

    /// 生成高亮文本
    fn generate_highlights(&self, entry: &MarketplaceEntry, query: &str) -> Vec<String> {
        let mut highlights = Vec::new();
        let query_lower = query.to_lowercase();

        if entry.plugin_info.name.to_lowercase().contains(&query_lower) {
            highlights.push(format!("Name: {}", entry.plugin_info.name));
        }

        if entry.plugin_info.description.to_lowercase().contains(&query_lower) {
            highlights.push(format!("Description: {}", entry.plugin_info.description));
        }

        highlights
    }

    /// 排序搜索结果
    fn sort_search_results(&self, results: &mut Vec<SearchResultItem>, parameters: &SearchParameters) {
        match parameters.sort_by {
            SortOption::Relevance => {
                results.sort_by(|a, b| {
                    match parameters.sort_order {
                        SortOrder::Descending => b.relevance_score.partial_cmp(&a.relevance_score).unwrap(),
                        SortOrder::Ascending => a.relevance_score.partial_cmp(&b.relevance_score).unwrap(),
                    }
                });
            }
            SortOption::Downloads => {
                results.sort_by(|a, b| {
                    match parameters.sort_order {
                        SortOrder::Descending => b.entry.download_stats.total_downloads.cmp(&a.entry.download_stats.total_downloads),
                        SortOrder::Ascending => a.entry.download_stats.total_downloads.cmp(&b.entry.download_stats.total_downloads),
                    }
                });
            }
            SortOption::Rating => {
                results.sort_by(|a, b| {
                    match parameters.sort_order {
                        SortOrder::Descending => b.entry.rating.average_rating.partial_cmp(&a.entry.rating.average_rating).unwrap(),
                        SortOrder::Ascending => a.entry.rating.average_rating.partial_cmp(&b.entry.rating.average_rating).unwrap(),
                    }
                });
            }
            SortOption::UpdatedAt => {
                results.sort_by(|a, b| {
                    match parameters.sort_order {
                        SortOrder::Descending => b.entry.updated_at.cmp(&a.entry.updated_at),
                        SortOrder::Ascending => a.entry.updated_at.cmp(&b.entry.updated_at),
                    }
                });
            }
            SortOption::Name => {
                results.sort_by(|a, b| {
                    match parameters.sort_order {
                        SortOrder::Ascending => a.entry.plugin_info.name.cmp(&b.entry.plugin_info.name),
                        SortOrder::Descending => b.entry.plugin_info.name.cmp(&a.entry.plugin_info.name),
                    }
                });
            }
            _ => {} // 其他排序选项的实现
        }
    }

    /// 生成搜索分面
    async fn generate_search_facets(&self, results: &[SearchResultItem]) -> SearchFacets {
        let mut categories = HashMap::new();
        let mut tags = HashMap::new();
        let mut authors = HashMap::new();
        let mut licenses = HashMap::new();

        for result in results {
            // 统计分类
            for category in &result.entry.categories {
                *categories.entry(category.clone()).or_insert(0) += 1;
            }

            // 统计标签
            for tag in &result.entry.tags {
                *tags.entry(tag.clone()).or_insert(0) += 1;
            }

            // 统计作者
            *authors.entry(result.entry.plugin_info.author.clone()).or_insert(0) += 1;

            // 统计许可证
            *licenses.entry(result.entry.license.license_type.clone()).or_insert(0) += 1;
        }

        SearchFacets {
            categories,
            tags,
            authors,
            licenses,
            price_ranges: HashMap::new(), // 简化处理
        }
    }

    /// 生成搜索建议
    async fn generate_search_suggestions(&self, query: &str) -> Vec<String> {
        // 简化的搜索建议逻辑
        vec![
            format!("{} plugin", query),
            format!("{} tool", query),
            format!("{} library", query),
        ]
    }

    /// 更新下载统计
    async fn update_download_statistics(&self, plugin_id: &PluginId) {
        let mut registry = self.registry.write().await;
        if let Some(entry) = registry.get_mut(plugin_id) {
            entry.download_stats.total_downloads += 1;
            entry.download_stats.daily_downloads += 1;
            entry.updated_at = chrono::Utc::now().timestamp();
        }
    }

    /// 更新插件评级
    async fn update_plugin_rating(&self, plugin_id: &PluginId) {
        let ratings = self.rating_system.ratings.read().await;
        if let Some(plugin_ratings) = ratings.get(plugin_id) {
            let total_score: u32 = plugin_ratings.iter().map(|r| r.score as u32).sum();
            let average_rating = total_score as f64 / plugin_ratings.len() as f64;

            let mut rating_distribution = HashMap::new();
            for rating in plugin_ratings {
                *rating_distribution.entry(rating.score).or_insert(0) += 1;
            }

            let mut registry = self.registry.write().await;
            if let Some(entry) = registry.get_mut(plugin_id) {
                entry.rating.average_rating = average_rating;
                entry.rating.rating_count = plugin_ratings.len() as u32;
                entry.rating.rating_distribution = rating_distribution;
                entry.updated_at = chrono::Utc::now().timestamp();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_marketplace_creation() {
        let marketplace = PluginMarketplace::new(MarketplaceConfig::default());
        let stats = marketplace.get_statistics().await;
        assert_eq!(stats.total_plugins, 0);
        assert_eq!(stats.published_plugins, 0);
    }

    #[tokio::test]
    async fn test_search_parameters() {
        let params = SearchParameters {
            query: "test".to_string(),
            categories: vec![],
            tags: vec![],
            authors: vec![],
            licenses: vec![],
            pricing: None,
            min_rating: None,
            sort_by: SortOption::Relevance,
            sort_order: SortOrder::Descending,
            page_size: 10,
            page_offset: 0,
        };

        assert_eq!(params.query, "test");
        assert_eq!(params.sort_by, SortOption::Relevance);
    }

    #[test]
    fn test_license_types() {
        let mit_license = LicenseType::MIT;
        let custom_license = LicenseType::Custom("My License".to_string());
        
        assert_eq!(mit_license, LicenseType::MIT);
        assert_ne!(mit_license, custom_license);
    }

    #[test]
    fn test_pricing_models() {
        let free_pricing = PricingInfo {
            pricing_model: PricingModel::Free,
            price: 0.0,
            currency: "USD".to_string(),
            trial_period: None,
            subscription_period: None,
        };

        assert_eq!(free_pricing.pricing_model, PricingModel::Free);
        assert_eq!(free_pricing.price, 0.0);
    }
}