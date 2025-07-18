#!/usr/bin/env python3
"""
MosesQuant Python策略示例

这个示例展示了如何使用Python编写量化交易策略，
利用MosesQuant的Rust底层引擎进行高性能计算。
"""

import sys
import os
from typing import List, Dict, Optional
from abc import ABC, abstractmethod

# 导入MosesQuant Python绑定
try:
    import moses_quant as mq
except ImportError:
    print("MosesQuant Python绑定未安装，请先编译安装")
    sys.exit(1)


class BasePythonAlphaModel(ABC):
    """
    Python Alpha模型基类
    
    用户需要继承此类并实现具体的策略逻辑
    """
    
    def __init__(self, name: str):
        self.name = name
        self.calculation_engine = None
        self.data_provider = None
        
    @abstractmethod
    def generate_insights(self, symbols: List[str]) -> List[mq.PyInsight]:
        """
        生成交易洞见
        
        Args:
            symbols: 需要分析的标的列表
            
        Returns:
            洞见列表
        """
        pass
    
    def initialize(self):
        """初始化策略"""
        print(f"[{self.name}] 策略初始化")
        
    def cleanup(self):
        """清理策略"""
        print(f"[{self.name}] 策略清理")


class RSIAlphaModel(BasePythonAlphaModel):
    """
    RSI Alpha模型示例
    
    基于RSI指标生成交易信号：
    - RSI > 70: 超买，生成卖出信号
    - RSI < 30: 超卖，生成买入信号
    """
    
    def __init__(self, rsi_period: int = 14, overbought: float = 70.0, oversold: float = 30.0):
        super().__init__("RSI_Alpha_Model")
        self.rsi_period = rsi_period
        self.overbought = overbought
        self.oversold = oversold
        
    def generate_insights(self, symbols: List[str]) -> List[mq.PyInsight]:
        """基于RSI指标生成交易洞见"""
        insights = []
        
        # 获取计算引擎
        calc_engine = mq.PyCalculationEngine()
        data_provider = mq.PyDataProvider()
        
        for symbol in symbols:
            try:
                # 获取历史价格数据
                prices = data_provider.get_price_history(symbol, 50)  # 获取50天历史数据
                
                if len(prices) < self.rsi_period + 1:
                    print(f"[{self.name}] {symbol}: 数据不足，跳过")
                    continue
                
                # 计算RSI
                rsi_values = calc_engine.calculate_rsi(prices, self.rsi_period)
                
                if not rsi_values:
                    print(f"[{self.name}] {symbol}: RSI计算失败，跳过")
                    continue
                
                latest_rsi = rsi_values[-1]
                print(f"[{self.name}] {symbol}: RSI = {latest_rsi:.2f}")
                
                # 生成交易信号
                if latest_rsi > self.overbought:
                    # 超买信号 - 卖出
                    insight = mq.PyInsight(symbol, "Down")
                    insight.confidence = min(0.9, (latest_rsi - self.overbought) / 10.0)
                    insight.magnitude = 1.0
                    insight.source_model = self.name
                    insights.append(insight)
                    print(f"[{self.name}] {symbol}: 生成卖出信号 (RSI={latest_rsi:.2f})")
                    
                elif latest_rsi < self.oversold:
                    # 超卖信号 - 买入
                    insight = mq.PyInsight(symbol, "Up")
                    insight.confidence = min(0.9, (self.oversold - latest_rsi) / 10.0)
                    insight.magnitude = 1.0
                    insight.source_model = self.name
                    insights.append(insight)
                    print(f"[{self.name}] {symbol}: 生成买入信号 (RSI={latest_rsi:.2f})")
                    
            except Exception as e:
                print(f"[{self.name}] {symbol}: 处理错误 - {e}")
                continue
        
        return insights


