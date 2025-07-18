# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **MosesQuant** - a high-performance Rust quantitative trading framework that combines WonderTrader's architecture with QuantConnect LEAN's modular design. The project implements a five-stage strategy pipeline and provides comprehensive market data processing, strategy execution, and risk management capabilities.

## Common Development Commands

### Building and Testing
```bash
# Check compilation without building
cargo check

# Build development version
cargo build

# Build optimized release version
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run clippy for code quality
cargo clippy

# Format code
cargo fmt
```

### Development Workflow
```bash
# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Generate documentation
cargo doc --open

# Check dependency tree
cargo tree
```

## Architecture Overview

### Core Design Principles

1. **Five-Stage Strategy Pipeline**: Universe Selection → Alpha Creation → Portfolio Construction → Risk Management → Execution
2. **Modular Architecture**: Pluggable components with well-defined traits
3. **Async-First Design**: Built on Tokio for high-performance concurrent operations
4. **Type Safety**: Comprehensive error handling with unified `Result<T>` type
5. **Configuration-Driven**: YAML-based configuration system for flexible deployment

### Key Architecture Components

#### Five-Stage Strategy Pipeline (`src/strategy.rs`)
- **Universe Selection**: Dynamic symbol selection based on market conditions
- **Alpha Creation**: Signal generation with multiple model support (SimpleAlpha, MovingAverageCross, Momentum)
- **Portfolio Construction**: Weight allocation strategies (EqualWeighting, InsightWeighting)
- **Risk Management**: Pre-execution risk controls with position and exposure limits
- **Execution**: Order generation and placement with configurable algorithms

#### Data Management System (`src/data.rs`)
- **Unified Data Interface**: Common trait for all data sources
- **Multiple Data Sources**: CSV, Binance, and extensible connector system
- **Intelligent Caching**: Request-level caching with hit/miss statistics
- **Real-time Streaming**: WebSocket-based market data subscriptions

#### Configuration System (`src/config.rs`)
- **Hierarchical Configuration**: Framework, strategy, and component-level settings
- **YAML-based**: Human-readable configuration files
- **Type-safe Deserialization**: Compile-time validation using Serde
- **Environment Variable Support**: Flexible deployment configurations

#### Strategy Runner (`src/runner.rs`)
- **Configuration-Driven Setup**: Automatic strategy instantiation from config
- **Multi-Strategy Support**: Parallel execution of multiple strategies
- **Data Source Management**: Automated data source registration and setup
- **Performance Monitoring**: Built-in execution statistics and timing

### Core Type System (`src/types.rs`)

The framework uses a comprehensive type system for financial data:

- **Market Data**: `Bar` (OHLCV) and `Tick` (quote) data structures
- **Trading Types**: `Order`, `Trade`, `Position` with full lifecycle tracking
- **Strategy Types**: `Insight`, `PortfolioTarget` for signal processing
- **Enums**: `Direction`, `OrderType`, `OrderStatus`, `AssetType`, `InsightDirection`

### Error Handling (`src/error.rs`)

- **Unified Error Type**: `CzscError` with categorized error variants
- **Result Type Alias**: `type Result<T> = std::result::Result<T, CzscError>`
- **Structured Logging**: `tracing` crate for production-ready logging

## Module Structure

```
src/
├── lib.rs                  # Main library entry point
├── types.rs                # Core data types and structures
├── error.rs                # Unified error handling
├── strategy.rs             # Five-stage strategy pipeline (1700+ lines)
├── data.rs                 # Data management system (280+ lines)
├── runner.rs               # Configuration-driven strategy runner (440+ lines)
├── config.rs               # Configuration management
├── events.rs               # Event handling system
├── data/                   # Data source implementations
│   ├── csv_source.rs       # CSV file data source
│   └── binance.rs          # Binance exchange connector
├── examples/               # Example implementations
│   ├── mod.rs
│   └── data_integration.rs
└── bin/
    └── moses_quant.rs      # CLI application entry point
```

## Strategy Development Patterns

### Creating Custom Alpha Models

Implement the `AlphaModel` trait for custom signal generation:

```rust
#[async_trait]
pub trait AlphaModel: Send + Sync {
    async fn generate_insights(&self, context: &StrategyContext, symbols: &[Symbol]) -> Result<Vec<Insight>>;
    fn name(&self) -> &str;
}
```

