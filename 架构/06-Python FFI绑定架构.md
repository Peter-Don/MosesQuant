# Python FFI绑定架构文档

## 概览

MosesQuant Python FFI绑定实现了Rust底层引擎与Python策略开发接口的无缝集成。用户可以使用Python编写量化交易策略，同时享受Rust引擎的高性能计算能力。

## 核心设计理念

### 1. 关注点分离

**Python层 (策略逻辑)**
- 策略开发和Alpha模型实现
- 业务逻辑和决策制定
- 快速原型开发和测试

**Rust层 (计算引擎)**
- 高性能数值计算
- 技术指标计算
- 内存安全和并发处理
- 系统级性能优化

### 2. 零成本抽象

- 最小化Python-Rust之间的数据转换开销
- 直接调用Rust函数，避免多层封装
- 智能的内存管理和生命周期控制

## 架构组件

### 1. Python绑定层

#### 1.1 PyAlphaModel
Python Alpha模型包装器，允许Python代码实现AlphaModel接口。

```python
class MyStrategy:
    def generate_insights(self, symbols):
        # 用户策略逻辑
        return insights
```

#### 1.2 PyCalculationEngine
高性能计算引擎的Python接口。

```python
engine = mq.PyCalculationEngine()
sma = engine.calculate_sma(prices, 20)
rsi = engine.calculate_rsi(prices, 14)
```

#### 1.3 PyDataProvider
数据访问接口，提供历史数据和实时数据。

```python
data_provider = mq.PyDataProvider()
prices = data_provider.get_price_history("BTCUSDT", 30)
```

### 2. 类型转换层

#### 2.1 PyInsight
Python洞见数据结构，与Rust Insight类型双向转换。

```python
insight = mq.PyInsight("BTCUSDT", "Up")
insight.confidence = 0.8
insight.magnitude = 1.0
```

#### 2.2 PyRiskMetrics
风险指标数据结构，包含完整的风险分析结果。

```python
risk_metrics = engine.calculate_risk_metrics(returns)
print(f"夏普比率: {risk_metrics.sharpe_ratio}")
print(f"最大回撤: {risk_metrics.max_drawdown}")
```

### 3. 功能模块

#### 3.1 技术指标计算
- **趋势指标**: SMA, EMA, MACD
- **震荡指标**: RSI, Stochastic, Williams %R
- **波动性指标**: Bollinger Bands, ATR
- **统计指标**: 均值、中位数、标准差、相关性

#### 3.2 风险管理
- **风险指标**: VaR, 波动率, 最大回撤
- **绩效指标**: 夏普比率, 索提诺比率, 卡尔玛比率
- **统计分析**: 偏度、峰度、相关性分析

#### 3.3 数据处理
- **历史数据**: 价格、成交量、技术指标历史
- **实时数据**: 市场快照、流式数据更新
- **数据质量**: 数据验证、清洗、补全

## 使用模式

### 1. 简单策略模式

```python
import moses_quant as mq

class SimpleRSIStrategy:
    def __init__(self):
        self.engine = mq.PyCalculationEngine()
        self.data_provider = mq.PyDataProvider()
    
    def generate_signals(self, symbols):
        insights = []
        for symbol in symbols:
            prices = self.data_provider.get_price_history(symbol, 30)
            rsi = self.engine.calculate_rsi(prices, 14)
            
            if rsi[-1] > 70:
                insight = mq.PyInsight(symbol, "Down")
                insight.confidence = 0.7
                insights.append(insight)
            elif rsi[-1] < 30:
                insight = mq.PyInsight(symbol, "Up")
                insight.confidence = 0.7
                insights.append(insight)
        
        return insights
```

### 2. 复合策略模式

```python
class CompositeStrategy:
    def __init__(self):
        self.engine = mq.PyCalculationEngine()
        self.data_provider = mq.PyDataProvider()
        self.strategies = [
            RSIStrategy(),
            MACrossStrategy(),
            MomentumStrategy()
        ]
    
    def generate_signals(self, symbols):
        all_insights = []
        for strategy in self.strategies:
            insights = strategy.generate_signals(symbols)
            all_insights.extend(insights)
        
        # 信号聚合逻辑
        return self.aggregate_signals(all_insights)
```

### 3. 高级分析模式

```python
class AdvancedAnalyzer:
    def __init__(self):
        self.engine = mq.PyCalculationEngine()
    
    def comprehensive_analysis(self, symbols):
        analysis_results = {}
        
        for symbol in symbols:
            prices = self.get_prices(symbol)
            
            # 技术指标分析
            technical_analysis = {
                'sma_20': self.engine.calculate_sma(prices, 20),
                'rsi_14': self.engine.calculate_rsi(prices, 14),
                'macd': self.engine.calculate_macd(prices, 12, 26, 9)
            }
            
            # 风险分析
            returns = self.calculate_returns(prices)
            risk_metrics = self.engine.calculate_risk_metrics(returns)
            
            # 相关性分析
            correlations = {}
            for other_symbol in symbols:
                if other_symbol != symbol:
                    other_prices = self.get_prices(other_symbol)
                    other_returns = self.calculate_returns(other_prices)
                    corr = self.engine.calculate_correlation(returns, other_returns)
                    correlations[other_symbol] = corr
            
            analysis_results[symbol] = {
                'technical': technical_analysis,
                'risk': risk_metrics,
                'correlations': correlations
            }
        
        return analysis_results
```

