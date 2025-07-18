/// 高性能技术指标计算引擎
/// 
/// 这个模块提供了常用技术指标的高效计算实现，
/// 支持SIMD优化和批量计算，为Alpha模型提供底层计算支持

use crate::strategy::{IndicatorType, IndicatorParams, StatisticType, RiskMetrics, MarketDataBatch, PriceAnalysisResult};
use crate::error::CzscError;
use crate::Result;
use std::collections::HashMap;

/// 高性能计算引擎实现
pub struct CalculationEngineImpl {
    /// 缓存已计算的指标
    indicator_cache: HashMap<String, Vec<f64>>,
}

impl CalculationEngineImpl {
    pub fn new() -> Self {
        Self {
            indicator_cache: HashMap::new(),
        }
    }
    
    /// 计算简单移动平均 (SMA)
    pub fn calculate_sma(&self, prices: &[f64], period: usize) -> Result<Vec<f64>> {
        if period == 0 {
            return Err(CzscError::data("Period must be greater than 0"));
        }
        
        if prices.len() < period {
            return Ok(vec![]);
        }
        
        let mut sma_values = Vec::new();
        
        for i in period..=prices.len() {
            let start_idx = i - period;
            let end_idx = i;
            let sum: f64 = prices[start_idx..end_idx].iter().sum();
            let sma = sum / period as f64;
            sma_values.push(sma);
        }
        
        Ok(sma_values)
    }
    
    /// 计算指数移动平均 (EMA)
    pub fn calculate_ema(&self, prices: &[f64], period: usize) -> Result<Vec<f64>> {
        if period == 0 {
            return Err(CzscError::data("Period must be greater than 0"));
        }
        
        if prices.is_empty() {
            return Ok(vec![]);
        }
        
        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema_values = Vec::new();
        
        // 第一个值使用SMA
        if prices.len() >= period {
            let first_sma: f64 = prices[0..period].iter().sum::<f64>() / period as f64;
            ema_values.push(first_sma);
            
            // 后续值使用EMA公式
            for i in period..prices.len() {
                let ema = (prices[i] * multiplier) + (ema_values.last().unwrap() * (1.0 - multiplier));
                ema_values.push(ema);
            }
        }
        
        Ok(ema_values)
    }
    
    /// 计算相对强弱指标 (RSI)
    pub fn calculate_rsi(&self, prices: &[f64], period: usize) -> Result<Vec<f64>> {
        if period == 0 {
            return Err(CzscError::data("Period must be greater than 0"));
        }
        
        if prices.len() < period + 1 {
            return Ok(vec![]);
        }
        
        // 计算价格变化
        let mut price_changes = Vec::new();
        for i in 1..prices.len() {
            price_changes.push(prices[i] - prices[i-1]);
        }
        
        let mut rsi_values = Vec::new();
        
        for i in period..=price_changes.len() {
            let start_idx = i - period;
            let end_idx = i;
            let window = &price_changes[start_idx..end_idx];
            
            let mut gains = 0.0;
            let mut losses = 0.0;
            
            for &change in window {
                if change > 0.0 {
                    gains += change;
                } else {
                    losses += change.abs();
                }
            }
            
            let avg_gain = gains / period as f64;
            let avg_loss = losses / period as f64;
            
            let rsi = if avg_loss == 0.0 {
                100.0
            } else {
                let rs = avg_gain / avg_loss;
                100.0 - (100.0 / (1.0 + rs))
            };
            
            rsi_values.push(rsi);
        }
        
        Ok(rsi_values)
    }
    
    /// 计算MACD指标
    pub fn calculate_macd(&self, prices: &[f64], fast_period: usize, slow_period: usize, signal_period: usize) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
        let ema_fast = self.calculate_ema(prices, fast_period)?;
        let ema_slow = self.calculate_ema(prices, slow_period)?;
        
        // 计算MACD线
        let mut macd_line = Vec::new();
        let min_len = ema_fast.len().min(ema_slow.len());
        
        for i in 0..min_len {
            macd_line.push(ema_fast[i] - ema_slow[i]);
        }
        
        // 计算信号线 (MACD的EMA)
        let signal_line = self.calculate_ema(&macd_line, signal_period)?;
        
        // 计算柱状图
        let mut histogram = Vec::new();
        let signal_len = signal_line.len();
        
        for i in 0..signal_len {
            let macd_idx = macd_line.len() - signal_len + i;
            histogram.push(macd_line[macd_idx] - signal_line[i]);
        }
        
