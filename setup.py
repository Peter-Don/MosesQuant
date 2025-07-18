#!/usr/bin/env python3
"""
MosesQuant Python绑定安装脚本

使用方法:
1. 确保已安装 Rust 和 Cargo
2. 运行: python setup.py install
3. 或者开发模式: python setup.py develop
"""

from setuptools import setup, Extension
from pybind11.setup_helpers import Pybind11Extension, build_ext
from pybind11 import get_cmake_dir
import pybind11
import os
import sys

# 检查是否有 pyo3 和 maturin
try:
    import maturin
    HAS_MATURIN = True
except ImportError:
    HAS_MATURIN = False

# 项目信息
project_name = "moses-quant"
version = "0.1.0"
description = "MosesQuant Python bindings for high-performance quantitative trading"

# 长描述
long_description = """
MosesQuant Python Bindings
==========================

MosesQuant是一个高性能量化交易框架，基于WonderTrader架构理念，
结合QuantConnect LEAN模块化设计，使用Rust实现零成本抽象。

通过这些Python绑定，用户可以:
- 使用Python编写量化交易策略
- 利用Rust底层引擎的高性能计算能力
- 访问丰富的技术指标和风险管理工具
- 享受类型安全和内存安全的保障

功能特性:
- 高性能技术指标计算（SMA, EMA, RSI, MACD, Bollinger Bands等）
- 风险指标计算（VaR, 夏普比率, 最大回撤等）
- 标准化的Alpha模型接口
- 事件驱动的策略框架
- 支持多种市场数据源

安装要求:
- Python 3.7+
- Rust 1.60+
- Cargo

使用示例:
```python
import moses_quant as mq

# 创建计算引擎
engine = mq.PyCalculationEngine()

# 计算技术指标
prices = [100, 101, 102, 103, 104, 105]
sma = engine.calculate_sma(prices, 3)
print(f"SMA: {sma}")

# 实现自定义Alpha模型
class MyAlphaModel:
    def generate_insights(self, symbols):
        # 您的策略逻辑
        pass
```

更多示例请参考 python_examples/ 目录。
"""

# 如果有 maturin，使用 maturin 构建
if HAS_MATURIN:
    print("使用 maturin 构建 Python 绑定...")
    
    # 创建 pyproject.toml
    pyproject_content = f"""
[build-system]
requires = ["maturin>=0.14,<0.15"]
build-backend = "maturin"

[project]
name = "{project_name}"
version = "{version}"
description = "{description}"
authors = [
    {{name = "MosesQuant Team", email = "support@mosesquant.com"}},
]
dependencies = [
    "numpy>=1.20.0",
    "pandas>=1.3.0",
]
requires-python = ">=3.7"
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Financial and Insurance Industry",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.7",
    "Programming Language :: Python :: 3.8",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Rust",
    "Topic :: Office/Business :: Financial",
    "Topic :: Scientific/Engineering :: Information Analysis",
]

[project.urls]
homepage = "https://github.com/your-org/MosesQuant"
repository = "https://github.com/your-org/MosesQuant"
documentation = "https://docs.mosesquant.com"

[tool.maturin]
features = ["python"]
"""
    
    with open("pyproject.toml", "w") as f:
        f.write(pyproject_content)
    
    print("pyproject.toml 已生成")
    print("请运行以下命令构建和安装:")
    print("  maturin develop  # 开发模式")
    print("  maturin build    # 构建wheel")
    print("  maturin build --release  # 构建优化版本")
    
else:
    # 备用方案: 使用传统的 setuptools
    print("未找到 maturin，使用传统方案...")
    print("请先安装 maturin:")
    print("  pip install maturin")
    
    setup(
        name=project_name,
        version=version,
        description=description,
        long_description=long_description,
        long_description_content_type="text/markdown",
        author="MosesQuant Team",
        author_email="support@mosesquant.com",
        url="https://github.com/your-org/MosesQuant",
        packages=[],
        python_requires=">=3.7",
        install_requires=[
            "numpy>=1.20.0",
            "pandas>=1.3.0",
        ],
        classifiers=[
            "Development Status :: 3 - Alpha",
            "Intended Audience :: Financial and Insurance Industry",
            "License :: OSI Approved :: MIT License",
            "Programming Language :: Python :: 3",
            "Programming Language :: Python :: 3.7",
            "Programming Language :: Python :: 3.8",
            "Programming Language :: Python :: 3.9",
            "Programming Language :: Python :: 3.10",
            "Programming Language :: Python :: 3.11",
            "Programming Language :: Rust",
            "Topic :: Office/Business :: Financial",
            "Topic :: Scientific/Engineering :: Information Analysis",
        ],
    )