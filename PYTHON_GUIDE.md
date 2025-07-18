# MosesQuant Python用户指南

## 简介

MosesQuant是一个高性能量化交易框架，使用Rust实现底层计算引擎，通过Python绑定提供易用的策略开发接口。

## 核心特性

### 🚀 高性能计算引擎
- **Rust底层**: 零成本抽象，内存安全
- **SIMD优化**: 向量化计算提升性能
- **并发支持**: 高效的并行计算
- **低延迟**: 微秒级响应时间

### 📊 丰富的技术指标
- **趋势指标**: SMA, EMA, MACD
- **震荡指标**: RSI, Stochastic, Williams %R
- **波动性指标**: Bollinger Bands, ATR
- **自定义指标**: 支持用户扩展

### 🎯 标准化接口
- **Alpha模型**: 标准化的策略开发接口
- **数据访问**: 统一的数据获取接口
- **风险管理**: 完整的风险指标计算
- **插件系统**: 支持动态加载策略

## 安装指南

### 前提条件
1. **Python 3.7+**: 推荐使用最新版本
2. **Rust 1.60+**: 用于编译底层引擎
3. **Cargo**: Rust的包管理器

### 安装步骤

#### 方法1: 使用maturin（推荐）
```bash
# 安装maturin
pip install maturin

# 克隆仓库
git clone https://github.com/your-org/MosesQuant.git
cd MosesQuant

# 安装依赖
pip install -r requirements.txt

# 开发模式安装
maturin develop

# 或者构建wheel包
maturin build --release
pip install target/wheels/*.whl
```

#### 方法2: 使用setup.py
```bash
# 确保已安装构建工具
pip install setuptools wheel

# 运行setup.py
python setup.py install
```

### 验证安装
```python
import moses_quant as mq
print(f"MosesQuant版本: {mq.VERSION}")
print(f"框架名称: {mq.FRAMEWORK_NAME}")
```

## 快速开始

### 基础用法

```python
import moses_quant as mq

# 创建计算引擎
engine = mq.PyCalculationEngine()

# 示例价格数据
prices = [100, 101, 102, 103, 104, 105, 104, 103, 102, 101]

# 计算技术指标
sma_5 = engine.calculate_sma(prices, 5)
ema_5 = engine.calculate_ema(prices, 5)
rsi_14 = engine.calculate_rsi(prices, 14)

print(f"SMA(5): {sma_5[-1]:.2f}")
print(f"EMA(5): {ema_5[-1]:.2f}")
print(f"RSI(14): {rsi_14[-1]:.2f}")
```

### 自定义Alpha模型

```python
class MyRSIStrategy:
    def __init__(self, period=14, overbought=70, oversold=30):
        self.period = period
        self.overbought = overbought
        self.oversold = oversold
        self.engine = mq.PyCalculationEngine()
        
    def generate_insights(self, symbols):
        insights = []
        data_provider = mq.PyDataProvider()
        
        for symbol in symbols:
            # 获取历史数据
            prices = data_provider.get_price_history(symbol, 30)
            
            # 计算RSI
            rsi_values = self.engine.calculate_rsi(prices, self.period)
            latest_rsi = rsi_values[-1]
            
            # 生成信号
            if latest_rsi > self.overbought:
                insight = mq.PyInsight(symbol, "Down")
                insight.confidence = 0.8
                insights.append(insight)
            elif latest_rsi < self.oversold:
                insight = mq.PyInsight(symbol, "Up")
                insight.confidence = 0.8
                insights.append(insight)
                
        return insights

# 使用策略
strategy = MyRSIStrategy()
symbols = ["BTCUSDT", "ETHUSDT", "ADAUSDT"]
insights = strategy.generate_insights(symbols)

for insight in insights:
    print(f"{insight.symbol}: {insight.direction} (置信度: {insight.confidence})")
```

## API参考

### PyCalculationEngine

高性能计算引擎，提供技术指标和统计计算功能。

#### 技术指标计算

```python
engine = mq.PyCalculationEngine()

# 简单移动平均
sma = engine.calculate_sma(prices: List[float], period: int) -> List[float]

# 指数移动平均
ema = engine.calculate_ema(prices: List[float], period: int) -> List[float]

# 相对强弱指标
rsi = engine.calculate_rsi(prices: List[float], period: int) -> List[float]

# MACD指标
macd, signal, histogram = engine.calculate_macd(
    prices: List[float], 
    fast_period: int, 
    slow_period: int, 
    signal_period: int
) -> Tuple[List[float], List[float], List[float]]

# 布林带
lower, middle, upper = engine.calculate_bollinger_bands(
    prices: List[float], 
    period: int, 
    std_multiplier: float
) -> Tuple[List[float], List[float], List[float]]
```

#### 统计计算

```python
# 基础统计
mean = engine.calculate_statistic(data: List[float], "mean") -> float
median = engine.calculate_statistic(data: List[float], "median") -> float
std_dev = engine.calculate_statistic(data: List[float], "std_dev") -> float

# 相关性
correlation = engine.calculate_correlation(
    series1: List[float], 
    series2: List[float]
) -> float

# 风险指标
risk_metrics = engine.calculate_risk_metrics(returns: List[float]) -> PyRiskMetrics
```

### PyInsight

交易洞见数据结构，表示策略生成的交易信号。