class MovingAverageCrossAlphaModel(BasePythonAlphaModel):
    """
    移动平均交叉Alpha模型示例
    
    基于快慢移动平均交叉生成交易信号：
    - 快线上穿慢线: 买入信号
    - 快线下穿慢线: 卖出信号
    """
    
    def __init__(self, fast_period: int = 10, slow_period: int = 20):
        super().__init__("MA_Cross_Alpha_Model")
        self.fast_period = fast_period
        self.slow_period = slow_period
        
    def generate_insights(self, symbols: List[str]) -> List[mq.PyInsight]:
        """基于移动平均交叉生成交易洞见"""
        insights = []
        
        # 获取计算引擎
        calc_engine = mq.PyCalculationEngine()
        data_provider = mq.PyDataProvider()
        
        for symbol in symbols:
            try:
                # 获取历史价格数据
                prices = data_provider.get_price_history(symbol, max(self.fast_period, self.slow_period) + 10)
                
                if len(prices) < self.slow_period + 2:
                    print(f"[{self.name}] {symbol}: 数据不足，跳过")
                    continue
                
                # 计算快慢移动平均
                fast_ma = calc_engine.calculate_sma(prices, self.fast_period)
                slow_ma = calc_engine.calculate_sma(prices, self.slow_period)
                
                if len(fast_ma) < 2 or len(slow_ma) < 2:
                    print(f"[{self.name}] {symbol}: 移动平均计算失败，跳过")
                    continue
                
                # 获取最新和前一个的移动平均值
                fast_current = fast_ma[-1]
                fast_previous = fast_ma[-2]
                slow_current = slow_ma[-1]
                slow_previous = slow_ma[-2]
                
                print(f"[{self.name}] {symbol}: 快线={fast_current:.2f}, 慢线={slow_current:.2f}")
                
                # 检测交叉信号
                # 金叉：快线上穿慢线
                if fast_previous <= slow_previous and fast_current > slow_current:
                    insight = mq.PyInsight(symbol, "Up")
                    insight.confidence = 0.7
                    insight.magnitude = 1.0
                    insight.source_model = self.name
                    insights.append(insight)
                    print(f"[{self.name}] {symbol}: 金叉买入信号")
                    
                # 死叉：快线下穿慢线
                elif fast_previous >= slow_previous and fast_current < slow_current:
                    insight = mq.PyInsight(symbol, "Down")
                    insight.confidence = 0.7
                    insight.magnitude = 1.0
                    insight.source_model = self.name
                    insights.append(insight)
                    print(f"[{self.name}] {symbol}: 死叉卖出信号")
                    
            except Exception as e:
                print(f"[{self.name}] {symbol}: 处理错误 - {e}")
                continue
        
        return insights


class CompositeAlphaModel(BasePythonAlphaModel):
    """
    复合Alpha模型示例
    
    结合多个Alpha模型的信号，生成综合交易洞见
    """
    
    def __init__(self, models: List[BasePythonAlphaModel]):
        super().__init__("Composite_Alpha_Model")
        self.models = models
        
    def generate_insights(self, symbols: List[str]) -> List[mq.PyInsight]:
        """综合多个模型的洞见"""
        all_insights = []
        
        # 收集所有模型的洞见
        for model in self.models:
            model_insights = model.generate_insights(symbols)
            all_insights.extend(model_insights)
        
        # 按标的分组洞见
        insights_by_symbol = {}
        for insight in all_insights:
            symbol = insight.symbol
            if symbol not in insights_by_symbol:
                insights_by_symbol[symbol] = []
            insights_by_symbol[symbol].append(insight)
        
        # 生成综合洞见
        composite_insights = []
        for symbol, symbol_insights in insights_by_symbol.items():
            if len(symbol_insights) >= 2:  # 至少需要2个模型同意
                # 计算平均置信度和方向
                up_insights = [i for i in symbol_insights if i.direction == "Up"]
                down_insights = [i for i in symbol_insights if i.direction == "Down"]
                
                if len(up_insights) > len(down_insights):
                    # 综合买入信号
                    avg_confidence = sum(i.confidence or 0.5 for i in up_insights) / len(up_insights)
                    composite_insight = mq.PyInsight(symbol, "Up")
                    composite_insight.confidence = avg_confidence
                    composite_insight.magnitude = 1.0
                    composite_insight.source_model = self.name
                    composite_insights.append(composite_insight)
                    print(f"[{self.name}] {symbol}: 综合买入信号 (置信度={avg_confidence:.2f})")
                    
                elif len(down_insights) > len(up_insights):
                    # 综合卖出信号
                    avg_confidence = sum(i.confidence or 0.5 for i in down_insights) / len(down_insights)
                    composite_insight = mq.PyInsight(symbol, "Down")
                    composite_insight.confidence = avg_confidence
                    composite_insight.magnitude = 1.0
                    composite_insight.source_model = self.name
                    composite_insights.append(composite_insight)
                    print(f"[{self.name}] {symbol}: 综合卖出信号 (置信度={avg_confidence:.2f})")
        
        return composite_insights


