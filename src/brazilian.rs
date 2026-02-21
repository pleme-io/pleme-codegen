//! Brazilian market specific macro implementations
//!
//! Provides macros for handling Brazilian business requirements:
//! - CPF (individual taxpayer registry) validation and formatting
//! - CEP (postal code) validation and formatting  
//! - CNPJ (business registry) validation
//! - Brazilian phone number formatting
//! - PIX payment integration

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use crate::utils::*;

/// Implementation of the BrazilianEntity derive macro
pub fn derive_brazilian_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("BrazilianEntity can only be used with structs with named fields"),
        },
        _ => panic!("BrazilianEntity can only be used with structs"),
    };
    
    // Find fields with Brazilian attributes
    let mut brazilian_implementations = Vec::new();
    
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        
        // Check for CPF field
        if has_attribute_flag(&field.attrs, "brazilian", "cpf") {
            brazilian_implementations.push(generate_cpf_field_implementation(struct_name, field_name));
        }
        
        // Check for CEP field
        if has_attribute_flag(&field.attrs, "brazilian", "cep") {
            brazilian_implementations.push(generate_cep_field_implementation(struct_name, field_name));
        }
        
        // Check for CNPJ field
        if has_attribute_flag(&field.attrs, "brazilian", "cnpj") {
            brazilian_implementations.push(generate_cnpj_field_implementation(struct_name, field_name));
        }
        
        // Check for phone field
        if has_attribute_flag(&field.attrs, "brazilian", "phone") {
            brazilian_implementations.push(generate_phone_field_implementation(struct_name, field_name));
        }
    }
    
    // Generate general Brazilian validation utilities
    let validation_utils = generate_brazilian_validation_utils();
    
    let expanded = quote! {
        #validation_utils
        
        #(#brazilian_implementations)*
        
        impl #struct_name {
            /// Check if this entity represents a Brazilian customer/business
            pub fn is_brazilian_entity(&self) -> bool {
                // This can be overridden by specific implementations
                true
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Generate CPF field implementation
fn generate_cpf_field_implementation(struct_name: &syn::Ident, field_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            /// Validate the CPF field
            pub fn validate_cpf_field(&self) -> Result<(), String> {
                if let Some(ref cpf) = self.#field_name {
                    if !validate_cpf(cpf) {
                        return Err(format!("Invalid CPF: {}", cpf));
                    }
                }
                Ok(())
            }
            
            /// Format the CPF field for display
            pub fn format_cpf_field(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cpf| format_cpf(cpf))
            }
            
            /// Get CPF digits only (no formatting)
            pub fn cpf_digits(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cpf| {
                    cpf.chars().filter(|c| c.is_ascii_digit()).collect()
                })
            }
            
            /// Set CPF from string (validates and stores)
            pub fn set_cpf(&mut self, cpf: &str) -> Result<(), String> {
                if validate_cpf(cpf) {
                    self.#field_name = Some(cpf.to_string());
                    self.touch(); // Update timestamp if available
                    Ok(())
                } else {
                    Err(format!("Invalid CPF: {}", cpf))
                }
            }
        }
    }
}

/// Generate CEP field implementation
fn generate_cep_field_implementation(struct_name: &syn::Ident, field_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            /// Validate the CEP field
            pub fn validate_cep_field(&self) -> Result<(), String> {
                if let Some(ref cep) = self.#field_name {
                    if !validate_cep(cep) {
                        return Err(format!("Invalid CEP: {}", cep));
                    }
                }
                Ok(())
            }
            
            /// Format the CEP field for display (XXXXX-XXX)
            pub fn format_cep_field(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cep| format_cep(cep))
            }
            
            /// Get CEP digits only (no formatting)
            pub fn cep_digits(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cep| {
                    cep.chars().filter(|c| c.is_ascii_digit()).collect()
                })
            }
            
            /// Set CEP from string (validates and stores)
            pub fn set_cep(&mut self, cep: &str) -> Result<(), String> {
                if validate_cep(cep) {
                    self.#field_name = Some(cep.to_string());
                    self.touch(); // Update timestamp if available
                    Ok(())
                } else {
                    Err(format!("Invalid CEP: {}", cep))
                }
            }
        }
    }
}

/// Generate CNPJ field implementation
fn generate_cnpj_field_implementation(struct_name: &syn::Ident, field_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            /// Validate the CNPJ field
            pub fn validate_cnpj_field(&self) -> Result<(), String> {
                if let Some(ref cnpj) = self.#field_name {
                    if !validate_cnpj(cnpj) {
                        return Err(format!("Invalid CNPJ: {}", cnpj));
                    }
                }
                Ok(())
            }
            
            /// Format the CNPJ field for display (XX.XXX.XXX/XXXX-XX)
            pub fn format_cnpj_field(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cnpj| format_cnpj(cnpj))
            }
            
            /// Get CNPJ digits only (no formatting)
            pub fn cnpj_digits(&self) -> Option<String> {
                self.#field_name.as_ref().map(|cnpj| {
                    cnpj.chars().filter(|c| c.is_ascii_digit()).collect()
                })
            }
            
            /// Set CNPJ from string (validates and stores)
            pub fn set_cnpj(&mut self, cnpj: &str) -> Result<(), String> {
                if validate_cnpj(cnpj) {
                    self.#field_name = Some(cnpj.to_string());
                    self.touch(); // Update timestamp if available
                    Ok(())
                } else {
                    Err(format!("Invalid CNPJ: {}", cnpj))
                }
            }
        }
    }
}

