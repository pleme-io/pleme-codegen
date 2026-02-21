//! Integration tests for pleme-codegen macros
//!
//! Tests that the generated code compiles and behaves correctly.

use pleme_codegen::*;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Test data structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestPayment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product: String,
    pub amount: Decimal,
    pub currency: String,
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
    Cancelled,
}

impl TestPayment {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            product: "test".to_string(),
            amount: Decimal::new(10000, 2), // $100.00
            currency: "BRL".to_string(),
            status: PaymentStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }
}

// Mock error type for testing
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

// Mock Redis pool type for testing
pub struct MockRedisPool;

impl MockRedisPool {
    pub async fn get(&self) -> Result<MockRedisConnection, TestError> {
        Ok(MockRedisConnection)
    }
}

pub struct MockRedisConnection;

// Mock database pool type for testing  
pub struct MockPgPool;

impl MockPgPool {
    pub async fn begin(&self) -> Result<MockTransaction, TestError> {
        Ok(MockTransaction)
    }
}

pub struct MockTransaction;

// =============================================================================
// CachedRepository Tests
// =============================================================================

#[derive(CachedRepository)]
#[cached(
    entity = "payment",
    key_pattern = "payment:{product}:{id}",
    ttl = 300,
    pool_field = "redis"
)]
pub struct TestCachedRepository {
    pub pool: MockPgPool,
    pub redis: Option<MockRedisPool>,
}

#[test]
fn test_cached_repository_compilation() {
    let repo = TestCachedRepository {
        pool: MockPgPool,
        redis: Some(MockRedisPool),
    };
    
    // Test that the struct compiles and methods are generated
    // Note: We can't actually test Redis operations in unit tests,
    // but we can verify the methods exist and have correct signatures
    
    // These would normally be async tests with a real Redis connection
    println!("CachedRepository macro generated methods successfully");
}

// =============================================================================
// DatabaseMapper Tests  
// =============================================================================

#[derive(DatabaseMapper)]
#[database(table = "test_payments", primary_key = "id")]
pub struct TestMappedEntity {
    pub id: Uuid,
    #[db(column = "user_id")]
    pub user_id: Uuid,
    pub amount: Decimal,
    #[db(enum)]
    pub status: PaymentStatus,
    pub created_at: DateTime<Utc>,
}

#[test]
fn test_database_mapper_sql_generation() {
    // Test that SQL statements are generated correctly
    assert!(TestMappedEntity::insert_sql().contains("INSERT INTO test_payments"));
    assert!(TestMappedEntity::find_by_id_sql().contains("SELECT"));
    assert!(TestMappedEntity::update_sql().contains("UPDATE test_payments"));
    assert!(TestMappedEntity::delete_sql().contains("DELETE FROM test_payments"));
    
    // Test metadata
    assert_eq!(TestMappedEntity::table_name(), "test_payments");
    assert_eq!(TestMappedEntity::primary_key(), "id");
    assert!(!TestMappedEntity::columns().is_empty());
}

#[test]
fn test_database_mapper_query_builder() {
    let builder = TestMappedEntity::query_builder()
        .where_clause("amount > 100")
        .order_by("created_at", "DESC")
        .limit(10);
    
    let sql = builder.build_select();
    assert!(sql.contains("SELECT * FROM test_payments"));
    assert!(sql.contains("WHERE amount > 100"));
    assert!(sql.contains("ORDER BY created_at DESC"));
    assert!(sql.contains("LIMIT 10"));
}

#[test]
fn test_entity_metadata() {
    let metadata = TestMappedEntity::entity_metadata();
    assert_eq!(metadata.name, "TestMappedEntity");
    assert_eq!(metadata.table, "test_payments");
    assert_eq!(metadata.primary_key, "id");
    assert!(!metadata.columns.is_empty());
    
    // Test Display implementation
    let display_str = format!("{}", metadata);
    assert!(display_str.contains("TestMappedEntity"));
    assert!(display_str.contains("test_payments"));
}

// =============================================================================
// TransactionalRepository Tests
// =============================================================================

#[derive(TransactionalRepository)]
#[transactional(
    pool_field = "pool",
    error_type = "TestError",
    lock_timeout = 30,
    isolation_level = "ReadCommitted"
)]
pub struct TestTransactionalRepository {
    pub pool: MockPgPool,
}

#[test]
fn test_transactional_repository_compilation() {
    let repo = TestTransactionalRepository {
        pool: MockPgPool,
    };
    
    // Test that the struct compiles and methods are generated
    // Note: We can't actually test database transactions in unit tests,
    // but we can verify the methods exist and have correct signatures
    
    println!("TransactionalRepository macro generated methods successfully");
}

// =============================================================================
// BrazilianPaymentEntity Tests
// =============================================================================

