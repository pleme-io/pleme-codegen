//! GraphQL Bridge macro implementation
//!
//! Automatically generates GraphQL-compatible types and conversions for Rust domain models.
//! Handles common type mismatches like Decimal <-> f64, JSON values, and optional types.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type, PathArguments, GenericArgument};

use crate::utils::*;

/// Implementation of the GraphQLBridge derive macro
pub fn derive_graphql_bridge(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    // Generate GraphQL types
    let graphql_input_name = syn::Ident::new(&format!("{}Input", struct_name_str), proc_macro2::Span::call_site());
    let graphql_object_name = syn::Ident::new(&format!("{}Object", struct_name_str), proc_macro2::Span::call_site());
    
    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("GraphQLBridge can only be used with structs with named fields"),
        },
        _ => panic!("GraphQLBridge can only be used with structs"),
    };
    
    // Generate GraphQL-compatible fields
    let (input_fields, object_fields) = generate_graphql_fields(fields);
    
    // Generate conversion implementations
    let to_graphql_impl = generate_to_graphql_conversion(struct_name, &graphql_object_name, fields);
    let from_graphql_impl = generate_from_graphql_conversion(struct_name, &graphql_input_name, fields);
    
    let expanded = quote! {
        /// GraphQL Input type for #struct_name
        #[derive(async_graphql::InputObject, Debug, Clone)]
        pub struct #graphql_input_name {
            #(#input_fields)*
        }
        
        /// GraphQL Object type for #struct_name
        #[derive(async_graphql::SimpleObject, Debug, Clone)]
        pub struct #graphql_object_name {
            #(#object_fields)*
        }
        
        #to_graphql_impl
        #from_graphql_impl
    };
    
    TokenStream::from(expanded)
}

/// Generate GraphQL-compatible field definitions
fn generate_graphql_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>
) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    let mut input_fields = Vec::new();
    let mut object_fields = Vec::new();
    
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        
        let (input_type, object_type) = convert_type_for_graphql(field_type);
        
        // Generate GraphQL documentation from Rust doc comments
        let doc_comment = format!("GraphQL field for {}", field_name);
        
        input_fields.push(quote! {
            #[doc = #doc_comment]
            pub #field_name: #input_type,
        });
        
        object_fields.push(quote! {
            #[doc = #doc_comment]
            pub #field_name: #object_type,
        });
    }
    
    (input_fields, object_fields)
}

/// Convert Rust type to GraphQL-compatible type
fn convert_type_for_graphql(ty: &Type) -> (TokenStream2, TokenStream2) {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            
            // Handle Option<T>
            if path.segments.len() == 1 && path.segments[0].ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        let (inner_input, inner_object) = convert_type_for_graphql(inner_type);
                        return (
                            quote! { Option<#inner_input> },
                            quote! { Option<#inner_object> }
                        );
                    }
                }
            }
            
            // Handle specific types that need conversion
            let type_str = quote! { #path }.to_string();
            match type_str.as_str() {
                "rust_decimal :: Decimal" | "Decimal" => {
                    (quote! { f64 }, quote! { f64 })
                }
                "serde_json :: Value" | "Value" => {
                    (quote! { async_graphql::Json<serde_json::Value> }, quote! { async_graphql::Json<serde_json::Value> })
                }
                "chrono :: DateTime < chrono :: Utc >" | "DateTime < Utc >" => {
                    (quote! { chrono::DateTime<chrono::Utc> }, quote! { chrono::DateTime<chrono::Utc> })
                }
                "uuid :: Uuid" | "Uuid" => {
                    (quote! { uuid::Uuid }, quote! { uuid::Uuid })
                }
                _ => {
                    // Default: use the type as-is
                    (quote! { #ty }, quote! { #ty })
                }
            }
        }
        _ => {
            // For other types, use as-is
            (quote! { #ty }, quote! { #ty })
        }
    }
}

/// Generate conversion from domain model to GraphQL object
fn generate_to_graphql_conversion(
    struct_name: &syn::Ident,
    graphql_object_name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>
) -> TokenStream2 {
    let field_conversions: Vec<TokenStream2> = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        
        let conversion = generate_field_to_graphql_conversion(field_name, field_type);
        
        quote! {
            #field_name: #conversion,
        }
    }).collect();
    
    quote! {
        impl From<#struct_name> for #graphql_object_name {
            fn from(entity: #struct_name) -> Self {
                Self {
                    #(#field_conversions)*
                }
            }
        }
        
        impl #struct_name {
            /// Convert to GraphQL object
            pub fn to_graphql_object(self) -> #graphql_object_name {
                self.into()
            }
        }
    }
}