/// Generate phone field implementation
fn generate_phone_field_implementation(struct_name: &syn::Ident, field_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            /// Validate the Brazilian phone field
            pub fn validate_phone_field(&self) -> Result<(), String> {
                if let Some(ref phone) = self.#field_name {
                    if !validate_brazilian_phone(phone) {
                        return Err(format!("Invalid Brazilian phone: {}", phone));
                    }
                }
                Ok(())
            }
            
            /// Format the phone field for display
            pub fn format_phone_field(&self) -> Option<String> {
                self.#field_name.as_ref().map(|phone| format_brazilian_phone(phone))
            }
            
            /// Get phone digits only (no formatting)
            pub fn phone_digits(&self) -> Option<String> {
                self.#field_name.as_ref().map(|phone| {
                    phone.chars().filter(|c| c.is_ascii_digit()).collect()
                })
            }
            
            /// Set phone from string (validates and stores)
            pub fn set_phone(&mut self, phone: &str) -> Result<(), String> {
                if validate_brazilian_phone(phone) {
                    self.#field_name = Some(phone.to_string());
                    self.touch(); // Update timestamp if available
                    Ok(())
                } else {
                    Err(format!("Invalid Brazilian phone: {}", phone))
                }
            }
        }
    }
}

/// Generate Brazilian validation utility functions
fn generate_brazilian_validation_utils() -> TokenStream2 {
    quote! {
        /// Brazilian CPF validation
        pub fn validate_cpf(cpf: &str) -> bool {
            let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
            
            if digits.len() != 11 {
                return false;
            }
            
            // Check for known invalid patterns (all same digits)
            if digits.chars().all(|c| c == digits.chars().next().unwrap()) {
                return false;
            }
            
            // Calculate verification digits
            let digits: Vec<u32> = digits.chars().map(|c| c.to_digit(10).unwrap()).collect();
            
            // First verification digit
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
            
            // Second verification digit
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
        
        /// Brazilian CNPJ validation
        pub fn validate_cnpj(cnpj: &str) -> bool {
            let digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
            
            if digits.len() != 14 {
                return false;
            }
            
            // Check for known invalid patterns
            if digits.chars().all(|c| c == digits.chars().next().unwrap()) {
                return false;
            }
            
            let digits: Vec<u32> = digits.chars().map(|c| c.to_digit(10).unwrap()).collect();
            
            // First verification digit
            let weights1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
            let sum1: u32 = digits[0..12].iter().zip(weights1.iter())
                .map(|(&d, &w)| d * w)
                .sum();
            let check1 = match sum1 % 11 {
                0 | 1 => 0,
                n => 11 - n,
            };
            
            if check1 != digits[12] {
                return false;
            }
            
            // Second verification digit
            let weights2 = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
            let sum2: u32 = digits[0..13].iter().zip(weights2.iter())
                .map(|(&d, &w)| d * w)
                .sum();
            let check2 = match sum2 % 11 {
                0 | 1 => 0,
                n => 11 - n,
            };
            
            check2 == digits[13]
        }
        
        /// Format CNPJ for display (XX.XXX.XXX/XXXX-XX)
        pub fn format_cnpj(cnpj: &str) -> String {
            let digits: String = cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 14 {
                format!("{}.{}.{}/{}-{}", 
                    &digits[0..2], &digits[2..5], 
                    &digits[5..8], &digits[8..12], 
                    &digits[12..14])
            } else {
                cnpj.to_string()
            }
        }
        
        /// Brazilian CEP validation
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
        
        /// Brazilian phone number validation (landline and mobile)
        pub fn validate_brazilian_phone(phone: &str) -> bool {
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            
            // With country code: 11 digits (5511XXXXXXXXX)
            // Without country code: 10 or 11 digits (11XXXXXXXXX or 11XXXXXXXXX)
            match digits.len() {
                10 => true, // Landline without country code
                11 => {
                    // Mobile without country code or landline with country code
                    let first_digit = digits.chars().nth(2).unwrap_or('0');
                    first_digit >= '6' // Mobile numbers start with 6, 7, 8, 9
                }
                13 => {
                    // With country code +55
                    digits.starts_with("55")
                }
                _ => false,
            }
        }
        
        /// Format Brazilian phone for display
        pub fn format_brazilian_phone(phone: &str) -> String {
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            
            match digits.len() {
                10 => format!("({}) {}-{}", &digits[0..2], &digits[2..6], &digits[6..10]),
                11 => format!("({}) {} {}-{}", &digits[0..2], &digits[2..3], &digits[3..7], &digits[7..11]),
                13 => format!("+{} ({}) {} {}-{}", &digits[0..2], &digits[2..4], &digits[4..5], &digits[5..9], &digits[9..13]),
                _ => phone.to_string(),
            }
        }
    }
}

/// CPF field attribute macro implementation
pub fn cpf_field(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // For now, just return the input unchanged
    // This would be expanded to add validation to specific fields
    TokenStream::from(quote! { #input })
}

/// CEP field attribute macro implementation  
pub fn cep_field(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // For now, just return the input unchanged
    // This would be expanded to add validation to specific fields
    TokenStream::from(quote! { #input })
}