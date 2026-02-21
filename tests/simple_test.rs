// Simple test to verify macros compile
use pleme_codegen::{DomainModel, GraphQLBridge, BrazilianEntity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, DomainModel, GraphQLBridge, BrazilianEntity)]
#[domain(table = "test_entities", cache_ttl = 300)]
struct TestEntity {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_model_macro() {
        let entity = TestEntity {
            name: "Test".to_string(),
        };
        
        // Test that methods are generated
        let _cache_key = entity.cache_key();
        assert_eq!(TestEntity::TABLE_NAME, "TestEntitys");
    }

    #[test]
    fn test_graphql_bridge_macro() {
        let entity = TestEntity {
            name: "Test".to_string(),
        };
        
        // Test that to_graphql method is generated
        let _json = entity.to_graphql();
        
        // Test validation method
        let _result = entity.validate_for_graphql();
    }

    #[test]
    fn test_brazilian_entity_macro() {
        // Test CPF validation  
        assert!(TestEntity::validate_cpf("111.444.777-35")); // Valid CPF
        assert!(!TestEntity::validate_cpf("123.456.789-09")); // Invalid checksum
        assert!(!TestEntity::validate_cpf("invalid"));
        
        // Test CEP validation
        assert!(TestEntity::validate_cep("12345-678"));
        assert!(!TestEntity::validate_cep("invalid"));
        
        // Test formatting
        let formatted_cpf = TestEntity::format_cpf("12345678909");
        assert_eq!(formatted_cpf, "123.456.789-09");
    }
}