def demonstrate_python_strategy():
    """演示Python策略的使用"""
    print("=" * 60)
    print(f"MosesQuant Python策略演示")
    print(f"框架版本: {mq.VERSION}")
    print("=" * 60)
    
    # 测试标的
    symbols = ["BTCUSDT", "ETHUSDT", "ADAUSDT", "SOLUSDT"]
    
    # 1. RSI策略演示
    print("\n1. RSI策略演示")
    print("-" * 40)
    rsi_model = RSIAlphaModel(rsi_period=14, overbought=70.0, oversold=30.0)
    rsi_model.initialize()
    rsi_insights = rsi_model.generate_insights(symbols)
    
    print(f"RSI策略生成了 {len(rsi_insights)} 个洞见:")
    for insight in rsi_insights:
        print(f"  - {insight.symbol}: {insight.direction} (置信度: {insight.confidence:.2f})")
    
    # 2. 移动平均交叉策略演示
    print("\n2. 移动平均交叉策略演示")
    print("-" * 40)
    ma_model = MovingAverageCrossAlphaModel(fast_period=10, slow_period=20)
    ma_model.initialize()
    ma_insights = ma_model.generate_insights(symbols)
    
    print(f"移动平均交叉策略生成了 {len(ma_insights)} 个洞见:")
    for insight in ma_insights:
        print(f"  - {insight.symbol}: {insight.direction} (置信度: {insight.confidence:.2f})")
    
    # 3. 复合策略演示
    print("\n3. 复合策略演示")
    print("-" * 40)
    composite_model = CompositeAlphaModel([rsi_model, ma_model])
    composite_model.initialize()
    composite_insights = composite_model.generate_insights(symbols)
    
    print(f"复合策略生成了 {len(composite_insights)} 个洞见:")
    for insight in composite_insights:
        print(f"  - {insight.symbol}: {insight.direction} (置信度: {insight.confidence:.2f})")
    
    # 4. 计算引擎功能演示
    print("\n4. 计算引擎功能演示")
    print("-" * 40)
    calc_engine = mq.PyCalculationEngine()
    data_provider = mq.PyDataProvider()
    
    # 获取示例数据
    prices = data_provider.get_price_history("BTCUSDT", 30)
    print(f"获取到 {len(prices)} 个价格数据点")
    
    # 计算各种指标
    sma_20 = calc_engine.calculate_sma(prices, 20)
    ema_20 = calc_engine.calculate_ema(prices, 20)
    rsi_14 = calc_engine.calculate_rsi(prices, 14)
    
    print(f"SMA(20): {sma_20[-1]:.2f}")
    print(f"EMA(20): {ema_20[-1]:.2f}")
    print(f"RSI(14): {rsi_14[-1]:.2f}")
    
    # 计算收益率
    returns = [(prices[i] - prices[i-1]) / prices[i-1] for i in range(1, len(prices))]
    
    # 计算风险指标
    risk_metrics = calc_engine.calculate_risk_metrics(returns)
    print(f"波动率: {risk_metrics.volatility:.4f}")
    print(f"VaR(95%): {risk_metrics.var_95:.4f}")
    print(f"夏普比率: {risk_metrics.sharpe_ratio:.4f}")
    print(f"最大回撤: {risk_metrics.max_drawdown:.4f}")
    
    print("\n" + "=" * 60)
    print("演示完成！")
    print("=" * 60)


if __name__ == "__main__":
    demonstrate_python_strategy()