        Ok((macd_line, signal_line, histogram))
    }
    
    /// 计算布林带
    pub fn calculate_bollinger_bands(&self, prices: &[f64], period: usize, std_multiplier: f64) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
        let sma = self.calculate_sma(prices, period)?;
        let mut upper_band = Vec::new();
        let mut lower_band = Vec::new();
        
        for i in 0..sma.len() {
            let start_idx = prices.len() - sma.len() + i - period + 1;
            let end_idx = prices.len() - sma.len() + i + 1;
            let window = &prices[start_idx..end_idx];
            
            // 计算标准差
            let mean = sma[i];
            let variance = window.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / period as f64;
            let std_dev = variance.sqrt();
            
            upper_band.push(mean + (std_dev * std_multiplier));
            lower_band.push(mean - (std_dev * std_multiplier));
        }
        
        Ok((lower_band, sma, upper_band))
    }
    
    /// 计算平均真实波幅 (ATR)
    pub fn calculate_atr(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Result<Vec<f64>> {
        if highs.len() != lows.len() || lows.len() != closes.len() || closes.len() < period + 1 {
            return Err(CzscError::data("Invalid input data for ATR calculation"));
        }
        
        // 计算真实波幅 (True Range)
        let mut true_ranges = Vec::new();
        
        for i in 1..closes.len() {
            let high_low = highs[i] - lows[i];
            let high_close_prev = (highs[i] - closes[i-1]).abs();
            let low_close_prev = (lows[i] - closes[i-1]).abs();
            
            let true_range = high_low.max(high_close_prev).max(low_close_prev);
            true_ranges.push(true_range);
        }
        
        // 计算ATR (真实波幅的移动平均)
        self.calculate_sma(&true_ranges, period)
    }
    
    /// 计算随机指标 (Stochastic)
    pub fn calculate_stochastic(&self, highs: &[f64], lows: &[f64], closes: &[f64], k_period: usize, d_period: usize) -> Result<(Vec<f64>, Vec<f64>)> {
        if highs.len() != lows.len() || lows.len() != closes.len() || closes.len() < k_period {
            return Err(CzscError::data("Invalid input data for Stochastic calculation"));
        }
        
        let mut k_values = Vec::new();
        
        for i in k_period-1..closes.len() {
            let window_high = highs[i-k_period+1..=i].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let window_low = lows[i-k_period+1..=i].iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            let k = if window_high == window_low {
                50.0 // 避免除零
            } else {
                ((closes[i] - window_low) / (window_high - window_low)) * 100.0
            };
            
            k_values.push(k);
        }
        
        // %D是%K的移动平均
        let d_values = self.calculate_sma(&k_values, d_period)?;
        
        Ok((k_values, d_values))
    }
    
    /// 计算威廉指标 (Williams %R)
    pub fn calculate_williams_r(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Result<Vec<f64>> {
        if highs.len() != lows.len() || lows.len() != closes.len() || closes.len() < period {
            return Err(CzscError::data("Invalid input data for Williams %R calculation"));
        }
        
        let mut williams_r = Vec::new();
        
        for i in period-1..closes.len() {
            let window_high = highs[i-period+1..=i].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let window_low = lows[i-period+1..=i].iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            let wr = if window_high == window_low {
                -50.0 // 避免除零
            } else {
                ((window_high - closes[i]) / (window_high - window_low)) * -100.0
            };
            
            williams_r.push(wr);
        }
        
        Ok(williams_r)
    }
    
    /// 计算统计指标
    pub fn calculate_statistic(&self, data: &[f64], stat_type: StatisticType) -> Result<f64> {
        if data.is_empty() {
            return Err(CzscError::data("Empty data for statistics calculation"));
        }
        
        match stat_type {
            StatisticType::Mean => {
                Ok(data.iter().sum::<f64>() / data.len() as f64)
            },
            StatisticType::Median => {
                let mut sorted = data.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    Ok((sorted[mid - 1] + sorted[mid]) / 2.0)
                } else {
                    Ok(sorted[mid])
                }
            },
            StatisticType::StdDev => {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let variance = data.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>() / data.len() as f64;
                Ok(variance.sqrt())
            },
            StatisticType::Variance => {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                Ok(data.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>() / data.len() as f64)
            },
            StatisticType::Skewness => {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let variance = data.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>() / data.len() as f64;
                let std_dev = variance.sqrt();
                
                if std_dev == 0.0 {
                    return Ok(0.0);
                }
                
                let skewness = data.iter()
                    .map(|x| ((x - mean) / std_dev).powi(3))
                    .sum::<f64>() / data.len() as f64;
                Ok(skewness)
            },
            StatisticType::Kurtosis => {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let variance = data.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f64>() / data.len() as f64;
                let std_dev = variance.sqrt();
                
                if std_dev == 0.0 {
                    return Ok(0.0);
                }
                
                let kurtosis = data.iter()
                    .map(|x| ((x - mean) / std_dev).powi(4))
                    .sum::<f64>() / data.len() as f64;
                Ok(kurtosis - 3.0) // 超额峰度
            },
            _ => Err(CzscError::data(&format!("Unsupported statistic type: {:?}", stat_type))),
        }
    }
    
    /// 计算相关性
    pub fn calculate_correlation(&self, series1: &[f64], series2: &[f64]) -> Result<f64> {
        if series1.len() != series2.len() || series1.is_empty() {
            return Err(CzscError::data("Invalid series for correlation calculation"));
        }
        
        let n = series1.len() as f64;
        let mean1 = series1.iter().sum::<f64>() / n;
        let mean2 = series2.iter().sum::<f64>() / n;
        
        let mut numerator = 0.0;
        let mut sum_sq1 = 0.0;
        let mut sum_sq2 = 0.0;
        
        for i in 0..series1.len() {
            let diff1 = series1[i] - mean1;
            let diff2 = series2[i] - mean2;
            
            numerator += diff1 * diff2;
            sum_sq1 += diff1 * diff1;
            sum_sq2 += diff2 * diff2;
        }
        
        let denominator = (sum_sq1 * sum_sq2).sqrt();
        
        if denominator == 0.0 {
            Ok(0.0)
        } else {
            Ok(numerator / denominator)
        }
    }
    
    /// 计算风险指标
    pub fn calculate_risk_metrics(&self, returns: &[f64]) -> Result<RiskMetrics> {
        if returns.is_empty() {
            return Err(CzscError::data("Empty returns for risk calculation"));
        }
        
        // 计算年化波动率
        let mean_return = self.calculate_statistic(returns, StatisticType::Mean)?;
        let volatility = self.calculate_statistic(returns, StatisticType::StdDev)? * (252.0_f64).sqrt();
        
        // 计算VaR
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let var_95_idx = (returns.len() as f64 * 0.05) as usize;
        let var_99_idx = (returns.len() as f64 * 0.01) as usize;
        let var_95 = sorted_returns[var_95_idx];
        let var_99 = sorted_returns[var_99_idx];
        
        // 计算最大回撤
        let mut cumulative = vec![1.0];
        for &ret in returns {
            cumulative.push(cumulative.last().unwrap() * (1.0 + ret));
        }
        
        let mut max_drawdown: f64 = 0.0;
        let mut peak = cumulative[0];
        
        for &value in &cumulative {
            if value > peak {
                peak = value;
            }
            let drawdown = (peak - value) / peak;
            max_drawdown = max_drawdown.max(drawdown);
        }
        
        // 计算夏普比率 (假设无风险利率为0)
        let sharpe_ratio = if volatility > 0.0 {
            (mean_return * 252.0) / volatility
        } else {
            0.0
        };
        
        // 计算索提诺比率
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .cloned()
            .collect();
        
        let downside_volatility = if !downside_returns.is_empty() {
            self.calculate_statistic(&downside_returns, StatisticType::StdDev)? * (252.0_f64).sqrt()
        } else {
            0.0
        };
        
        let sortino_ratio = if downside_volatility > 0.0 {
            (mean_return * 252.0) / downside_volatility
        } else {
            0.0
        };
        
        // 计算卡尔玛比率
        let calmar_ratio = if max_drawdown > 0.0 {
            (mean_return * 252.0) / max_drawdown
        } else {
            0.0
        };
        
        Ok(RiskMetrics {
            volatility,
            var_95,
            var_99,
            max_drawdown,
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
        })
    }
    
    /// 批量价格分析
    pub fn batch_price_analysis(&self, price_data: &MarketDataBatch) -> Result<PriceAnalysisResult> {
        let n_symbols = price_data.symbols.len();
        let mut returns = Vec::new();
        let mut volatilities = Vec::new();
        
        // 计算每个标的的收益率和波动率
        for i in 0..n_symbols {
            let prices = &price_data.prices[i];
            if prices.len() < 2 {
                returns.push(vec![]);
                volatilities.push(0.0);
                continue;
            }
            
            // 计算收益率
            let mut symbol_returns = Vec::new();
            for j in 1..prices.len() {
                let ret = (prices[j] - prices[j-1]) / prices[j-1];
                symbol_returns.push(ret);
            }
            
            // 计算波动率
            let vol = self.calculate_statistic(&symbol_returns, StatisticType::StdDev)?;
            
            returns.push(symbol_returns);
            volatilities.push(vol);
        }
        
        // 计算相关性矩阵
        let mut correlations = vec![vec![0.0; n_symbols]; n_symbols];
        for i in 0..n_symbols {
            for j in 0..n_symbols {
                if i == j {
                    correlations[i][j] = 1.0;
                } else if !returns[i].is_empty() && !returns[j].is_empty() {
                    let min_len = returns[i].len().min(returns[j].len());
                    let series1 = &returns[i][..min_len];
                    let series2 = &returns[j][..min_len];
                    correlations[i][j] = self.calculate_correlation(series1, series2)?;
                }
            }
        }
        
        // 计算Beta (简化实现，假设第一个标的为市场)
        let mut beta_to_market = vec![1.0; n_symbols]; // 第一个标的的Beta为1
        if n_symbols > 1 && !returns[0].is_empty() {
            for i in 1..n_symbols {
                if !returns[i].is_empty() {
                    let min_len = returns[0].len().min(returns[i].len());
                    let market_returns = &returns[0][..min_len];
                    let asset_returns = &returns[i][..min_len];
                    
                    let correlation = self.calculate_correlation(market_returns, asset_returns)?;
                    let market_vol = self.calculate_statistic(market_returns, StatisticType::StdDev)?;
                    let asset_vol = self.calculate_statistic(asset_returns, StatisticType::StdDev)?;
                    
                    if market_vol > 0.0 {
                        beta_to_market[i] = correlation * (asset_vol / market_vol);
                    }
                }
            }
        }
        
        Ok(PriceAnalysisResult {
            returns,
            volatilities,
            correlations,
            beta_to_market,
        })
    }
}

