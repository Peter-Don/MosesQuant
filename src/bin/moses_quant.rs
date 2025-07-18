//! MosesQuant 主程序 - 配置驱动运行
//! 
//! 通过YAML配置文件驱动的量化交易系统

use czsc_core::{
    config::{ConfigManager, generate_default_config_file},
    runner::StrategyRunner,
    Result,
};
use std::env;
use std::path::Path;

/// 程序入口点
#[tokio::main]
async fn main() {
    // 初始化日志系统
    tracing_subscriber::fmt::init();
    
    // 运行主逻辑并处理错误
    match run_main().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("❌ 程序运行失败: {}", e);
            std::process::exit(1);
        }
    }
}

/// 主要逻辑函数
async fn run_main() -> Result<()> {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    
    match args.len() {
        1 => {
            // 没有参数，使用默认配置
            run_with_default_config().await
        }
        2 => {
            let command = &args[1];
            match command.as_str() {
                "init" => {
                    // 生成默认配置文件
                    generate_config_file().await
                }
                path => {
                    // 使用指定的配置文件
                    run_with_config_file(path).await
                }
            }
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

/// 使用默认配置运行
async fn run_with_default_config() -> Result<()> {
    tracing::info!("🚀 启动 MosesQuant 量化交易系统 (默认配置)");
    
    // 创建默认配置
    let config_manager = ConfigManager::new_default();
    config_manager.validate()?;
    
    // 创建并运行策略
    let mut runner = StrategyRunner::from_config(config_manager.get_config().clone()).await?;
    
    // 运行策略
    let results = runner.run_strategies().await?;
    
    // 显示结果
    display_results(&results).await;
    
    // 显示统计信息
    let stats = runner.get_data_stats().await;
    tracing::info!("📊 数据统计: 请求={}, 缓存命中率={:.2}%", 
        stats.requests, stats.cache_hit_rate() * 100.0);
    
    tracing::info!("🎉 策略运行完成");
    Ok(())
}

/// 使用配置文件运行
async fn run_with_config_file(config_path: &str) -> Result<()> {
    tracing::info!("🚀 启动 MosesQuant 量化交易系统");
    tracing::info!("📄 配置文件: {}", config_path);
    
    // 检查配置文件是否存在
    if !Path::new(config_path).exists() {
        tracing::error!("❌ 配置文件不存在: {}", config_path);
        tracing::info!("💡 使用 'moses_quant init' 生成默认配置文件");
        return Ok(());
    }
    
    // 加载配置
    let config_manager = ConfigManager::load_from_file(config_path).await?;
    config_manager.validate()?;
    
    let config = config_manager.get_config();
    tracing::info!("🏗️  框架: {} v{}", config.framework.name, config.framework.version);
    tracing::info!("💰 初始资金: ${:.2}", config.framework.initial_capital);
    tracing::info!("🔧 运行模式: {:?}", config.framework.mode);
    
    // 创建并运行策略
    let mut runner = StrategyRunner::from_config(config.clone()).await?;
    
    // 运行策略
    let results = runner.run_strategies().await?;
    
    // 显示结果
    display_results(&results).await;
    
    // 显示统计信息
    let stats = runner.get_data_stats().await;
    tracing::info!("📊 数据统计: 请求={}, 缓存命中率={:.2}%", 
        stats.requests, stats.cache_hit_rate() * 100.0);
    
    tracing::info!("🎉 策略运行完成");
    Ok(())
}

/// 生成默认配置文件
async fn generate_config_file() -> Result<()> {
    let config_path = "moses_quant_config.yaml";
    
    tracing::info!("📝 生成默认配置文件: {}", config_path);
    
    generate_default_config_file(config_path).await?;
    
    tracing::info!("✅ 配置文件生成完成");
    tracing::info!("🔧 请编辑配置文件后运行: moses_quant {}", config_path);
    
    Ok(())
}

/// 显示策略运行结果
async fn display_results(results: &[czsc_core::strategy::StrategyResult]) {
    tracing::info!("📈 策略运行结果:");
    
    for (i, result) in results.iter().enumerate() {
        tracing::info!("  策略 {} 结果:", i + 1);
        
        if result.success {
            tracing::info!("    ✅ 执行成功");
            tracing::info!("    🎯 标的数量: {}", result.universe_size);
            tracing::info!("    🧠 洞见生成: {}", result.insights_generated);
            tracing::info!("    📊 投资组合目标: {}", result.targets_created);
            tracing::info!("    📋 订单生成: {}", result.orders_generated);
            tracing::info!("    ⏱️  执行时间: {:?}", result.execution_time);
            
            // 显示订单详情
            if !result.orders.is_empty() {
                tracing::info!("    📋 生成的订单:");
                for order in &result.orders {
                    tracing::info!("      - {} {} {} {:.6} @ {:?}",
                        order.order_id[..8].to_string() + "...",
                        order.symbol.value,
                        match order.direction {
                            czsc_core::Direction::Long => "BUY",
                            czsc_core::Direction::Short => "SELL",
                        },
                        order.quantity,
                        order.price
                    );
                }
            }
        } else {
            tracing::warn!("    ❌ 执行失败");
        }
    }
}

/// 打印使用说明
fn print_usage() {
    println!("MosesQuant 量化交易系统");
    println!();
    println!("用法:");
    println!("  moses_quant                    # 使用默认配置运行");
    println!("  moses_quant init               # 生成默认配置文件");
    println!("  moses_quant <config_file>      # 使用指定配置文件运行");
    println!();
    println!("示例:");
    println!("  moses_quant init");
    println!("  moses_quant moses_quant_config.yaml");
    println!();
    println!("配置文件格式: YAML");
    println!("更多信息请查看文档: https://github.com/your-repo/moses-quant");
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_generate_config_file() {
        let test_config_path = "test_config.yaml";
        
        // 生成配置文件
        generate_default_config_file(test_config_path).await.unwrap();
        
        // 验证文件存在
        assert!(Path::new(test_config_path).exists());
        
        // 验证可以加载配置
        let config_manager = ConfigManager::load_from_file(test_config_path).await.unwrap();
        assert!(config_manager.validate().is_ok());
        
        // 清理测试文件
        let _ = tokio::fs::remove_file(test_config_path).await;
    }
}