#[derive(BrazilianPaymentEntity)]
#[brazilian_payment(
    tax_type = "icms",
    currency = "BRL",
    icms_rate = 0.18,
    pis_rate = 0.0165,
    cofins_rate = 0.076
)]
pub struct TestBrazilianPayment {
    pub id: Uuid,
    pub amount: Decimal,
    pub status: PaymentStatus,
    pub user_id: Uuid,
    pub product: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TestBrazilianPayment {
    // Implement required methods for BrazilianPaymentEntity
    fn get_id(&self) -> Uuid {
        self.id
    }
    
    fn get_amount(&self) -> Option<Decimal> {
        Some(self.amount)
    }
    
    fn get_status(&self) -> PaymentStatus {
        self.status
    }
    
    fn set_status(&mut self, status: PaymentStatus) {
        self.status = status;
    }
    
    fn set_updated_at(&mut self, timestamp: DateTime<Utc>) {
        self.updated_at = timestamp;
    }
    
    fn get_customer_document(&self) -> Option<String> {
        Some("123.456.789-00".to_string())
    }
}

#[test]
fn test_brazilian_amount_formatting() {
    let amount = Decimal::new(123456, 2); // R$ 1,234.56
    let formatted = TestBrazilianPayment::format_brl_amount(amount);
    assert_eq!(formatted, "R$ 1.234,56");
    
    let parsed = TestBrazilianPayment::parse_brl_amount("R$ 1.234,56").unwrap();
    assert_eq!(parsed, amount);
}

#[test]  
fn test_brazilian_amount_parsing() {
    let test_cases = vec![
        ("R$ 100,00", Decimal::new(10000, 2)),
        ("R$ 1.234,56", Decimal::new(123456, 2)),
        ("100,50", Decimal::new(10050, 2)),
    ];
    
    for (input, expected) in test_cases {
        let parsed = TestBrazilianPayment::parse_brl_amount(input).unwrap();
        assert_eq!(parsed, expected, "Failed to parse {}", input);
    }
}

#[test]
fn test_pix_key_validation() {
    use crate::PixKeyType;
    
    // Test CPF validation
    assert!(TestBrazilianPayment::validate_pix_key("123.456.789-00", PixKeyType::Cpf).is_ok());
    assert!(TestBrazilianPayment::validate_pix_key("invalid-cpf", PixKeyType::Cpf).is_err());
    
    // Test email validation
    assert!(TestBrazilianPayment::validate_pix_key("user@example.com", PixKeyType::Email).is_ok());
    assert!(TestBrazilianPayment::validate_pix_key("invalid-email", PixKeyType::Email).is_err());
    
    // Test phone validation
    assert!(TestBrazilianPayment::validate_pix_key("11987654321", PixKeyType::Phone).is_ok());
    assert!(TestBrazilianPayment::validate_pix_key("123", PixKeyType::Phone).is_err());
}

#[test]
fn test_brazilian_tax_calculation() {
    let payment = TestBrazilianPayment {
        id: Uuid::new_v4(),
        amount: Decimal::new(100000, 2), // R$ 1,000.00
        status: PaymentStatus::Pending,
        user_id: Uuid::new_v4(),
        product: "test".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let tax_breakdown = payment.calculate_brazilian_taxes().unwrap();
    
    // Verify tax calculations
    assert_eq!(tax_breakdown.gross_amount, Decimal::new(100000, 2));
    assert_eq!(tax_breakdown.currency, "BRL");
    
    // ICMS should be 18% of gross amount
    let expected_icms = Decimal::new(100000, 2) * Decimal::new(18, 2); // 18%
    assert_eq!(tax_breakdown.icms_amount, expected_icms);
    
    // Total taxes should be sum of all tax components
    let expected_total = tax_breakdown.icms_amount + 
                        tax_breakdown.pis_amount + 
                        tax_breakdown.cofins_amount;
    assert_eq!(tax_breakdown.total_taxes, expected_total);
    
    // Net amount should be gross minus total taxes
    assert_eq!(tax_breakdown.net_amount, 
               tax_breakdown.gross_amount - tax_breakdown.total_taxes);
}

#[test]
fn test_brazilian_receipt_generation() {
    let payment = TestBrazilianPayment {
        id: Uuid::new_v4(),
        amount: Decimal::new(50000, 2), // R$ 500.00
        status: PaymentStatus::Completed,
        user_id: Uuid::new_v4(),
        product: "test".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let receipt = payment.generate_brazilian_receipt().unwrap();
    
    assert_eq!(receipt.amount, Decimal::new(50000, 2));
    assert_eq!(receipt.formatted_amount, "R$ 500,00");
    assert_eq!(receipt.status, "Concluído");
    assert_eq!(receipt.merchant_name, "Pleme Tecnologia Ltda");
    assert_eq!(receipt.customer_document, "123.456.789-00");
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_macro_composition() {
    // Test that multiple macros can be applied to the same struct
    #[derive(DatabaseMapper, BrazilianPaymentEntity)]
    #[database(table = "payments")]
    #[brazilian_payment(currency = "BRL")]
    pub struct ComposedEntity {
        pub id: Uuid,
        pub amount: Decimal,
        pub status: PaymentStatus,
        pub user_id: Uuid,
        pub product: String,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }
    
    impl ComposedEntity {
        fn get_id(&self) -> Uuid { self.id }
        fn get_amount(&self) -> Option<Decimal> { Some(self.amount) }
        fn get_status(&self) -> PaymentStatus { self.status }
        fn set_status(&mut self, status: PaymentStatus) { self.status = status; }
        fn set_updated_at(&mut self, timestamp: DateTime<Utc>) { self.updated_at = timestamp; }
        fn get_customer_document(&self) -> Option<String> { None }
    }
    
    // Test that both macros work together
    assert_eq!(ComposedEntity::table_name(), "payments");
    
    let amount = Decimal::new(100000, 2);
    let formatted = ComposedEntity::format_brl_amount(amount);
    assert!(formatted.starts_with("R$"));
}

#[test]
fn test_error_handling() {
    // Test that generated code handles errors appropriately
    let invalid_amount = TestBrazilianPayment::parse_brl_amount("invalid");
    assert!(invalid_amount.is_err());
    
    let invalid_pix = TestBrazilianPayment::validate_pix_key("", crate::PixKeyType::Cpf);
    assert!(invalid_pix.is_err());
}