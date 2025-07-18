//! Python FFI绑定模块
//! 
//! 为Python用户提供MosesQuant框架的策略开发接口

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use crate::{
    strategy::{AlphaModel, StrategyContext, AlphaModelConfig, StatisticType, RiskMetrics},
    types::{Symbol, Insight, InsightDirection, AssetType},
    indicators::CalculationEngineImpl,
    Result, CzscError,
};

#[cfg(feature = "python")]
use std::collections::HashMap;

#[cfg(feature = "python")]
use async_trait::async_trait;

#[cfg(feature = "python")]
use std::sync::Arc;

/// Python Alpha模型包装器
/// 
/// 允许Python代码实现AlphaModel接口
#[cfg(feature = "python")]
#[pyclass]
pub struct PyAlphaModel {
    /// Python对象
    python_obj: PyObject,
    /// 模型名称
    name: String,
    /// 模型配置
    config: AlphaModelConfig,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyAlphaModel {
    #[new]
    fn new(python_obj: PyObject, name: String) -> Self {
        Self {
            python_obj,
            name,
            config: AlphaModelConfig {
                name: "PyAlphaModel".to_string(),
                enable_fast_path: false,
                signal_decay_time: None,
                parameters: std::collections::HashMap::new(),
            },
        }
    }
    
    /// 获取模型名称
    fn get_name(&self) -> &str {
        &self.name
    }
}

#[cfg(feature = "python")]
#[async_trait]
impl AlphaModel for PyAlphaModel {
    async fn generate_insights(&self, _context: &StrategyContext, symbols: &[Symbol]) -> Result<Vec<Insight>> {
        Python::with_gil(|py| {
            // 将Rust数据转换为Python对象
            let py_symbols = symbols.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            
            // 调用Python方法
            let result = self.python_obj
                .call_method1(py, "generate_insights", (py_symbols,))
                .map_err(|e| CzscError::strategy(&format!("Python method call failed: {}", e)))?;
            
            // 将Python结果转换回Rust类型
            let py_insights: Vec<PyInsight> = result.extract(py)
                .map_err(|e| CzscError::strategy(&format!("Failed to extract insights: {}", e)))?;
            
            let insights = py_insights.into_iter()
                .map(|py_insight| py_insight.to_rust())
                .collect();
            
            Ok(insights)
        })
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn config(&self) -> &AlphaModelConfig {
        &self.config
    }
    
    async fn initialize(&mut self, _context: &StrategyContext) -> Result<()> {
        Python::with_gil(|py| {
            self.python_obj
                .call_method0(py, "initialize")
                .map_err(|e| CzscError::strategy(&format!("Python initialize failed: {}", e)))?;
            Ok(())
        })
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        Python::with_gil(|py| {
            self.python_obj
                .call_method0(py, "cleanup")
                .map_err(|e| CzscError::strategy(&format!("Python cleanup failed: {}", e)))?;
            Ok(())
        })
    }
}

/// Python洞见数据结构
#[cfg(feature = "python")]
#[pyclass]
#[derive(Clone)]
pub struct PyInsight {
    #[pyo3(get, set)]
    pub symbol: String,
    #[pyo3(get, set)]
    pub direction: String,  // "Up", "Down", "Flat"
    #[pyo3(get, set)]
    pub confidence: Option<f64>,
    #[pyo3(get, set)]
    pub magnitude: Option<f64>,
    #[pyo3(get, set)]
    pub weight: Option<f64>,
    #[pyo3(get, set)]
    pub source_model: Option<String>,
    #[pyo3(get, set)]
    pub generated_time_utc: i64,
    #[pyo3(get, set)]
    pub close_time_utc: Option<i64>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyInsight {
    #[new]
    fn new(symbol: String, direction: String) -> Self {
        Self {
            symbol,
            direction,
            confidence: None,
            magnitude: None,
            weight: None,
            source_model: None,
            generated_time_utc: chrono::Utc::now().timestamp_millis(),
            close_time_utc: None,
        }
    }
    
    /// 计算洞见评分
    fn score(&self) -> f64 {
        let confidence = self.confidence.unwrap_or(0.5);
        let magnitude = self.magnitude.unwrap_or(1.0);
        confidence * magnitude
    }
}

#[cfg(feature = "python")]
impl PyInsight {
    /// 转换为Rust Insight类型
    pub fn to_rust(&self) -> Insight {
        let direction = match self.direction.as_str() {
            "Up" => InsightDirection::Up,
            "Down" => InsightDirection::Down,
            _ => InsightDirection::Flat,
        };
        
        Insight {
            symbol: Symbol::new(&self.symbol, "BINANCE", AssetType::Crypto), // 默认市场和资产类型
            direction,
            magnitude: self.magnitude,
            confidence: self.confidence,
            period: None,
            generated_time: self.generated_time_utc,
            expiry_time: self.close_time_utc,
        }
    }
    
