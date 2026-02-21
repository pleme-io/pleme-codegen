# pleme-codegen

**Procedural macros for generating boilerplate code in Pleme services with Brazilian market features**

## Overview

pleme-codegen is our macro-driven development solution that eliminates 95%+ of boilerplate code in service development. Instead of writing repetitive patterns manually, we encode them in macros that generate correct, consistent, tested code automatically.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pleme-codegen = { path = "./.deps/pleme-codegen" }
```

## Available Macros

### 1. DomainModel

Automatically generates standard domain model patterns:

```rust
use pleme_codegen::DomainModel;

#[derive(Debug, Clone, Serialize, Deserialize, DomainModel)]
#[domain(table = "users", cache_ttl = 300)]
pub struct User {
    pub email: String,
    pub name: String,
    // id, product, created_at, updated_at auto-generated
}

// Generated methods:
// - User::TABLE_NAME constant
// - cache_key() method
// - Database query helpers
```

**Auto-generated features:**
- UUID primary key (`id`)
- Multi-tenancy field (`product`)
- Timestamps (`created_at`, `updated_at`) 
- Cache key generation (`cache_key()`)
- Database table constants (`TABLE_NAME`)

### 2. GraphQLBridge

Handles type conversions between Rust and GraphQL:

```rust
use pleme_codegen::GraphQLBridge;

#[derive(GraphQLBridge)]
pub struct ProductPrice {
    pub base_price: Decimal,    // Auto-converts to f64 for GraphQL
    pub metadata: serde_json::Value,
}

// Generated methods:
// - to_graphql() for JSON serialization
// - Automatic Decimal ↔ f64 conversion
```

**Auto-generated features:**
- Decimal ↔ f64 conversions for GraphQL compatibility
- JSON Value handling
- DateTime formatting
- Type-safe GraphQL integration

### 3. BrazilianEntity

Brazilian market-specific validations and formatting:

```rust
use pleme_codegen::BrazilianEntity;

#[derive(BrazilianEntity)]
pub struct Customer {
    pub name: String,
}

// Generated methods:
// - Customer::validate_cpf(cpf) 
// - Customer::format_cpf(cpf)
```

**Auto-generated features:**
- CPF validation and formatting (XXX.XXX.XXX-XX)
- CEP validation and formatting (XXXXX-XXX) 
- CNPJ validation for businesses
- Brazilian phone number handling

## Usage Examples

### Complete Service Model

```rust
use pleme_codegen::{DomainModel, GraphQLBridge, BrazilianEntity};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

// Complete domain model with all features
#[derive(Debug, Clone, Serialize, Deserialize, DomainModel, GraphQLBridge, BrazilianEntity)]
#[domain(table = "orders", cache_ttl = 600)]
pub struct Order {
    pub customer_id: Uuid,
    pub total: Decimal,          // Auto-converts to f64 for GraphQL
    pub currency: String,
    // id, product, created_at, updated_at auto-generated
}

// Usage:
fn example() {
    // Table name constant
    println!("Table: {}", Order::TABLE_NAME); // "orders"
    
    // Cache key generation
    let order = Order { /* ... */ };
    let key = order.cache_key(); // "product:order:uuid"
    
    // GraphQL conversion
    let graphql_repr = order.to_graphql(); // JSON string
    
    // Brazilian validation (static methods)
    let is_valid = Order::validate_cpf("12345678901"); // true
    let formatted = Order::format_cpf("12345678901"); // "123.456.789-01"
}
```

### Market-Specific Models

```rust
// Brazilian customer with document validation
#[derive(BrazilianEntity)]
pub struct BrazilianCustomer {
    pub email: String,
    // Auto-generated CPF/CNPJ validation methods
}

// Usage:
let cpf_valid = BrazilianCustomer::validate_cpf("12345678901");
let formatted_cpf = BrazilianCustomer::format_cpf("12345678901");
```

## Development Workflow

1. **Design domain models** with macro derives (5 minutes)
2. **Let macros generate infrastructure** (automatic)
3. **Focus on business logic** (30-60 minutes)
4. **Enhance macros when patterns emerge** (as needed)

## Pattern Recognition

When developing services, always look for opportunities to enhance pleme-codegen:

### If you write the same code twice → Create a macro
### If you see boilerplate → Propose a macro enhancement  
### If you fix a bug in multiple places → Centralize in macro

## Macro Enhancement Process

1. **Document the pattern** in current service
2. **Create enhancement issue** for pleme-codegen
3. **Implement macro enhancement**
4. **Test with current service**
5. **Apply to other services**
6. **Remove manual implementations**

## 🚀 Advanced AI-Driven Macros (NEW 2024)

### SmartRepository - AI-Enhanced Repository Pattern
```rust
#[derive(SmartRepository)]
pub struct OrderRepository {
    // Auto-generates: CRUD with observability, smart caching, bulk operations
}