/// Generate conversion from GraphQL input to domain model
fn generate_from_graphql_conversion(
    struct_name: &syn::Ident,
    graphql_input_name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>
) -> TokenStream2 {
    let field_conversions: Vec<TokenStream2> = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        
        let conversion = generate_field_from_graphql_conversion(field_name, field_type);
        
        quote! {
            #field_name: #conversion,
        }
    }).collect();
    
    quote! {
        impl From<#graphql_input_name> for #struct_name {
            fn from(input: #graphql_input_name) -> Self {
                Self {
                    #(#field_conversions)*
                }
            }
        }
        
        impl #graphql_input_name {
            /// Convert to domain model
            pub fn to_domain_model(self) -> #struct_name {
                self.into()
            }
        }
    }
}

/// Generate field conversion from domain model to GraphQL
fn generate_field_to_graphql_conversion(field_name: &syn::Ident, field_type: &Type) -> TokenStream2 {
    match field_type {
        Type::Path(type_path) => {
            let path = &type_path.path;
            
            // Handle Option<T>
            if path.segments.len() == 1 && path.segments[0].ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        let inner_conversion = generate_inner_type_to_graphql_conversion(inner_type);
                        return quote! { entity.#field_name.map(|v| #inner_conversion) };
                    }
                }
            }
            
            let type_str = quote! { #path }.to_string();
            match type_str.as_str() {
                "rust_decimal :: Decimal" | "Decimal" => {
                    quote! { { use num_traits::ToPrimitive; entity.#field_name.to_f64().unwrap_or(0.0) } }
                }
                "serde_json :: Value" | "Value" => {
                    quote! { async_graphql::Json(entity.#field_name) }
                }
                _ => {
                    quote! { entity.#field_name }
                }
            }
        }
        _ => {
            quote! { entity.#field_name }
        }
    }
}

/// Generate field conversion from GraphQL to domain model
fn generate_field_from_graphql_conversion(field_name: &syn::Ident, field_type: &Type) -> TokenStream2 {
    match field_type {
        Type::Path(type_path) => {
            let path = &type_path.path;
            
            // Handle Option<T>
            if path.segments.len() == 1 && path.segments[0].ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        let inner_conversion = generate_inner_type_from_graphql_conversion(inner_type);
                        return quote! { input.#field_name.map(|v| #inner_conversion) };
                    }
                }
            }
            
            let type_str = quote! { #path }.to_string();
            match type_str.as_str() {
                "rust_decimal :: Decimal" | "Decimal" => {
                    quote! { rust_decimal::Decimal::from_f64_retain(input.#field_name).unwrap_or_default() }
                }
                "serde_json :: Value" | "Value" => {
                    quote! { input.#field_name.0 }
                }
                _ => {
                    quote! { input.#field_name }
                }
            }
        }
        _ => {
            quote! { input.#field_name }
        }
    }
}

/// Generate inner type conversion for Option<T> to GraphQL
fn generate_inner_type_to_graphql_conversion(inner_type: &Type) -> TokenStream2 {
    match inner_type {
        Type::Path(type_path) => {
            let type_str = quote! { #type_path }.to_string();
            match type_str.as_str() {
                "rust_decimal :: Decimal" | "Decimal" => {
                    quote! { { use num_traits::ToPrimitive; v.to_f64().unwrap_or(0.0) } }
                }
                "serde_json :: Value" | "Value" => {
                    quote! { async_graphql::Json(v) }
                }
                _ => {
                    quote! { v }
                }
            }
        }
        _ => {
            quote! { v }
        }
    }
}

/// Generate inner type conversion for Option<T> from GraphQL
fn generate_inner_type_from_graphql_conversion(inner_type: &Type) -> TokenStream2 {
    match inner_type {
        Type::Path(type_path) => {
            let type_str = quote! { #type_path }.to_string();
            match type_str.as_str() {
                "rust_decimal :: Decimal" | "Decimal" => {
                    quote! { rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default() }
                }
                "serde_json :: Value" | "Value" => {
                    quote! { v.0 }
                }
                _ => {
                    quote! { v }
                }
            }
        }
        _ => {
            quote! { v }
        }
    }
}