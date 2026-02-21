//! DatabaseMapper derive macro implementation
//!
//! Auto-generates database row to struct mappings, eliminating ~400 lines of 
//! repetitive mapping code per entity.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{
    parse_macro_input, DeriveInput, Data, Fields, Field, Type, Attribute, Meta, NestedMeta, Lit,
    PathSegment, GenericArgument, TypePath, AngleBracketedGenericArguments, Ident
};

/// Field mapping configuration
#[derive(Default)]
struct FieldMapping {
    db_column: Option<String>,
    json_field: bool,
    enum_conversion: bool,
    optional: bool,
    custom_type: Option<String>,
}

/// Database mapping configuration
#[derive(Default)]
struct DatabaseConfig {
    table: Option<String>,
    primary_key: Option<String>,
}

impl DatabaseConfig {
    fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut config = DatabaseConfig::default();
        
        for attr in attrs {
            if attr.path.is_ident("database") {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    for nested_meta in meta_list.nested {
                        if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested_meta {
                            if let Lit::Str(lit_str) = name_value.lit {
                                match name_value.path.get_ident().map(|i| i.to_string()).as_deref() {
                                    Some("table") => config.table = Some(lit_str.value()),
                                    Some("primary_key") => config.primary_key = Some(lit_str.value()),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        
        config
    }
}

impl FieldMapping {
    fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut mapping = FieldMapping::default();
        
        for attr in attrs {
            if attr.path.is_ident("db") {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    for nested_meta in meta_list.nested {
                        match nested_meta {
                            NestedMeta::Meta(Meta::NameValue(name_value)) => {
                                if let Lit::Str(lit_str) = name_value.lit {
                                    if name_value.path.is_ident("column") {
                                        mapping.db_column = Some(lit_str.value());
                                    } else if name_value.path.is_ident("type") {
                                        mapping.custom_type = Some(lit_str.value());
                                    }
                                }
                            }
                            NestedMeta::Meta(Meta::Path(path)) => {
                                if path.is_ident("json") {
                                    mapping.json_field = true;
                                } else if path.is_ident("enum") {
                                    mapping.enum_conversion = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        mapping
    }
}

fn is_option_type(ty: &Type) -> (bool, Option<&Type>) {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.first() {
                        return (true, Some(inner_type));
                    }
                }
            }
        }
    }
    (false, None)
}

fn extract_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            type_path.path.segments.last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
        _ => "Unknown".to_string()
    }
}

pub fn derive_database_mapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let config = DatabaseConfig::from_attrs(&input.attrs);
    
    let table_name = config.table.unwrap_or_else(|| {
        format!("{}s", struct_name.to_string().to_lowercase())
    });
    
    let primary_key = config.primary_key.unwrap_or_else(|| "id".to_string());
    
    // Extract fields from the struct
    let fields = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => panic!("DatabaseMapper only works with named fields"),
            }
        }
        _ => panic!("DatabaseMapper only works with structs"),
    };
    
    // Generate from_row method
    let mut from_row_assignments = Vec::new();
    let mut to_params_assignments = Vec::new();
    let mut column_list = Vec::new();
    let mut placeholders = Vec::new();
    let mut update_assignments = Vec::new();
    
    for (i, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let mapping = FieldMapping::from_attrs(&field.attrs);
        
        let db_column = mapping.db_column.unwrap_or_else(|| field_name.to_string());
        let (is_optional, inner_type) = is_option_type(field_type);
        
        column_list.push(db_column.clone());
        placeholders.push(format!("${}", i + 1));
        
        if field_name.to_string() != primary_key {
            update_assignments.push(format!("{} = ${}", db_column, i + 1));
        }
        
        // Generate from_row assignment based on field type and mapping
        let assignment = if mapping.json_field {
            if is_optional {
                quote! {
                    #field_name: row.#field_name.as_ref().and_then(|json| 
                        serde_json::from_value(json.clone()).ok()
                    )
                }
            } else {
                quote! {
                    #field_name: serde_json::from_value(row.#field_name.clone())
                        .map_err(|e| sqlx::Error::ColumnDecode {
                            index: stringify!(#field_name).to_string(),
                            source: Box::new(e),
                        })?
                }
            }
        } else if mapping.enum_conversion {
            let type_name = if let Some(inner) = inner_type {
                extract_type_name(inner)
            } else {
                extract_type_name(field_type)
            };
            let type_ident = format_ident!("{}", type_name);
            
            if is_optional {
                quote! {
                    #field_name: row.#field_name.as_ref().and_then(|s| 
                        #type_ident::from_str(s).ok()
                    )
                }
            } else {
                quote! {
                    #field_name: #type_ident::from_str(&row.#field_name)
                        .map_err(|e| sqlx::Error::ColumnDecode {
                            index: stringify!(#field_name).to_string(), 
                            source: Box::new(e),
                        })?
                }
            }
        } else if mapping.custom_type.is_some() {
            // Handle custom type conversion
            let custom_type = mapping.custom_type.unwrap();
            let conversion = format_ident!("{}", custom_type);
            
            quote! {
                #field_name: #conversion::try_from(row.#field_name)?
            }
        } else {
            // Direct assignment
            quote! {
                #field_name: row.#field_name
            }
        };
        
        from_row_assignments.push(assignment);
        
        // Generate to_params assignment
        let param_assignment = if mapping.json_field {
            quote! {
                serde_json::to_value(&self.#field_name)
                    .map_err(|e| sqlx::Error::Encode(Box::new(e)))?
            }
        } else if mapping.enum_conversion {
            if is_optional {
                quote! {
                    self.#field_name.as_ref().map(|e| e.to_string())
                }
            } else {
                quote! {
                    self.#field_name.to_string()
                }
            }
        } else {
            quote! { &self.#field_name }
        };
        
        to_params_assignments.push(param_assignment);
    }
    
    let column_list_str = column_list.join(", ");
    let placeholders_str = placeholders.join(", ");
    let update_assignments_str = update_assignments.join(", ");
    
    let expanded = quote! {
        impl #struct_name {
            /// Create entity from database row
            pub fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
                use sqlx::Row;
                
                Ok(Self {
                    #(#from_row_assignments),*
                })
            }
            
            /// Convert entity to database parameters for insert
            pub fn to_insert_params(&self) -> Result<Vec<Box<dyn sqlx::postgres::PgArgumentBuffer>>, sqlx::Error> {
                let mut params = Vec::new();
                
                // This would be filled with actual parameter conversion
                // For now, this is a placeholder that needs to be implemented
                // based on the actual field types
                
                Ok(params)
            }
            
            /// Get SQL INSERT statement for this entity
            pub const fn insert_sql() -> &'static str {
                concat!(
                    "INSERT INTO ", #table_name, " (", #column_list_str, ") VALUES (", #placeholders_str, ") RETURNING *"
                )
            }
            
            /// Get SQL SELECT statement for finding by primary key
            pub const fn find_by_id_sql() -> &'static str {
                concat!(
                    "SELECT ", #column_list_str, " FROM ", #table_name, " WHERE ", #primary_key, " = $1"
                )
            }
            
