# MosesQuant Python绑定

[![Build Status](https://github.com/your-org/MosesQuant/workflows/CI/badge.svg)](https://github.com/your-org/MosesQuant/actions)
[![PyPI version](https://badge.fury.io/py/moses-quant.svg)](https://badge.fury.io/py/moses-quant)
[![Python versions](https://img.shields.io/pypi/pyversions/moses-quant.svg)](https://pypi.org/project/moses-quant/)
[![License](https://img.shields.io/github/license/your-org/MosesQuant.svg)](https://github.com/your-org/MosesQuant/blob/master/LICENSE)

MosesQuant是一个高性能量化交易框架，使用Rust构建底层计算引擎，通过Python绑定提供易用的策略开发接口。

## 🚀 特性

- **高性能计算**: Rust底层引擎，零成本抽象
- **丰富的技术指标**: SMA, EMA, RSI, MACD, Bollinger Bands等
- **风险管理**: VaR, 夏普比率, 最大回撤等风险指标
- **标准化接口**: 清晰的Alpha模型开发接口
- **类型安全**: 编译时类型检查，运行时内存安全
- **易于使用**: Python友好的API设计

## 📦 安装

### 从PyPI安装 (推荐)

```bash
pip install moses-quant
```

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/your-org/MosesQuant.git
cd MosesQuant

# 安装构建依赖
pip install maturin

# 构建并安装
maturin develop --release
```

## 🎯 快速开始

### 基础计算

```python
import moses_quant as mq

# 创建计算引擎
engine = mq.PyCalculationEngine()

# 示例价格数据
prices = [100, 101, 102, 103, 104, 105, 104, 103, 102, 101]

# 计算技术指标
sma_5 = engine.calculate_sma(prices, 5)
rsi_14 = engine.calculate_rsi(prices, 14)

print(f"SMA(5): {sma_5[-1]:.2f}")
print(f"RSI(14): {rsi_14[-1]:.2f}")
```

### 策略开发

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
            insight = mq.PyInsight(symbol, "Down")
            insight.confidence = 0.8
            return insight
        elif latest_rsi < self.oversold:
            insight = mq.PyInsight(symbol, "Up")
            insight.confidence = 0.8
            return insight
        
        return None

# 使用策略
strategy = RSIStrategy()
data_provider = mq.PyDataProvider()

symbols = ["BTCUSDT", "ETHUSDT"]
for symbol in symbols:
    prices = data_provider.get_price_history(symbol, 30)
    signal = strategy.generate_signals(symbol, prices)
    if signal:
        print(f"{symbol}: {signal.direction} (置信度: {signal.confidence})")
```

### 风险分析

```python
# 计算收益率
returns = []
for i in range(1, len(prices)):
    ret = (prices[i] - prices[i-1]) / prices[i-1]
    returns.append(ret)

# 计算风险指标
risk_metrics = engine.calculate_risk_metrics(returns)

print(f"波动率: {risk_metrics.volatility:.4f}")
print(f"VaR(95%): {risk_metrics.var_95:.4f}")
print(f"夏普比率: {risk_metrics.sharpe_ratio:.4f}")
print(f"最大回撤: {risk_metrics.max_drawdown:.4f}")
```

## 📊 支持的指标

### 趋势指标
- **SMA**: 简单移动平均
- **EMA**: 指数移动平均
- **MACD**: 移动平均收敛发散

### 震荡指标
- **RSI**: 相对强弱指标
- **Stochastic**: 随机指标
- **Williams %R**: 威廉指标

### 波动性指标
- **Bollinger Bands**: 布林带
- **ATR**: 平均真实波幅

### 统计指标
- **均值、中位数、标准差**
- **偏度、峰度**
- **相关性分析**

### 风险指标
- **VaR**: 风险价值
- **夏普比率**: 风险调整收益
- **索提诺比率**: 下行风险调整收益
- **卡尔玛比率**: 回撤调整收益
- **最大回撤**: 最大损失

## 📖 API文档

### PyCalculationEngine

高性能计算引擎，提供所有技术指标和统计计算功能。

```python
engine = mq.PyCalculationEngine()

# 技术指标
sma = engine.calculate_sma(prices, period)
ema = engine.calculate_ema(prices, period)
rsi = engine.calculate_rsi(prices, period)
macd_line, signal_line, histogram = engine.calculate_macd(prices, 12, 26, 9)
lower, middle, upper = engine.calculate_bollinger_bands(prices, 20, 2.0)

# 统计指标
mean = engine.calculate_statistic(data, "mean")
std_dev = engine.calculate_statistic(data, "std_dev")
correlation = engine.calculate_correlation(series1, series2)

# 风险指标
risk_metrics = engine.calculate_risk_metrics(returns)
```

### PyInsight

交易洞见数据结构，表示策略生成的交易信号。

```python
insight = mq.PyInsight(symbol, direction)
insight.confidence = 0.8  # 置信度 [0.0, 1.0]
insight.magnitude = 1.0   # 信号强度
insight.weight = 0.1      # 权重
```

### PyDataProvider

数据访问接口，提供历史数据和实时数据。

```python
data_provider = mq.PyDataProvider()
prices = data_provider.get_price_history(symbol, days)
snapshot = data_provider.get_market_snapshot(symbols)
```

## 🔧 高级用法

### 复合策略

```python
class CompositeStrategy:
    def __init__(self):
        self.strategies = [
            RSIStrategy(period=14),
            MACrossStrategy(fast=10, slow=20),
            MomentumStrategy(period=5)
        ]
    
    def generate_signals(self, symbol, prices):
        signals = []
        for strategy in self.strategies:
            signal = strategy.generate_signals(symbol, prices)
            if signal:
                signals.append(signal)
        
        return self.aggregate_signals(signals)
```

### 批量分析

```python
def analyze_portfolio(symbols, engine):
    results = {}
    for symbol in symbols:
        prices = get_price_data(symbol)
        returns = calculate_returns(prices)
        
        # 技术分析
        sma_20 = engine.calculate_sma(prices, 20)
        rsi_14 = engine.calculate_rsi(prices, 14)
        
        # 风险分析
        risk_metrics = engine.calculate_risk_metrics(returns)
        
        results[symbol] = {
            'sma_20': sma_20[-1],
            'rsi_14': rsi_14[-1],
            'sharpe_ratio': risk_metrics.sharpe_ratio,
            'max_drawdown': risk_metrics.max_drawdown
        }
    
    return results
```

## 🎨 示例策略

查看 [python_examples/](python_examples/) 目录获取更多示例：

- **RSI策略**: 基于相对强弱指标的交易策略
- **移动平均交叉**: 快慢移动平均交叉策略
- **布林带策略**: 基于布林带的均值回归策略
- **复合策略**: 多个策略信号的组合

## 🔬 性能

MosesQuant的Python绑定通过Rust底层引擎提供优异的性能：

- **技术指标计算**: 比纯Python实现快10-100倍
- **内存使用**: 零拷贝数据传输，内存使用最优
- **并发处理**: 支持多线程并行计算
- **类型安全**: 编译时类型检查，运行时无开销

## 📚 文档

- [Python用户指南](PYTHON_GUIDE.md)
- [API参考文档](docs/python_api.md)
- [架构文档](架构/06-Python FFI绑定架构.md)
- [示例代码](python_examples/)

## 🤝 贡献

欢迎贡献代码！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与开发。

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/your-org/MosesQuant.git
cd MosesQuant

# 安装开发依赖
pip install -r requirements.txt

# 安装开发模式
maturin develop

# 运行测试
cargo test
python -m pytest python_examples/test_examples.py
```

## 📄 许可证

本项目使用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 🔗 相关链接

- [GitHub仓库](https://github.com/your-org/MosesQuant)
- [PyPI包](https://pypi.org/project/moses-quant/)
- [文档网站](https://docs.mosesquant.com)
- [社区论坛](https://forum.mosesquant.com)

## 🆘 获取帮助

如果遇到问题或需要帮助：

1. 查看 [FAQ](docs/FAQ.md)
2. 搜索 [GitHub Issues](https://github.com/your-org/MosesQuant/issues)
3. 提交新的 [Issue](https://github.com/your-org/MosesQuant/issues/new)
4. 加入 [Discord社区](https://discord.gg/mosesquant)

## 🙏 致谢

- [PyO3](https://github.com/PyO3/pyo3) - 优秀的Python-Rust绑定库
- [Maturin](https://github.com/PyO3/maturin) - Python扩展构建工具
- [QuantConnect LEAN](https://github.com/QuantConnect/Lean) - 架构设计灵感
- [WonderTrader](https://github.com/wondertrader/wondertrader) - 量化交易框架参考

---

**MosesQuant** - 高性能量化交易，Python易用性与Rust性能的完美结合 🚀