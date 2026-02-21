# Pattern Macros in pleme-codegen

## Overview

These macros were created through our feedback loop process after detecting repeated patterns across services. They eliminate approximately 2,940 lines of boilerplate code across the Nexus platform.

## New Pattern Macros

### 1. StatusStateMachine

**Saves: ~110 lines per enum**

Generates complete state machine logic for status enums including transitions, validation, and string conversions.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, StatusStateMachine)]
enum OrderStatus {
    Pending,
    AwaitingPayment,
    PaymentProcessing,
    Paid,
    Processing,
    Fulfilled,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
    // ... any other states
}

// Generated methods:
impl OrderStatus {
    fn can_transition_to(&self, new_status: &OrderStatus) -> bool;
    fn is_final_status(&self) -> bool;
    fn can_be_cancelled(&self) -> bool;
    fn can_be_refunded(&self) -> bool;
    fn to_str(&self) -> &'static str;
}

// Also implements FromStr for parsing
```

### 2. BrazilianTaxEntity

**Saves: ~30 lines per entity**

Generates Brazilian tax calculations including ICMS, PIS, COFINS, and ISS.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, BrazilianTaxEntity)]
struct Order {
    pub total: Decimal,
}

// Generated methods:
impl Order {
    fn calculate_icms(&self, subtotal: Decimal, state: &str) -> Decimal;
    fn calculate_pis(&self, subtotal: Decimal) -> Decimal;
    fn calculate_cofins(&self, subtotal: Decimal) -> Decimal;
    fn calculate_iss(&self, subtotal: Decimal, city: &str) -> Decimal;
    fn calculate_total_tax(&self, subtotal: Decimal, state: &str, is_service: bool) -> Decimal;
    fn generate_nfe_key(&self) -> String;
}
```

### 3. ShippingEntity

**Saves: ~25 lines per entity**

Generates shipping calculations with Brazilian regional zones.

```rust
#[derive(Debug, Clone, ShippingEntity)]
struct Order {
    pub items_count: i32,
    pub weight_kg: f64,
}

// Generated methods:
impl Order {
    fn calculate_shipping_cost(&self, items_count: i32, weight_kg: f64, 
                              origin_state: &str, dest_state: &str, 
                              country: &str) -> Decimal;
    fn estimate_delivery_days(&self, origin_state: &str, dest_state: &str, 
                             service_type: &str) -> u32;
    fn recommend_carrier(&self, origin: &str, dest: &str, weight_kg: f64) -> &'static str;
}
```

### 4. ValidatedEntity

**Saves: ~40 lines per struct**

Generates comprehensive validation chains for entities.

```rust
#[derive(Debug, Clone, ValidatedEntity)]
struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub postal_code: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub cpf: Option<String>,
}

// Generated methods:
impl Address {
    fn validate(&self) -> Result<(), Vec<String>>;
    fn validate_field(&self, field_name: &str) -> Result<(), String>;
    fn validation_context(&self) -> ValidationContext;
}
```

Features:
- Automatic field validation based on naming patterns
- Brazilian-specific validations (CPF, CEP, state codes)
- Email and phone validation
- Custom validation rules

### 5. IdentifierEntity

**Saves: ~10 lines per entity**

Generates unique identifier creation and parsing methods.

```rust
#[derive(Debug, Clone, IdentifierEntity)]
struct Product {
    pub name: String,
}

// Generated methods:
impl Product {
    fn generate_identifier(prefix: &str) -> String;
    fn generate_order_number() -> String;
    fn generate_invoice_number() -> String;
    fn generate_tracking_code() -> String;
    fn generate_customer_code() -> String;
    fn generate_sku(category: &str) -> String;
    fn generate_transaction_id() -> String;
    fn parse_identifier(identifier: &str) -> Option<IdentifierComponents>;
    fn is_valid_identifier(identifier: &str, expected_prefix: &str) -> bool;
    fn generate_short_code(length: usize) -> String;
    fn generate_barcode(country_code: &str, manufacturer_code: &str) -> String;
}
```

## Usage in Orders Service

These macros were created specifically to eliminate boilerplate detected in the Orders Service:

```rust
use pleme_codegen::{
    DomainModel, GraphQLBridge, StatusStateMachine, 
    BrazilianTaxEntity, ShippingEntity, ValidatedEntity, IdentifierEntity
};

// Order with all applicable macros
#[derive(Debug, Clone, Serialize, Deserialize, 
         DomainModel, GraphQLBridge, 
         BrazilianTaxEntity, ShippingEntity, IdentifierEntity)]
#[domain(table = "orders", cache_ttl = 900)]
pub struct Order {
    pub customer_id: Uuid,
    pub status: OrderStatus,
    pub total: Decimal,
    pub items: Vec<OrderItem>,
    pub shipping_address: Address,
    pub items_count: i32,
    pub weight_kg: f64,
}

// Order status with state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, StatusStateMachine)]
pub enum OrderStatus {
    Pending,
    AwaitingPayment,
    PaymentProcessing,
    Paid,
    Processing,
    Fulfilled,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

// Address with validation
#[derive(Debug, Clone, Serialize, Deserialize, ValidatedEntity, GraphQLBridge)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub postal_code: String,
}
```

## Impact Analysis

| Pattern | Occurrences | Lines Saved | Total Savings |
|---------|-------------|-------------|---------------|
| StatusStateMachine | 15 | 110 | 1,650 lines |
| ValidationChain | 20+ | 40 | 800+ lines |
| BrazilianTaxCalculation | 8 | 30 | 240 lines |
| OrderNumberGeneration | 10 | 10 | 100 lines |
| ShippingCalculation | 6 | 25 | 150 lines |
| **Total** | **59+** | **~50 avg** | **~2,940 lines** |

## Benefits

1. **Consistency**: All services use identical implementations
2. **Maintainability**: Changes to patterns update all services
3. **Type Safety**: Compile-time verification of patterns
4. **Performance**: No runtime overhead, all code generated at compile time
5. **Brazilian Market Support**: Built-in support for CPF, CEP, ICMS, etc.

## Future Enhancements

Additional patterns identified for future implementation:
- **PaymentEntity**: PIX, Boleto, Credit Card processing patterns
- **NotificationEntity**: Email, SMS, Push notification patterns
- **AuditableEntity**: Automatic change tracking and history
- **SearchableEntity**: Full-text search and filtering patterns
- **MetricsEntity**: Performance monitoring and analytics