            /// Get SQL UPDATE statement for this entity
            pub const fn update_sql() -> &'static str {
                concat!(
                    "UPDATE ", #table_name, " SET ", #update_assignments_str, " WHERE ", #primary_key, " = $1 RETURNING *"
                )
            }
            
            /// Get SQL DELETE statement for this entity
            pub const fn delete_sql() -> &'static str {
                concat!(
                    "DELETE FROM ", #table_name, " WHERE ", #primary_key, " = $1"
                )
            }
            
            /// Get table name
            pub const fn table_name() -> &'static str {
                #table_name
            }
            
            /// Get primary key column name
            pub const fn primary_key() -> &'static str {
                #primary_key
            }
            
            /// Get all column names
            pub const fn columns() -> &'static [&'static str] {
                &[#(#column_list),*]
            }
            
            /// Convert entity to JSON for caching
            pub fn to_cache_json(&self) -> Result<String, serde_json::Error> {
                serde_json::to_string(self)
            }
            
            /// Create entity from cached JSON
            pub fn from_cache_json(json: &str) -> Result<Self, serde_json::Error> {
                serde_json::from_str(json)
            }
            
            /// Create a query builder for this entity type
            pub fn query_builder() -> DatabaseQueryBuilder<#struct_name> {
                DatabaseQueryBuilder::new(#table_name)
            }
            
            /// Get entity metadata for introspection
            pub fn entity_metadata() -> EntityMetadata {
                EntityMetadata {
                    name: stringify!(#struct_name),
                    table: #table_name,
                    primary_key: #primary_key,
                    columns: Self::columns().to_vec(),
                    supports_soft_delete: false, // Could be made configurable
                    supports_timestamps: true,   // Could be made configurable
                }
            }
        }
        
        /// Query builder for enhanced database operations
        pub struct DatabaseQueryBuilder<T> {
            table: String,
            wheres: Vec<String>,
            orders: Vec<String>,
            limit: Option<i64>,
            offset: Option<i64>,
            _phantom: std::marker::PhantomData<T>,
        }
        
        impl<T> DatabaseQueryBuilder<T> {
            pub fn new(table: &str) -> Self {
                Self {
                    table: table.to_string(),
                    wheres: Vec::new(),
                    orders: Vec::new(),
                    limit: None,
                    offset: None,
                    _phantom: std::marker::PhantomData,
                }
            }
            
            pub fn where_clause(mut self, clause: &str) -> Self {
                self.wheres.push(clause.to_string());
                self
            }
            
            pub fn order_by(mut self, column: &str, direction: &str) -> Self {
                self.orders.push(format!("{} {}", column, direction));
                self
            }
            
            pub fn limit(mut self, limit: i64) -> Self {
                self.limit = Some(limit);
                self
            }
            
            pub fn offset(mut self, offset: i64) -> Self {
                self.offset = Some(offset);
                self
            }
            
            pub fn build_select(&self) -> String {
                let mut query = format!("SELECT * FROM {}", self.table);
                
                if !self.wheres.is_empty() {
                    query.push_str(&format!(" WHERE {}", self.wheres.join(" AND ")));
                }
                
                if !self.orders.is_empty() {
                    query.push_str(&format!(" ORDER BY {}", self.orders.join(", ")));
                }
                
                if let Some(limit) = self.limit {
                    query.push_str(&format!(" LIMIT {}", limit));
                }
                
                if let Some(offset) = self.offset {
                    query.push_str(&format!(" OFFSET {}", offset));
                }
                
                query
            }
        }
        
        /// Entity metadata for runtime introspection
        #[derive(Debug, Clone)]
        pub struct EntityMetadata {
            pub name: &'static str,
            pub table: &'static str,
            pub primary_key: &'static str,
            pub columns: Vec<&'static str>,
            pub supports_soft_delete: bool,
            pub supports_timestamps: bool,
        }
        
        impl std::fmt::Display for EntityMetadata {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Entity {} -> Table {} (PK: {})", 
                       self.name, self.table, self.primary_key)
            }
        }
    };
    
    eprintln!("[pleme-codegen] DatabaseMapper pattern applied to {}", struct_name);
    TokenStream::from(expanded)
}