// Generated methods:
// - create_with_observability() - Audit trails + performance tracking
// - find_with_smart_cache() - Multi-layer caching with automatic invalidation
// - bulk_create_optimized() - Batch processing with performance optimization
// - build_optimized_query() - AI-enhanced query optimization
```

### SmartService - Resilient Service Layer
```rust
#[derive(SmartService)]
pub struct OrderService {
    // Auto-generates: Circuit breakers, retry logic, distributed tracing
}

// Generated methods:
// - execute_with_resilience() - Automatic retry with exponential backoff
// - execute_with_tracing() - Distributed tracing integration
// - health_check_comprehensive() - Deep dependency health verification
```

### SmartMigration - Intelligent Database Schema Management
```rust
#[derive(SmartMigration)]
pub struct Order {
    // Auto-generates: Migration SQL, schema validation, performance indexes
}

// Generated methods:
// - generate_migration_sql() - Complete schema with optimizations
// - validate_schema_compatibility() - Runtime schema verification
// - suggest_performance_indexes() - AI-suggested indexes based on usage patterns
```

### ArchitecturalMonitor - Continuous Architectural Observability
```rust
#[derive(ArchitecturalMonitor)]
pub struct MyEntity {
    // Auto-generates: Pattern detection, performance monitoring, debt analysis
}

// Generated methods:
// - monitor_operation() - Track performance and patterns
// - analyze_architectural_patterns() - Detect and classify patterns
// - generate_health_report() - Comprehensive architectural health assessment
// - calculate_health_score() - Quantified architectural quality (0.0-1.0)
```

## 🎯 AI-Driven Architectural Intelligence

### Continuous Pattern Analysis
Our enhanced macro system continuously monitors and analyzes architectural patterns:

- **Pattern Usage Tracking**: Automatically detects frequently used patterns
- **Performance Monitoring**: Tracks generated code performance in real-time
- **Technical Debt Detection**: AI identifies accumulating architectural debt
- **Enhancement Suggestions**: Automatically suggests macro improvements

### Architectural Observability Dashboard
```rust
// Get real-time architectural health report
let report = pleme_codegen::get_architectural_report();
println!("{}", serde_json::to_string_pretty(&report)?);

// Output:
// {
//   "total_patterns_tracked": 15,
//   "high_usage_patterns": [("DomainModel", 45), ("GraphQLBridge", 32)],
//   "performance_issues": [("database_query", 850.5)],
//   "debt_indicators_count": 3,
//   "critical_debt_count": 0,
//   "high_debt_count": 1,
//   "suggestions": [
//     "Consider creating specialized macro for 'OrderProcessing' pattern (used 25 times)",
//     "Optimize 'database_query' operation - average 850ms (consider caching)"
//   ]
// }
```

### Automatic Refactoring Suggestions
The system automatically detects when patterns should be enhanced:

- **Usage Threshold Detection**: Suggests new macros when patterns exceed usage thresholds
- **Performance Regression**: Identifies when generated code performance degrades
- **Complexity Analysis**: Monitors architectural complexity and suggests simplifications
- **Market-Specific Enhancements**: Detects Brazilian market patterns and suggests optimizations

## Future AI Enhancements (Roadmap)

- **AutoMacro**: AI generates new macros from code patterns automatically
- **Performance Predictor**: AI predicts performance issues before deployment
- **Architecture Advisor**: AI suggests architectural improvements based on industry patterns
- **Code Migration Assistant**: AI assists in migrating between architectural patterns
- **Brazilian Market Intelligence**: Advanced Brazilian business logic automation

## Benefits

| Aspect | Before (Manual) | After (Macros) | Improvement |
|--------|-----------------|---------------|-------------|
| Domain model setup | 50+ lines | 2 lines | 95%+ |
| GraphQL integration | 30+ lines | 1 derive | 90%+ |
| Brazilian features | 100+ lines | 1 derive | 95%+ |
| **Total per service** | **3-5 hours** | **5 minutes** | **98%** |

## Architecture Philosophy

pleme-codegen embodies our "Generate, Don't Document" philosophy:

- **Instead of documenting complex patterns** → Generate them automatically
- **Instead of training on consistency** → Enforce at compile time
- **Instead of reviewing for patterns** → Make patterns impossible to bypass
- **Instead of accumulating technical debt** → Eliminate it at the source

This is our **permanent solution** to architectural complexity debt.

## Contributing

When you identify a pattern worth automating:

1. Document it clearly with examples
2. Consider the abstraction level (not too specific, not too generic)
3. Write comprehensive tests for the generated code
4. Update this README with usage examples

## Testing

Run the tests to verify macro generation:

```bash
cd pleme-codegen
cargo test
cargo expand  # See generated code (requires cargo-expand)
```

## License

MIT License - Internal Pleme tool for service development acceleration.