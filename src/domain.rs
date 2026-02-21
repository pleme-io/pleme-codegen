//! Domain Model macro implementation
//!
//! Generates standard entity patterns with automatic:
//! - UUID primary keys
//! - Multi-tenancy support
//! - Created/updated timestamps
//! - Serde serialization
//! - Cache key generation
//! - Database table mapping

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use crate::utils::*;

/// Implementation of the DomainModel derive macro
pub fn derive_domain_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    // Extract attributes
    let table_name = get_attribute_value(&input.attrs, "domain", "table");
    let cache_ttl = get_attribute_int(&input.attrs, "domain", "cache_ttl").unwrap_or(300);
    let tenant_field = get_attribute_value(&input.attrs, "domain", "tenant_field")
        .unwrap_or_else(|| "product".to_string());
    
    // Get existing fields
    let existing_fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("DomainModel can only be used with structs with named fields"),
        },
        _ => panic!("DomainModel can only be used with structs"),
    };
    
    // Check if standard fields already exist
    let has_id = existing_fields.iter().any(|f| f.ident.as_ref().unwrap() == "id");
    let has_product = existing_fields.iter().any(|f| f.ident.as_ref().unwrap() == &tenant_field);
    let has_created_at = existing_fields.iter().any(|f| f.ident.as_ref().unwrap() == "created_at");
    let has_updated_at = existing_fields.iter().any(|f| f.ident.as_ref().unwrap() == "updated_at");
    
    // Generate additional fields if they don't exist
    let additional_fields = generate_additional_fields(has_id, has_product, has_created_at, has_updated_at, &tenant_field);
    
    // Generate implementations
    let cache_impl = generate_cache_implementation(struct_name, cache_ttl);
    let table_impl = generate_table_implementation(struct_name, table_name);
    let constructor_impl = generate_constructor_implementation(struct_name, existing_fields, &tenant_field);
    let validation_impl = generate_validation_implementation(struct_name);
    let query_impl = generate_query_implementation(struct_name);
    
    let expanded = quote! {
        // Add the additional fields to the struct
        #additional_fields
        
        // Standard derives for domain models
        impl #struct_name {
            /// Cache TTL in seconds
            pub const CACHE_TTL: u64 = #cache_ttl;
        }
        
        #cache_impl
        #table_impl
        #constructor_impl
        #validation_impl
        #query_impl
        
        // Automatic serde derives
        impl serde::Serialize for #struct_name {}
        impl<'de> serde::Deserialize<'de> for #struct_name {}
    };
    
    TokenStream::from(expanded)
}

/// Generate additional standard fields if they don't exist
fn generate_additional_fields(
    has_id: bool, 
    has_product: bool, 
    has_created_at: bool, 
    has_updated_at: bool,
    tenant_field: &str
) -> TokenStream2 {
    let mut fields = Vec::new();
    
    if !has_id {
        fields.push(quote! {
            /// Unique identifier for this entity
            pub id: uuid::Uuid,
        });
    }
    
    if !has_product {
        let tenant_ident = syn::Ident::new(tenant_field, proc_macro2::Span::call_site());
        fields.push(quote! {
            /// Product/tenant identifier for multi-tenancy  
            pub #tenant_ident: String,
        });
    }
    
    if !has_created_at {
        fields.push(quote! {
            /// When this entity was created
            pub created_at: chrono::DateTime<chrono::Utc>,
        });
    }
    
    if !has_updated_at {
        fields.push(quote! {
            /// When this entity was last updated
            pub updated_at: chrono::DateTime<chrono::Utc>,
        });
    }
    
    if fields.is_empty() {
        quote! {}
    } else {
        quote! {
            // Additional standard fields
            #(#fields)*
        }
    }
}

/// Generate cache-related implementations
fn generate_cache_implementation(struct_name: &syn::Ident, cache_ttl: u64) -> TokenStream2 {
    let struct_name_str = struct_name.to_string().to_lowercase();
    
    quote! {
        impl #struct_name {
            /// Generate cache key for this entity instance
            pub fn cache_key(&self) -> String {
                format!("{}:{}:{}", self.product, #struct_name_str, self.id)
            }
            
            /// Generate cache key for entity by ID and product
            pub fn cache_key_for(product: &str, id: uuid::Uuid) -> String {
                format!("{}:{}:{}", product, #struct_name_str, id)
            }
            
            /// Generate cache key pattern for all entities in product
            pub fn cache_pattern(product: &str) -> String {
                format!("{}:{}:*", product, #struct_name_str)
            }
            
            /// Cache TTL for this entity type
            pub fn cache_ttl() -> u64 {
                #cache_ttl
            }
        }
    }
}

/// Generate table-related implementations
fn generate_table_implementation(struct_name: &syn::Ident, table_name: Option<String>) -> TokenStream2 {
    let table = table_name.unwrap_or_else(|| {
        let name = struct_name.to_string().to_lowercase();
        if name.ends_with('y') {
            format!("{}ies", &name[..name.len()-1])
        } else if name.ends_with('s') {
            name
        } else {
            format!("{}s", name)
        }
    });
    
    quote! {
        impl #struct_name {
            /// Database table name for this entity
            pub const TABLE_NAME: &'static str = #table;
            
            /// Get the table name
            pub fn table_name() -> &'static str {
                Self::TABLE_NAME
            }
        }
    }
}