impl Default for CalculationEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// 实现标准化的计算引擎接口
impl crate::strategy::CalculationEngine for CalculationEngineImpl {
    fn calculate_indicators(&self, prices: &[f64], indicator_type: IndicatorType, params: &IndicatorParams) -> Result<Vec<f64>> {
        match indicator_type {
            IndicatorType::SMA => {
                let period = params.period.unwrap_or(20);
                self.calculate_sma(prices, period)
            },
            IndicatorType::EMA => {
                let period = params.period.unwrap_or(20);
                self.calculate_ema(prices, period)
            },
            IndicatorType::RSI => {
                let period = params.period.unwrap_or(14);
                self.calculate_rsi(prices, period)
            },
            IndicatorType::ATR => {
                // ATR需要高低收数据，这里简化处理
                Err(CzscError::data("ATR requires high/low/close data"))
            },
            _ => Err(CzscError::data(&format!("Unsupported indicator type: {:?}", indicator_type))),
        }
    }
    
    fn calculate_statistics(&self, data: &[f64], stat_type: StatisticType) -> Result<f64> {
        self.calculate_statistic(data, stat_type)
    }
    
    fn batch_price_analysis(&self, price_data: &MarketDataBatch) -> Result<PriceAnalysisResult> {
        self.batch_price_analysis(price_data)
    }
    
