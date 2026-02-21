//! Identifier Generation Pattern Macros
//! 
//! Unique identifier generation with customizable formats

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// IdentifierEntity - Generate unique identifiers (saves ~10 lines per entity)
pub fn derive_identifier_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    eprintln!("[pleme-codegen] IdentifierEntity pattern applied to {} - saving ~10 lines", struct_name);
    
    let expanded = quote! {
        impl #struct_name {
            /// Generate unique identifier with customizable format
            pub fn generate_identifier(prefix: &str) -> String {
                let timestamp = chrono::Utc::now();
                let uuid_short = uuid::Uuid::new_v4()
                    .to_string()
                    .chars()
                    .take(8)
                    .collect::<String>()
                    .to_uppercase();
                
                let identifier = format!("{}-{}-{}", 
                    prefix,
                    timestamp.format("%Y%m%d%H%M%S"),
                    uuid_short
                );
                
                tracing::debug!(
                    entity = %stringify!(#struct_name),
                    prefix = %prefix,
                    identifier = %identifier,
                    "Generated unique identifier"
                );
                
                identifier
            }
            
            /// Generate order number with Brazilian format
            pub fn generate_order_number() -> String {
                let timestamp = chrono::Utc::now();
                let random = rand::random::<u32>() % 10000;
                
                format!("PED-{}{:04}", 
                    timestamp.format("%Y%m%d"),
                    random
                )
            }
            
            /// Generate invoice number (NFe compatible)
            pub fn generate_invoice_number() -> String {
                let timestamp = chrono::Utc::now();
                let sequential = Self::get_next_sequential();
                
                format!("NF-{}-{:06}", 
                    timestamp.format("%Y%m"),
                    sequential
                )
            }
            
            /// Generate tracking code
            pub fn generate_tracking_code() -> String {
                let chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
                let mut rng = rand::thread_rng();
                
                let code: String = (0..16)
                    .map(|_| {
                        let idx = rand::Rng::gen_range(&mut rng, 0..chars.len());
                        chars.chars().nth(idx).unwrap()
                    })
                    .collect();
                
                // Format as BR-XXXX-XXXX-XXXX-XXXX
                format!("BR-{}-{}-{}-{}", 
                    &code[0..4], 
                    &code[4..8], 
                    &code[8..12], 
                    &code[12..16]
                )
            }
            
            /// Generate customer code
            pub fn generate_customer_code() -> String {
                Self::generate_identifier("CLI")
            }
            
            /// Generate product SKU
            pub fn generate_sku(category: &str) -> String {
                let timestamp = chrono::Utc::now();
                let random = rand::random::<u16>();
                
                format!("{}-{}{:05}", 
                    category.to_uppercase(),
                    timestamp.format("%y%m"),
                    random
                )
            }
            
            /// Generate transaction ID
            pub fn generate_transaction_id() -> String {
                let uuid = uuid::Uuid::new_v4();
                let timestamp = chrono::Utc::now();
                
                format!("TXN-{}-{}", 
                    timestamp.timestamp(),
                    uuid.to_string().replace("-", "").chars().take(12).collect::<String>()
                )
            }
            
            /// Parse identifier to extract components
            pub fn parse_identifier(identifier: &str) -> Option<(String, String, String)> {
                let parts: Vec<&str> = identifier.split('-').collect();
                
                if parts.len() >= 3 {
                    Some((
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                    ))
                } else {
                    None
                }
            }
            
            /// Validate identifier format
            pub fn is_valid_identifier(identifier: &str, expected_prefix: &str) -> bool {
                if let Some((prefix, timestamp, unique_part)) = Self::parse_identifier(identifier) {
                    prefix == expected_prefix && 
                    timestamp.len() >= 8 &&
                    !unique_part.is_empty()
                } else {
                    false
                }
            }
            
            /// Get next sequential number (would connect to Redis/DB in real implementation)
            fn get_next_sequential() -> u32 {
                // In production, this would atomically increment a counter in Redis/DB
                rand::random::<u32>() % 1000000
            }
            
            /// Generate short code for URLs
            pub fn generate_short_code(length: usize) -> String {
                use rand::Rng;
                const CHARSET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
                let mut rng = rand::thread_rng();
                
                (0..length)
                    .map(|_| {
                        let idx = rng.gen_range(0..CHARSET.len());
                        CHARSET[idx] as char
                    })
                    .collect()
            }
            
            /// Generate barcode (EAN-13 compatible)
            pub fn generate_barcode(country_code: &str, manufacturer_code: &str) -> String {
                let product_code = rand::random::<u32>() % 10000;
                let base = format!("{}{}{:05}", country_code, manufacturer_code, product_code);
                
                // Calculate check digit (simplified)
                let check_digit = Self::calculate_ean13_check_digit(&base);
                
                format!("{}{}", base, check_digit)
            }
            
            fn calculate_ean13_check_digit(code: &str) -> u8 {
                let digits: Vec<u8> = code.chars()
                    .filter_map(|c| c.to_digit(10).map(|d| d as u8))
                    .collect();
                
                let sum: u32 = digits.iter().enumerate()
                    .map(|(i, &d)| {
                        if i % 2 == 0 { d as u32 } else { (d * 3) as u32 }
                    })
                    .sum();
                
                ((10 - (sum % 10)) % 10) as u8
            }
        }
        
    };
    
    TokenStream::from(expanded)
}