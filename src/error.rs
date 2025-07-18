//! MosesQuant 错误处理系统
//! 
//! 统一的错误类型和错误处理机制

use thiserror::Error;

/// 框架统一错误类型
#[derive(Error, Debug)]
pub enum CzscError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
    
    #[error("Data error: {message}")]
    Data { message: String },
    
    #[error("Trading error: {message}")]
    Trading { message: String },
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Strategy error: {message}")]
    Strategy { message: String },
    
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Generic error: {message}")]
    Generic { message: String },
}

impl CzscError {
    /// 创建数据相关错误
    pub fn data(message: &str) -> Self {
        Self::Data {
            message: message.to_string(),
        }
    }
    
    /// 创建交易相关错误
    pub fn trading(message: &str) -> Self {
        Self::Trading {
            message: message.to_string(),
        }
    }
    
    /// 创建配置相关错误
    pub fn config(message: &str) -> Self {
        Self::Config {
            message: message.to_string(),
        }
    }
    
    /// 创建策略相关错误
    pub fn strategy(message: &str) -> Self {
        Self::Strategy {
            message: message.to_string(),
        }
    }
    
    /// 创建网络相关错误
    pub fn network(message: &str) -> Self {
        Self::Network {
            message: message.to_string(),
        }
    }
    
    /// 创建验证相关错误
    pub fn validation(message: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
        }
    }
    
    /// 创建通用错误
    pub fn generic(message: &str) -> Self {
        Self::Generic {
            message: message.to_string(),
        }
    }
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, CzscError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let error = CzscError::data("Test data error");
        assert!(matches!(error, CzscError::Data { .. }));
        assert_eq!(error.to_string(), "Data error: Test data error");
    }
    
    #[test]
    fn test_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let czsc_error = CzscError::from(io_error);
        assert!(matches!(czsc_error, CzscError::Io(_)));
    }
    
    #[test]
    fn test_result_type() {
        let success: Result<i32> = Ok(42);
        let failure: Result<i32> = Err(CzscError::generic("Test error"));
        
        assert!(success.is_ok());
        assert!(failure.is_err());
    }
}