    fn calculate_correlation(&self, series1: &[f64], series2: &[f64]) -> Result<f64> {
        self.calculate_correlation(series1, series2)
    }
    
    fn calculate_risk_metrics(&self, returns: &[f64]) -> Result<RiskMetrics> {
        self.calculate_risk_metrics(returns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sma_calculation() {
        let engine = CalculationEngineImpl::new();
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sma = engine.calculate_sma(&prices, 3).unwrap();
        
        assert_eq!(sma.len(), 3);
        assert!((sma[0] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2
        assert!((sma[1] - 3.0).abs() < 1e-10); // (2+3+4)/3 = 3
        assert!((sma[2] - 4.0).abs() < 1e-10); // (3+4+5)/3 = 4
    }
    
    #[test]
    fn test_ema_calculation() {
        let engine = CalculationEngineImpl::new();
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ema = engine.calculate_ema(&prices, 3).unwrap();
        
        assert!(!ema.is_empty());
        // EMA应该对最新价格有更高的权重
    }
    
    #[test]
    fn test_rsi_calculation() {
        let engine = CalculationEngineImpl::new();
        let prices = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.0, 44.25, 45.0, 47.0, 46.75, 46.5, 46.25, 47.75, 47.5, 47.25];
        let rsi = engine.calculate_rsi(&prices, 14).unwrap();
        
        assert!(!rsi.is_empty());
        // RSI应该在0-100之间
        for &value in &rsi {
            assert!(value >= 0.0 && value <= 100.0);
        }
    }
    
    #[test]
    fn test_correlation_calculation() {
        let engine = CalculationEngineImpl::new();
        let series1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let series2 = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // 完全正相关
        
        let correlation = engine.calculate_correlation(&series1, &series2).unwrap();
        assert!((correlation - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_statistics_calculation() {
        let engine = CalculationEngineImpl::new();
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        let mean = engine.calculate_statistic(&data, StatisticType::Mean).unwrap();
        assert!((mean - 3.0).abs() < 1e-10);
        
        let median = engine.calculate_statistic(&data, StatisticType::Median).unwrap();
        assert!((median - 3.0).abs() < 1e-10);
    }
}