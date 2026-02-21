//! Utility functions for macro generation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Lit, Meta};
use heck::{ToSnakeCase, ToPascalCase, ToKebabCase};

/// Extract string value from attribute
pub fn get_attribute_value(attrs: &[Attribute], name: &str, key: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(name) {
            let mut result = None;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(key) {
                    if let Ok(lit_str) = meta.value()?.parse::<syn::LitStr>() {
                        result = Some(lit_str.value());
                    }
                }
                Ok(())
            });
            if result.is_some() {
                return result;
            }
        }
    }
    None
}

/// Extract integer value from attribute
pub fn get_attribute_int(attrs: &[Attribute], name: &str, key: &str) -> Option<u64> {
    for attr in attrs {
        if attr.path().is_ident(name) {
            let mut result = None;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(key) {
                    if let Ok(lit_int) = meta.value()?.parse::<syn::LitInt>() {
                        result = lit_int.base10_parse().ok();
                    }
                }
                Ok(())
            });
            if result.is_some() {
                return result;
            }
        }
    }
    None
}

/// Check if attribute flag is present
pub fn has_attribute_flag(attrs: &[Attribute], name: &str, flag: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident(name) {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(flag) {
                    found = true;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// Generate standard domain model fields
pub fn generate_standard_fields() -> TokenStream {
    quote! {
        /// Unique identifier for this entity
        pub id: uuid::Uuid,
        
        /// Product/tenant identifier for multi-tenancy
        pub product: String,
        
        /// When this entity was created
        pub created_at: chrono::DateTime<chrono::Utc>,
        
        /// When this entity was last updated
        pub updated_at: chrono::DateTime<chrono::Utc>,
    }
}

/// Generate standard derives for domain models
pub fn generate_standard_derives() -> TokenStream {
    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    }
}

/// Generate cache key function
pub fn generate_cache_key_fn(struct_name: &str) -> TokenStream {
    let cache_prefix = struct_name.to_snake_case();
    
    quote! {
        impl #struct_name {
            /// Generate cache key for this entity
            pub fn cache_key(&self) -> String {
                format!("{}:{}:{}", self.product, #cache_prefix, self.id)
            }
            
            /// Generate cache key for entity by ID
            pub fn cache_key_for(product: &str, id: uuid::Uuid) -> String {
                format!("{}:{}:{}", product, #cache_prefix, id)
            }
            
            /// Generate cache key pattern for product
            pub fn cache_pattern(product: &str) -> String {
                format!("{}:{}:*", product, #cache_prefix)
            }
        }
    }
}

/// Generate new constructor with standard fields
pub fn generate_new_constructor(struct_name: &str, fields: &[String]) -> TokenStream {
    let struct_ident = syn::Ident::new(struct_name, proc_macro2::Span::call_site());
    
    let field_params: Vec<TokenStream> = fields.iter().map(|field| {
        let field_ident = syn::Ident::new(field, proc_macro2::Span::call_site());
        quote! { #field_ident: String }
    }).collect();
    
    let field_assigns: Vec<TokenStream> = fields.iter().map(|field| {
        let field_ident = syn::Ident::new(field, proc_macro2::Span::call_site());
        quote! { #field_ident }
    }).collect();
    
    quote! {
        impl #struct_ident {
            /// Create a new instance with auto-generated standard fields
            pub fn new(
                product: String,
                #(#field_params,)*
            ) -> Self {
                let now = chrono::Utc::now();
                Self {
                    id: uuid::Uuid::new_v4(),
                    product,
                    created_at: now,
                    updated_at: now,
                    #(#field_assigns,)*
                }
            }
            
            /// Update the updated_at timestamp
            pub fn touch(&mut self) {
                self.updated_at = chrono::Utc::now();
            }
        }
    }
}

/// Generate validation trait implementation
pub fn generate_validation() -> TokenStream {
    quote! {
        /// Validation trait for domain models
        pub trait Validate {
            type Error;
            
            /// Validate this entity
            fn validate(&self) -> Result<(), Self::Error>;
        }
    }
}

/// Generate table name constant
pub fn generate_table_constant(struct_name: &str, table_name: Option<String>) -> TokenStream {
    let struct_ident = syn::Ident::new(struct_name, proc_macro2::Span::call_site());
    let table = table_name.unwrap_or_else(|| struct_name.to_snake_case() + "s");
    
    quote! {
        impl #struct_ident {
            /// Database table name for this entity
            pub const TABLE_NAME: &'static str = #table;
        }
    }
}

/// Convert Rust type to GraphQL-compatible type
pub fn rust_to_graphql_type(type_str: &str) -> String {
    match type_str {
        "rust_decimal::Decimal" | "Decimal" => "f64".to_string(),
        "serde_json::Value" | "Value" => "JSON".to_string(),
        "chrono::DateTime<chrono::Utc>" | "DateTime<Utc>" => "DateTime".to_string(),
        "uuid::Uuid" | "Uuid" => "UUID".to_string(),
        other => other.to_string(),
    }
}

/// Generate GraphQL field conversion
pub fn generate_graphql_conversion(field_name: &str, rust_type: &str, is_option: bool) -> (TokenStream, TokenStream) {
    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
    
    let (to_graphql, from_graphql) = match rust_type {
        "rust_decimal::Decimal" | "Decimal" => {
            if is_option {
                (
                    quote! { { use num_traits::ToPrimitive; self.#field_ident.map(|d| d.to_f64().unwrap_or(0.0)) } },
                    quote! { input.#field_ident.map(rust_decimal::Decimal::from_f64_retain).flatten() }
                )
            } else {
                (
                    quote! { { use num_traits::ToPrimitive; self.#field_ident.to_f64().unwrap_or(0.0) } },
                    quote! { rust_decimal::Decimal::from_f64_retain(input.#field_ident).unwrap_or_default() }
                )
            }
        }
        "serde_json::Value" | "Value" => {
            (
                quote! { self.#field_ident.clone() },
                quote! { input.#field_ident.clone() }
            )
        }
        _ => {
            (
                quote! { self.#field_ident.clone() },
                quote! { input.#field_ident.clone() }
            )
        }
    };
    
    (to_graphql, from_graphql)
}

/// Generate Brazilian validation functions
pub fn generate_cpf_validation() -> TokenStream {
    quote! {
        /// Validate Brazilian CPF document
        pub fn validate_cpf(cpf: &str) -> bool {
            let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
            
            if digits.len() != 11 {
                return false;
            }
            
            // Check for known invalid patterns
            if digits.chars().all(|c| c == digits.chars().next().unwrap()) {
                return false;
            }
            
            // Calculate verification digits
            let digits: Vec<u32> = digits.chars().map(|c| c.to_digit(10).unwrap()).collect();
            
            let sum1: u32 = digits[0..9].iter().enumerate()
                .map(|(i, &d)| d * (10 - i as u32))
                .sum();
            let check1 = match sum1 % 11 {
                0 | 1 => 0,
                n => 11 - n,
            };
            
            if check1 != digits[9] {
                return false;
            }
            
            let sum2: u32 = digits[0..10].iter().enumerate()
                .map(|(i, &d)| d * (11 - i as u32))
                .sum();
            let check2 = match sum2 % 11 {
                0 | 1 => 0,
                n => 11 - n,
            };
            
            check2 == digits[10]
        }
        
        /// Format CPF for display (XXX.XXX.XXX-XX)
        pub fn format_cpf(cpf: &str) -> String {
            let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 11 {
                format!("{}.{}.{}-{}", 
                    &digits[0..3], &digits[3..6], 
                    &digits[6..9], &digits[9..11])
            } else {
                cpf.to_string()
            }
        }
    }
}

/// Generate CEP validation functions  
pub fn generate_cep_validation() -> TokenStream {
    quote! {
        /// Validate Brazilian CEP (postal code)
        pub fn validate_cep(cep: &str) -> bool {
            let digits: String = cep.chars().filter(|c| c.is_ascii_digit()).collect();
            digits.len() == 8
        }
        
        /// Format CEP for display (XXXXX-XXX)
        pub fn format_cep(cep: &str) -> String {
            let digits: String = cep.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 8 {
                format!("{}-{}", &digits[0..5], &digits[5..8])
            } else {
                cep.to_string()
            }
        }
    }
}