### Portfolio Construction

Implement `PortfolioConstructor` for custom weight allocation:

```rust
#[async_trait]
pub trait PortfolioConstructor: Send + Sync {
    async fn create_targets(&self, context: &StrategyContext, insights: &[Insight]) -> Result<Vec<PortfolioTarget>>;
    fn name(&self) -> &str;
}
```

### Risk Management

Implement `RiskManager` for custom risk controls:

```rust
#[async_trait]
pub trait RiskManager: Send + Sync {
    async fn validate_targets(&self, context: &StrategyContext, targets: &[PortfolioTarget]) -> Result<Vec<PortfolioTarget>>;
    fn name(&self) -> &str;
}
```

## Configuration System

### Framework Configuration Structure

```yaml
framework:
  name: "MosesQuant"
  version: "0.1.0"
  initial_capital: 100000.0
  timezone: "UTC"

data_sources:
  - name: "binance_spot"
    source_type: "Binance"
    enabled: true
    symbols: ["BTCUSDT", "ETHUSDT"]
    connection:
      params:
        testnet: true

strategies:
  - id: "simple_trend"
    name: "Simple Trend Following"
    enabled: true
    universe_selector:
      component_type: "SimpleUniverseSelector"
      parameters:
        symbols: ["BTCUSDT", "ETHUSDT"]
    alpha_model:
      component_type: "SimpleAlphaModel"
    portfolio_constructor:
      component_type: "SimplePortfolioConstructor"

risk_management:
  max_position_size: 10.0
  max_total_exposure: 95.0
  daily_loss_limit: 2.0

execution:
  order_type: "Market"
  min_order_size: 0.001
  max_order_size: 1000.0
```

## Testing Strategy

### Test Organization
- **Unit Tests**: Each module has comprehensive unit tests
- **Integration Tests**: Cross-module functionality testing
- **Mock Data**: `MemoryDataSource` for testing data flows
- **Async Testing**: `#[tokio::test]` for async components

### Running Tests
```bash
# Run all tests
cargo test

# Run tests for specific module
cargo test strategy

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_strategy_framework
```

## Performance Considerations

### Async Design
- **Tokio Runtime**: All I/O operations are async
- **Concurrent Execution**: Multiple strategies run in parallel
- **Non-blocking**: Data fetching and processing don't block execution

### Memory Management
- **Arc<RwLock<T>>**: Shared state management for concurrent access
- **Efficient Caching**: LRU-style cache management in data layer
- **Zero-copy Operations**: Minimal data copying in hot paths

### Error Handling
- **Graceful Degradation**: Strategies continue running if individual components fail
- **Comprehensive Logging**: Structured logging for debugging and monitoring
- **Recovery Mechanisms**: Automatic retry and fallback strategies

## Common Issues and Solutions

### Compilation Issues
- **Missing Dependencies**: Ensure all Cargo.toml dependencies are available
- **Async Trait Issues**: Use `#[async_trait]` for trait methods that return futures
- **Type Mismatches**: Pay attention to `Result<T>` vs `std::result::Result<T, E>`

### Runtime Issues
- **Data Source Failures**: Check network connectivity and API credentials
- **Configuration Errors**: Validate YAML syntax and required fields
- **Strategy Execution**: Monitor logs for insight generation and risk management

### Development Tips
- **Configuration First**: Start with valid configuration files
- **Test Data Sources**: Use `MemoryDataSource` for isolated testing
- **Monitor Performance**: Use built-in statistics and timing information
- **Error Propagation**: Use `?` operator for clean error handling

## Integration Points

### Data Sources
- **CSV Files**: For historical backtesting with local data
- **Binance API**: For real-time cryptocurrency data
- **Custom Connectors**: Implement `DataSource` trait for new exchanges

### Strategy Components
- **Alpha Models**: Signal generation algorithms
- **Portfolio Constructors**: Position sizing and allocation
- **Risk Managers**: Pre-execution risk controls
- **Execution Algorithms**: Order placement strategies

### Configuration Management
- **YAML Files**: Primary configuration format
- **Environment Variables**: Runtime configuration overrides
- **Validation**: Compile-time and runtime configuration validation

This MosesQuant framework provides a robust foundation for quantitative trading system development with emphasis on performance, modularity, and type safety.