/// Generate constructor implementation
fn generate_constructor_implementation(
    struct_name: &syn::Ident, 
    existing_fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    tenant_field: &str
) -> TokenStream2 {
    // Get field names and types for constructor parameters
    let field_params: Vec<TokenStream2> = existing_fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        
        // Skip standard fields that are auto-generated
        if field_name == "id" || field_name == tenant_field || 
           field_name == "created_at" || field_name == "updated_at" {
            quote! {}
        } else {
            quote! { #field_name: #field_type, }
        }
    }).filter(|tokens| !tokens.is_empty()).collect();
    
    let field_assigns: Vec<TokenStream2> = existing_fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        
        // Skip standard fields that are auto-generated
        if field_name == "id" || field_name == tenant_field || 
           field_name == "created_at" || field_name == "updated_at" {
            quote! {}
        } else {
            quote! { #field_name, }
        }
    }).filter(|tokens| !tokens.is_empty()).collect();
    
    let tenant_ident = syn::Ident::new(tenant_field, proc_macro2::Span::call_site());
    
    quote! {
        impl #struct_name {
            /// Create a new instance with auto-generated standard fields
            pub fn new(
                #tenant_ident: String,
                #(#field_params)*
            ) -> Self {
                let now = chrono::Utc::now();
                Self {
                    id: uuid::Uuid::new_v4(),
                    #tenant_ident,
                    created_at: now,
                    updated_at: now,
                    #(#field_assigns)*
                }
            }
            
            /// Update the updated_at timestamp
            pub fn touch(&mut self) {
                self.updated_at = chrono::Utc::now();
            }
            
            /// Check if this entity belongs to the given product/tenant
            pub fn belongs_to_product(&self, product: &str) -> bool {
                self.#tenant_ident == product
            }
        }
    }
}

/// Generate validation implementation
fn generate_validation_implementation(struct_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            /// Validate this entity (override in specific implementations)
            pub fn validate(&self) -> Result<(), String> {
                // Basic validation - entity has required fields
                if self.id.is_nil() {
                    return Err("ID cannot be nil".to_string());
                }
                
                if self.product.trim().is_empty() {
                    return Err("Product field cannot be empty".to_string());
                }
                
                Ok(())
            }
            
            /// Check if entity is valid
            pub fn is_valid(&self) -> bool {
                self.validate().is_ok()
            }
        }
    }
}

/// Generate query helper implementation
fn generate_query_implementation(struct_name: &syn::Ident) -> TokenStream2 {
    let table_name_method = quote! { Self::table_name() };
    
    quote! {
        impl #struct_name {
            /// Generate SELECT query for this entity by ID
            pub fn select_by_id_query() -> String {
                format!("SELECT * FROM {} WHERE id = $1 AND product = $2", #table_name_method)
            }
            
            /// Generate INSERT query for this entity
            pub fn insert_query(field_count: usize) -> String {
                let placeholders: Vec<String> = (1..=field_count).map(|i| format!("${}", i)).collect();
                format!("INSERT INTO {} VALUES ({})", #table_name_method, placeholders.join(", "))
            }
            
            /// Generate UPDATE query for this entity
            pub fn update_query(fields: &[&str]) -> String {
                let set_clauses: Vec<String> = fields.iter().enumerate()
                    .map(|(i, field)| format!("{} = ${}", field, i + 1))
                    .collect();
                format!("UPDATE {} SET {} WHERE id = ${} AND product = ${}",
                    #table_name_method, 
                    set_clauses.join(", "),
                    fields.len() + 1,
                    fields.len() + 2
                )
            }
            
            /// Generate DELETE query for this entity
            pub fn delete_query() -> String {
                format!("DELETE FROM {} WHERE id = $1 AND product = $2", #table_name_method)
            }
            
            /// Generate COUNT query for this entity type in product
            pub fn count_by_product_query() -> String {
                format!("SELECT COUNT(*) FROM {} WHERE product = $1", #table_name_method)
            }
        }
    }
}