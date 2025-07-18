# CZSC Enhanced Rust Core Engine

基于wondertrader架构设计的高性能Rust量化交易核心引擎，专为CZSC Enhanced框架优化。

## 🚀 项目概述

CZSC Enhanced Rust核心引擎是一个高性能的量化交易系统，集成了：

- **缠中说禅(CZSC)理论**
- **Smart Money Concepts (SMC)**
- **Inner Circle Trader (ICT)**
- **Wyckoff理论**
- **多时框架融合分析**

## 🏗️ 架构设计

### 核心模块

```text
┌─────────────────────────────────────────────────────────────┐
│                    CZSC Enhanced Core                       │
├─────────────────┬─────────────────┬─────────────────────────┤
│ Trading Engine  │  Data Engine    │    Backtest Engine      │
│                 │                 │                         │
│ • Strategy Mgr  │ • Data Storage  │ • Historical Data       │
│ • Risk Monitor  │ • Live Feed     │ • Match Engine          │
│ • Order Exec    │ • Market Data   │ • Performance Analysis  │
└─────────────────┼─────────────────┼─────────────────────────┤
│                    CZSC Enhanced Algorithms                 │
│                                                             │
│ • POI Detection  • Multi-Timeframe Analysis • Signal Gen   │
└─────────────────────────────────────────────────────────────┘
```

### 模块说明

#### 1. 交易引擎 (Trading Engine)
- **策略管理**: 多策略并行执行和生命周期管理
- **风险控制**: 实时风险监控和仓位管理
- **订单执行**: 高性能订单路由和执行引擎
- **投资组合**: 多品种组合管理和优化

#### 2. 数据引擎 (Data Engine)
- **数据存储**: 高效的历史数据存储(LMDB/RocksDB)
- **实时数据**: 多源数据聚合和处理
- **数据质量**: 数据清洗和质量监控

#### 3. 回测引擎 (Backtest Engine)
- **数据回放**: 高精度历史数据回放
- **撮合引擎**: 模拟交易撮合和成交
- **性能分析**: 全面的回测性能指标

#### 4. CZSC增强算法
- **POI检测**: FVG、Order Block、流动性检测
- **分型分析**: 缠论分型和笔段识别
- **结构分析**: 市场结构和趋势识别
- **多时框融合**: 跨时框信号确认和过滤

#### 5. 数据连接器 (Connectors)
- **市场数据**: 支持多种数据源接入
- **交易接口**: 统一的交易API抽象
- **加密货币**: 数字货币交易所接入
- **期货**: 期货交易所接入

## 🛠️ 技术特性

### 性能优化
- **零拷贝数据处理**: 最小化内存分配和拷贝
- **异步并发**: 基于Tokio的高并发处理
- **SIMD优化**: 向量化数值计算
- **内存映射**: 高效的数据存储访问

### 安全保障
- **内存安全**: Rust语言的内存安全保证
- **类型安全**: 强类型系统防止运行时错误
- **并发安全**: 无数据竞争的并发设计

### 可扩展性
- **模块化设计**: 松耦合的模块化架构
- **插件系统**: 支持策略和连接器插件
- **配置驱动**: 灵活的配置管理系统

## 📦 依赖库

### 核心依赖
- **tokio**: 异步运行时
- **serde**: 序列化/反序列化
- **chrono**: 时间处理
- **ndarray**: 数值计算
- **rayon**: 并行计算

### 数据存储
- **lmdb**: 高性能嵌入式数据库
- **rocksdb**: 可扩展键值存储

### 网络通信
- **tonic**: gRPC客户端/服务端
- **hyper**: HTTP客户端/服务端
- **reqwest**: HTTP客户端

### Python集成
- **pyo3**: Python FFI绑定 (可选)

## 🚦 快速开始

### 安装依赖
```bash
# 安装Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 切换到项目目录
cd czsc_core

# 检查项目编译
cargo check

# 运行测试
cargo test

# 构建发布版本
cargo build --release
```

### 基本使用
```rust
use czsc_core::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建引擎配置
    let config = EngineConfig {
        trading: TradingConfig::default(),
        data: DataConfig::default(),
        czsc: CzscConfig::default(),
        backtest: None,
    };
    
    // 创建引擎管理器
    let mut engine = EngineManager::new(config)?;
    
    // 初始化和启动
    engine.initialize().await?;
    engine.start().await?;
    
    // 处理市场数据
    let market_data = MarketSlice { /* ... */ };
    engine.process_market_data(market_data).await?;
    
    Ok(())
}
```

## 🧪 开发状态

### 已完成 ✅
- [x] 项目架构设计
- [x] 核心数据结构定义
- [x] 基础模块框架
- [x] 交易引擎框架
- [x] 数据引擎框架
- [x] 回测引擎框架
- [x] CZSC算法框架
- [x] 连接器接口定义

### 进行中 🔄
- [ ] POI检测算法实现
- [ ] 多时框分析实现
- [ ] CZSC核心算法移植
- [ ] 性能优化和测试

### 计划中 📋
- [ ] Python FFI接口
- [ ] 具体交易所连接器
- [ ] 图形化监控界面
- [ ] 完整的回测报告
- [ ] 生产环境部署

## 🤝 与Python框架集成

Rust核心引擎通过以下方式与Python CZSC Enhanced框架集成：

1. **FFI绑定**: 通过pyo3提供Python接口
2. **数据共享**: 高效的数据交换机制
3. **配置统一**: 共享配置格式和管理
4. **性能互补**: Python用于策略开发，Rust用于性能关键路径

## 📊 性能对比

相比纯Python实现的预期性能提升：

| 模块 | 性能提升 | 说明 |
|------|----------|------|
| POI检测 | 10-50x | 大数据集向量化处理 |
| 信号计算 | 5-20x | 并行数值计算 |
| 数据处理 | 3-10x | 零拷贝内存操作 |
| 回测引擎 | 20-100x | 高效历史数据回放 |

## 📄 许可证

本项目采用与CZSC Enhanced框架相同的许可证。

## 🔗 相关资源

- [wondertrader项目](https://github.com/wondertrader/wondertrader)
- [CZSC Enhanced Python框架](../README.md)
- [Rust异步编程](https://tokio.rs/)
- [pyo3 Python绑定](https://pyo3.rs/)

---

**注意**: 这是一个正在开发中的项目，API可能会有变化。建议在生产环境使用前充分测试。