## 性能特性

### 1. 高效的数据传输

- **零拷贝传输**: 大型数组直接在Rust和Python之间共享内存
- **批量操作**: 支持向量化计算，提高处理效率
- **智能缓存**: 计算结果缓存，避免重复计算

### 2. 内存管理

- **自动内存管理**: Rust的所有权系统保证内存安全
- **引用计数**: 共享数据使用Arc智能指针
- **垃圾回收**: Python对象由GIL管理，无需手动释放

### 3. 并发处理

- **线程安全**: 所有绑定类型都实现了Send + Sync
- **并行计算**: 支持多线程并行处理不同标的
- **异步支持**: 兼容Python的异步编程模式

## 错误处理

### 1. 错误类型映射

```python
try:
    result = engine.calculate_sma(prices, 0)  # 无效参数
except RuntimeError as e:
    print(f"计算错误: {e}")

try:
    prices = data_provider.get_price_history("INVALID", 30)
except ValueError as e:
    print(f"数据错误: {e}")
```

### 2. 异常处理策略

- **参数验证**: 输入参数在Rust层验证，返回清晰错误信息
- **优雅降级**: 计算失败时返回空结果而非崩溃
- **错误传播**: Rust错误正确映射到Python异常

## 扩展性

### 1. 自定义指标

```python
# 未来支持的扩展方式
class CustomIndicator:
    def calculate(self, prices, params):
        # 自定义计算逻辑
        return result

engine.register_custom_indicator("my_indicator", CustomIndicator())
```

### 2. 插件系统

```python
# 策略插件接口
class StrategyPlugin:
    def create_alpha_model(self, config):
        return MyAlphaModel(config)
    
    def create_risk_manager(self, config):
        return MyRiskManager(config)
```

### 3. 多语言支持

- **Python**: 主要支持语言，功能最完整
- **JavaScript**: 通过WebAssembly支持
- **Java**: 通过JNI绑定
- **C/C++**: 标准C FFI接口

## 部署和分发

### 1. 构建系统

```bash
# 使用maturin构建Python wheel
maturin build --release

# 安装到Python环境
pip install target/wheels/moses_quant-*.whl
```

### 2. 依赖管理

```toml
[dependencies]
pyo3 = { version = "0.18", features = ["extension-module"] }
pythonize = "0.18"
```

### 3. 跨平台支持

- **Windows**: 原生支持，预编译wheel
- **Linux**: 支持多种发行版
- **macOS**: Intel和Apple Silicon支持

## 测试策略

### 1. 单元测试

```python
def test_sma_calculation():
    engine = mq.PyCalculationEngine()
    prices = [1, 2, 3, 4, 5]
    result = engine.calculate_sma(prices, 3)
    assert len(result) == 3
    assert abs(result[0] - 2.0) < 1e-10
```

### 2. 集成测试

```python
def test_strategy_integration():
    strategy = SimpleRSIStrategy()
    symbols = ["BTCUSDT", "ETHUSDT"]
    insights = strategy.generate_signals(symbols)
    assert all(isinstance(insight, mq.PyInsight) for insight in insights)
```

### 3. 性能测试

```python
import time

def benchmark_calculation():
    engine = mq.PyCalculationEngine()
    prices = [random.random() for _ in range(10000)]
    
    start = time.time()
    for _ in range(1000):
        engine.calculate_sma(prices, 20)
    end = time.time()
    
    print(f"SMA计算性能: {1000 / (end - start):.2f} ops/sec")
```

## 最佳实践

### 1. 策略开发

- **模块化设计**: 将策略拆分为独立的模块
- **配置外部化**: 使用配置文件管理策略参数
- **错误处理**: 完善的异常处理和日志记录
- **测试驱动**: 为每个策略编写单元测试

### 2. 性能优化

- **批量处理**: 尽可能使用批量操作
- **缓存结果**: 缓存计算结果避免重复计算
- **并行计算**: 利用多线程处理多个标的
- **内存优化**: 及时释放不需要的数据

### 3. 风险管理

- **参数验证**: 严格验证输入参数
- **边界检查**: 检查数组边界和数值范围
- **异常处理**: 优雅处理计算异常
- **资源管理**: 合理管理计算资源

这个Python FFI绑定架构为MosesQuant提供了强大的Python策略开发能力，使用户能够享受Rust的高性能计算能力，同时保持Python的开发便利性。