```python
insight = mq.PyInsight(symbol: str, direction: str)

# 属性
insight.symbol: str           # 交易标的
insight.direction: str        # 方向: "Up", "Down", "Flat"
insight.confidence: float     # 置信度 [0.0, 1.0]
insight.magnitude: float      # 信号强度
insight.weight: float         # 权重
insight.source_model: str     # 来源模型
insight.generated_time_utc: int  # 生成时间
insight.close_time_utc: int   # 关闭时间

# 方法
score = insight.score()       # 计算洞见评分
```

### PyRiskMetrics

风险指标数据结构，包含各种风险度量。

```python
risk_metrics = engine.calculate_risk_metrics(returns)

# 属性
risk_metrics.volatility: float      # 波动率
risk_metrics.var_95: float          # 95% VaR
risk_metrics.var_99: float          # 99% VaR
risk_metrics.max_drawdown: float    # 最大回撤
risk_metrics.sharpe_ratio: float    # 夏普比率
risk_metrics.sortino_ratio: float   # 索提诺比率
risk_metrics.calmar_ratio: float    # 卡尔玛比率
```

### PyDataProvider

数据访问接口，提供历史数据和实时数据获取能力。

```python
data_provider = mq.PyDataProvider()

# 获取历史价格
prices = data_provider.get_price_history(symbol: str, days: int) -> List[float]

# 获取市场快照
snapshot = data_provider.get_market_snapshot(symbols: List[str]) -> Dict[str, float]
```

## 示例策略

### 1. RSI策略

```python
class RSIStrategy:
    def __init__(self, period=14, overbought=70, oversold=30):
        self.period = period
        self.overbought = overbought
        self.oversold = oversold
        self.engine = mq.PyCalculationEngine()
        
    def generate_signals(self, symbol, prices):
        rsi_values = self.engine.calculate_rsi(prices, self.period)
        latest_rsi = rsi_values[-1]
        
        if latest_rsi > self.overbought:
            return "SELL"
        elif latest_rsi < self.oversold:
            return "BUY"
        else:
            return "HOLD"
```

### 2. 移动平均交叉策略

```python
class MACrossStrategy:
    def __init__(self, fast_period=10, slow_period=20):
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.engine = mq.PyCalculationEngine()
        
    def generate_signals(self, symbol, prices):
        fast_ma = self.engine.calculate_sma(prices, self.fast_period)
        slow_ma = self.engine.calculate_sma(prices, self.slow_period)
        
        if fast_ma[-1] > slow_ma[-1] and fast_ma[-2] <= slow_ma[-2]:
            return "BUY"  # 金叉
        elif fast_ma[-1] < slow_ma[-1] and fast_ma[-2] >= slow_ma[-2]:
            return "SELL"  # 死叉
        else:
            return "HOLD"
```

### 3. 布林带策略

```python
class BollingerBandsStrategy:
    def __init__(self, period=20, std_multiplier=2.0):
        self.period = period
        self.std_multiplier = std_multiplier
        self.engine = mq.PyCalculationEngine()
        
    def generate_signals(self, symbol, prices):
        lower, middle, upper = self.engine.calculate_bollinger_bands(
            prices, self.period, self.std_multiplier
        )
        
        current_price = prices[-1]
        
        if current_price < lower[-1]:
            return "BUY"   # 价格触及下轨
        elif current_price > upper[-1]:
            return "SELL"  # 价格触及上轨
        else:
            return "HOLD"
```

## 性能优化建议

### 1. 数据处理优化
```python
# 好的做法：批量计算
symbols = ["BTCUSDT", "ETHUSDT", "ADAUSDT"]
all_prices = {symbol: get_prices(symbol) for symbol in symbols}

# 批量计算指标
for symbol, prices in all_prices.items():
    sma = engine.calculate_sma(prices, 20)
    # 处理结果...
```

### 2. 内存管理
```python
# 避免在循环中创建大量对象
insights = []
for symbol in symbols:
    insight = mq.PyInsight(symbol, "Up")
    insight.confidence = 0.8
    insights.append(insight)
```

### 3. 并行计算
```python
from concurrent.futures import ThreadPoolExecutor

def process_symbol(symbol):
    prices = get_prices(symbol)
    return engine.calculate_rsi(prices, 14)

# 并行处理多个标的
with ThreadPoolExecutor(max_workers=4) as executor:
    results = list(executor.map(process_symbol, symbols))
```

## 最佳实践

### 1. 策略开发
- 使用标准化的接口定义
- 分离数据获取和计算逻辑
- 实现清晰的信号生成逻辑
- 添加适当的错误处理

### 2. 性能考虑
- 缓存计算结果
- 使用批量操作
- 避免不必要的数据拷贝
- 合理设置计算周期

### 3. 风险管理
- 始终计算风险指标
- 设置合理的止损和止盈
- 监控仓位规模
- 实施资金管理

## 常见问题

### Q: 如何处理缺失数据？
A: 计算引擎会自动处理数据长度不足的情况，返回空列表或适当的默认值。

### Q: 如何提高计算性能？
A: 使用批量计算、缓存结果、并行处理等技术。

### Q: 如何扩展自定义指标？
A: 目前支持通过组合现有指标实现复杂逻辑，未来版本将支持自定义指标扩展。

### Q: 如何处理实时数据？
A: 使用PyDataProvider接口，实现自己的数据源连接器。

## 更多资源

- [GitHub仓库](https://github.com/your-org/MosesQuant)
- [完整示例](python_examples/)
- [API文档](docs/api/)
- [社区论坛](https://forum.mosesquant.com)

## 贡献指南

欢迎贡献代码、报告问题或提出改进建议：

1. Fork仓库
2. 创建功能分支
3. 提交更改
4. 发起Pull Request

## 许可证

MIT License - 详见LICENSE文件。