    /// 从Rust Insight类型创建
    pub fn from_rust(insight: &Insight) -> Self {
        let direction = match insight.direction {
            InsightDirection::Up => "Up".to_string(),
            InsightDirection::Down => "Down".to_string(),
            InsightDirection::Flat => "Flat".to_string(),
        };
        
        Self {
            symbol: insight.symbol.to_string(),
            direction,
            confidence: insight.confidence,
            magnitude: insight.magnitude,
            weight: None, // 这个字段在 Rust Insight 中不存在
            source_model: None, // 这个字段在 Rust Insight 中不存在
            generated_time_utc: insight.generated_time,
            close_time_utc: insight.expiry_time,
        }
    }
}

/// Python计算引擎包装器
#[cfg(feature = "python")]
#[pyclass]
pub struct PyCalculationEngine {
    engine: Arc<CalculationEngineImpl>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyCalculationEngine {
    #[new]
    fn new() -> Self {
        Self {
            engine: Arc::new(CalculationEngineImpl::new()),
        }
    }
    
    /// 计算简单移动平均 (SMA)
    fn calculate_sma(&self, prices: Vec<f64>, period: usize) -> PyResult<Vec<f64>> {
        self.engine.calculate_sma(&prices, period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算指数移动平均 (EMA)
    fn calculate_ema(&self, prices: Vec<f64>, period: usize) -> PyResult<Vec<f64>> {
        self.engine.calculate_ema(&prices, period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算相对强弱指标 (RSI)
    fn calculate_rsi(&self, prices: Vec<f64>, period: usize) -> PyResult<Vec<f64>> {
        self.engine.calculate_rsi(&prices, period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算MACD指标
    fn calculate_macd(&self, prices: Vec<f64>, fast_period: usize, slow_period: usize, signal_period: usize) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
        self.engine.calculate_macd(&prices, fast_period, slow_period, signal_period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算布林带
    fn calculate_bollinger_bands(&self, prices: Vec<f64>, period: usize, std_multiplier: f64) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
        self.engine.calculate_bollinger_bands(&prices, period, std_multiplier)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算统计指标
    fn calculate_statistic(&self, data: Vec<f64>, stat_type: String) -> PyResult<f64> {
        let stat_type = match stat_type.as_str() {
            "mean" => StatisticType::Mean,
            "median" => StatisticType::Median,
            "std_dev" => StatisticType::StdDev,
            "variance" => StatisticType::Variance,
            "skewness" => StatisticType::Skewness,
            "kurtosis" => StatisticType::Kurtosis,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid statistic type")),
        };
        
        self.engine.calculate_statistic(&data, stat_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算相关性
    fn calculate_correlation(&self, series1: Vec<f64>, series2: Vec<f64>) -> PyResult<f64> {
        self.engine.calculate_correlation(&series1, &series2)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
    
    /// 计算风险指标
    fn calculate_risk_metrics(&self, returns: Vec<f64>) -> PyResult<PyRiskMetrics> {
        let risk_metrics = self.engine.calculate_risk_metrics(&returns)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(PyRiskMetrics::from_rust(&risk_metrics))
    }
}

/// Python风险指标数据结构
#[cfg(feature = "python")]
#[pyclass]
#[derive(Clone)]
pub struct PyRiskMetrics {
    #[pyo3(get, set)]
    pub volatility: f64,
    #[pyo3(get, set)]
    pub var_95: f64,
    #[pyo3(get, set)]
    pub var_99: f64,
    #[pyo3(get, set)]
    pub max_drawdown: f64,
    #[pyo3(get, set)]
    pub sharpe_ratio: f64,
    #[pyo3(get, set)]
    pub sortino_ratio: f64,
    #[pyo3(get, set)]
    pub calmar_ratio: f64,
}

#[cfg(feature = "python")]
impl PyRiskMetrics {
    /// 从Rust RiskMetrics类型创建
    pub fn from_rust(risk_metrics: &RiskMetrics) -> Self {
        Self {
            volatility: risk_metrics.volatility,
            var_95: risk_metrics.var_95,
            var_99: risk_metrics.var_99,
            max_drawdown: risk_metrics.max_drawdown,
            sharpe_ratio: risk_metrics.sharpe_ratio,
            sortino_ratio: risk_metrics.sortino_ratio,
            calmar_ratio: risk_metrics.calmar_ratio,
        }
    }
}

/// Python数据访问器
#[cfg(feature = "python")]
#[pyclass]
pub struct PyDataProvider {
    // 这里暂时为空，实际使用时需要连接到真实的数据源
}

#[cfg(feature = "python")]
#[pymethods]
impl PyDataProvider {
    #[new]
    fn new() -> Self {
        Self {}
    }
    
    /// 获取历史价格数据（模拟实现）
    fn get_price_history(&self, _symbol: String, days: usize) -> PyResult<Vec<f64>> {
        // 这里是模拟数据，实际实现需要连接到真实数据源
        let mut prices = Vec::new();
        let base_price = 100.0;
        
        for i in 0..days {
            let price = base_price + (i as f64 * 0.5) + (rand::random::<f64>() - 0.5) * 10.0;
            prices.push(price);
        }
        
        Ok(prices)
    }
    
    /// 获取市场数据快照（模拟实现）
    fn get_market_snapshot(&self, symbols: Vec<String>) -> PyResult<HashMap<String, f64>> {
        let mut snapshot = HashMap::new();
        
        for symbol in symbols {
            let price = 100.0 + (rand::random::<f64>() - 0.5) * 20.0;
            snapshot.insert(symbol, price);
        }
        
        Ok(snapshot)
    }
}

/// Python策略上下文
#[cfg(feature = "python")]
#[pyclass]
pub struct PyStrategyContext {
    calculation_engine: PyCalculationEngine,
    data_provider: PyDataProvider,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyStrategyContext {
    #[new]
    fn new() -> Self {
        Self {
            calculation_engine: PyCalculationEngine::new(),
            data_provider: PyDataProvider::new(),
        }
    }
    
    /// 获取计算引擎
    fn get_calculation_engine(&self) -> PyCalculationEngine {
        PyCalculationEngine::new()
    }
    
    /// 获取数据提供器
    fn get_data_provider(&self) -> PyDataProvider {
        PyDataProvider::new()
    }
}

/// Python模块初始化
#[cfg(feature = "python")]
#[pymodule]
fn moses_quant(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAlphaModel>()?;
    m.add_class::<PyInsight>()?;
    m.add_class::<PyCalculationEngine>()?;
    m.add_class::<PyRiskMetrics>()?;
    m.add_class::<PyDataProvider>()?;
    m.add_class::<PyStrategyContext>()?;
    
    // 添加常量
    m.add("FRAMEWORK_NAME", crate::FRAMEWORK_NAME)?;
    m.add("VERSION", crate::VERSION)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "python")]
    #[test]
    fn test_py_insight_conversion() {
        let py_insight = PyInsight {
            symbol: "BTCUSDT".to_string(),
            direction: "Up".to_string(),
            confidence: Some(0.8),
            magnitude: Some(1.5),
            weight: Some(0.1),
            source_model: Some("TestModel".to_string()),
            generated_time_utc: 1234567890,
            close_time_utc: None,
        };
        
        let rust_insight = py_insight.to_rust();
        assert_eq!(rust_insight.symbol.value, "BTCUSDT");
        assert!(matches!(rust_insight.direction, InsightDirection::Up));
        assert_eq!(rust_insight.confidence, Some(0.8));
    }
    
    #[cfg(feature = "python")]
    #[test]
    fn test_calculation_engine_creation() {
        let engine = PyCalculationEngine::new();
        // 测试引擎可以正常创建
        assert!(true);
    }
}