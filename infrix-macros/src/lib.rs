//! Infrix Contract Procedural Macros
//!
//! This crate provides procedural macros for developing Infrix smart contracts.
//!
//! # Macros
//!
//! - `#[contract]` - Marks a struct as a contract, generating entry points
//! - `#[init]` - Marks the contract initialization function
//! - `#[call]` - Marks a function as callable (state-modifying)
//! - `#[view]` - Marks a function as view-only (read-only)
//! - `#[event]` - Generates event emission helpers
//! - `#[storage]` - Marks storage fields with automatic serialization
//!
//! # Example
//!
//! ```ignore
//! use ::infrix_sdk::prelude::*;
//!
//! #[contract]
//! pub struct Token {
//!     name: String,
//!     symbol: String,
//!     decimals: u8,
//!     total_supply: U256,
//! }
//!
//! #[event]
//! pub struct Transfer {
//!     from: Address,
//!     to: Address,
//!     amount: U256,
//! }
//!
//! impl Token {
//!     #[init]
//!     pub fn new(name: String, symbol: String, decimals: u8) -> Self {
//!         Self { name, symbol, decimals, total_supply: U256::ZERO }
//!     }
//!
//!     #[call]
//!     pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
//!         // Implementation
//!         Ok(())
//!     }
//!
//!     #[view]
//!     pub fn balance_of(&self, owner: Address) -> U256 {
//!         // Implementation
//!         U256::ZERO
//!     }
//! }
//! ```

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Expr, FnArg, Ident, ImplItem, ImplItemFn, ItemFn, ItemImpl, ItemStruct, Lit,
    LitInt, LitStr, Meta, Pat, ReturnType, Token, Type,
};

mod governance_macros;

/// Marks a struct as an Infrix smart contract.
///
/// This macro generates:
/// - WASM entry points for contract calls
/// - ABI encoding/decoding for function dispatch
/// - Storage layout management
/// - Contract metadata
///
/// # Attributes
///
/// - `#[contract(version = "1.0.0")]` - Specify contract version
/// - `#[contract(upgradeable)]` - Mark contract as upgradeable
///
/// # Example
///
/// ```ignore
/// #[contract]
/// pub struct MyContract {
///     owner: Address,
///     value: U256,
/// }
/// ```
#[proc_macro_attribute]
pub fn contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ContractArgs);
    let input = parse_macro_input!(item as ItemStruct);

    match generate_contract(args, input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Arguments for the contract macro
struct ContractArgs {
    version: Option<String>,
    upgradeable: bool,
}

impl Parse for ContractArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut version = None;
        let mut upgradeable = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "version" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        version = Some(s.value());
                    }
                }
                "upgradeable" => {
                    upgradeable = true;
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown contract attribute: {}", ident),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ContractArgs {
            version,
            upgradeable,
        })
    }
}

fn generate_contract(args: ContractArgs, input: ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let version = args.version.unwrap_or_else(|| "0.1.0".to_string());
    let upgradeable = args.upgradeable;

    // Generate storage key for the contract
    let storage_key = format!("__contract_{}", struct_name_str.to_lowercase());

    // Generate field accessors for storage
    let field_accessors = generate_field_accessors(&input)?;

    // Generate contract metadata
    let metadata = quote! {
        impl #struct_name {
            /// Contract name
            pub const CONTRACT_NAME: &'static str = #struct_name_str;

            /// Contract version
            pub const CONTRACT_VERSION: &'static str = #version;

            /// Storage key for contract state
            pub const STORAGE_KEY: &'static str = #storage_key;

            /// Whether the contract is upgradeable
            pub const UPGRADEABLE: bool = #upgradeable;
        }
    };

    // Generate ContractInstance trait implementation
    let contract_impl = quote! {
        impl ::infrix_sdk::infrix_types::ContractInstance for #struct_name {
            fn load() -> Option<Self> {
                let key = Self::STORAGE_KEY.as_bytes();
                let data = ::infrix_sdk::storage::get(key)?;
                Self::decode(&data).ok()
            }

            fn save(&self) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                let key = Self::STORAGE_KEY.as_bytes();
                let mut buffer = [0u8; 4096];
                let len = self.encode(&mut buffer)?;
                ::infrix_sdk::storage::set(key, &buffer[..len]);
                Ok(())
            }

            fn delete() {
                let key = Self::STORAGE_KEY.as_bytes();
                ::infrix_sdk::storage::delete(key);
            }
        }
    };

    // Generate encoding/decoding implementations
    let encoding = generate_encoding_impl(&input)?;

    Ok(quote! {
        #input

        #metadata

        #field_accessors

        #encoding

        #contract_impl
    })
}

fn generate_field_accessors(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let mut accessors = Vec::new();

    if let syn::Fields::Named(fields) = &input.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let storage_key = format!("_field_{}", field_name);

            // Generate getter
            let getter_name = format_ident!("get_{}", field_name);
            accessors.push(quote! {
                /// Get the value of #field_name from storage
                pub fn #getter_name() -> Option<#field_type> {
                    let key = #storage_key.as_bytes();
                    let data = ::infrix_sdk::storage::get(key)?;
                    <#field_type as ::infrix_sdk::infrix_types::Decode>::decode(&data).ok()
                }
            });

            // Generate setter
            let setter_name = format_ident!("set_{}", field_name);
            accessors.push(quote! {
                /// Set the value of #field_name in storage
                pub fn #setter_name(value: &#field_type) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                    let key = #storage_key.as_bytes();
                    let mut buffer = [0u8; 1024];
                    let len = <#field_type as ::infrix_sdk::infrix_types::Encode>::encode(value, &mut buffer)?;
                    ::infrix_sdk::storage::set(key, &buffer[..len]);
                    Ok(())
                }
            });
        }
    }

    Ok(quote! {
        impl #struct_name {
            #(#accessors)*
        }
    })
}

fn generate_encoding_impl(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let mut encode_fields = Vec::new();
    let mut decode_fields = Vec::new();
    let mut field_names = Vec::new();

    if let syn::Fields::Named(fields) = &input.fields {
        for (_i, field) in fields.named.iter().enumerate() {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            field_names.push(field_name.clone());

            encode_fields.push(quote! {
                offset += <#field_type as ::infrix_sdk::infrix_types::Encode>::encode(&self.#field_name, &mut buffer[offset..])?;
            });

            decode_fields.push(quote! {
                let (#field_name, consumed) = <#field_type as ::infrix_sdk::infrix_types::Decode>::decode_with_len(&data[offset..])?;
                offset += consumed;
            });
        }
    }

    Ok(quote! {
        impl ::infrix_sdk::infrix_types::Encode for #struct_name {
            fn encode(&self, buffer: &mut [u8]) -> Result<usize, ::infrix_sdk::infrix_types::Error> {
                let mut offset = 0;
                #(#encode_fields)*
                Ok(offset)
            }
        }

        impl ::infrix_sdk::infrix_types::Decode for #struct_name {
            fn decode(data: &[u8]) -> Result<Self, ::infrix_sdk::infrix_types::Error> {
                let (result, _) = Self::decode_with_len(data)?;
                Ok(result)
            }

            fn decode_with_len(data: &[u8]) -> Result<(Self, usize), ::infrix_sdk::infrix_types::Error> {
                let mut offset = 0;
                #(#decode_fields)*
                Ok((Self { #(#field_names),* }, offset))
            }
        }
    })
}

/// Marks a function as the contract initialization function.
///
/// The init function is called once when the contract is deployed.
/// It should return `Self` or `Result<Self, Error>`.
///
/// # Example
///
/// ```ignore
/// #[init]
/// pub fn new(name: String) -> Self {
///     Self { name, counter: U256::ZERO }
/// }
/// ```
#[proc_macro_attribute]
pub fn init(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ImplItemFn);

    match generate_init(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_init(input: ImplItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let inputs = &input.sig.inputs;
    let output = &input.sig.output;

    // Extract parameter types for ABI
    let params = extract_params(inputs)?;
    let param_decodes = generate_param_decodes(&params)?;
    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();

    // Generate the wrapper function
    let wrapper_name = format_ident!("__init_wrapper");

    Ok(quote! {
        #(#attrs)*
        #visibility fn #fn_name(#inputs) #output #block

        /// WASM entry point for initialization
        #[doc(hidden)]
        pub fn #wrapper_name(input: &[u8]) -> Result<Self, ::infrix_sdk::infrix_types::Error> {
            let mut offset = 0;
            #param_decodes
            let instance = Self::#fn_name(#(#param_names),*);

            // Handle Result return type
            let instance = match core::convert::Into::<Result<Self, ::infrix_sdk::infrix_types::Error>>::into(
                ::infrix_sdk::infrix_types::IntoResult::into_result(instance)
            ) {
                Ok(i) => i,
                Err(e) => return Err(e),
            };

            // Save the contract state
            ::infrix_sdk::infrix_types::ContractInstance::save(&instance)?;
            Ok(instance)
        }

        /// ABI signature for init function
        pub const INIT_SIGNATURE: &'static str = #fn_name_str;
    })
}

/// Marks a function as callable (state-modifying).
///
/// Call functions can modify contract state and emit events.
/// They are invoked via transactions.
///
/// # Attributes
///
/// - `#[call(payable)]` - Function can receive tokens
/// - `#[call(only_owner)]` - Only contract owner can call
///
/// # Example
///
/// ```ignore
/// #[call]
/// pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
///     // Implementation
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn call(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CallArgs);
    let input = parse_macro_input!(item as ImplItemFn);

    match generate_call(args, input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct CallArgs {
    payable: bool,
    only_owner: bool,
    selector: Option<u32>,
}

impl Parse for CallArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut payable = false;
        let mut only_owner = false;
        let mut selector = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "payable" => payable = true,
                "only_owner" => only_owner = true,
                "selector" => {
                    input.parse::<Token![=]>()?;
                    let lit: Lit = input.parse()?;
                    if let Lit::Int(i) = lit {
                        selector = Some(i.base10_parse()?);
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown call attribute: {}", ident),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(CallArgs {
            payable,
            only_owner,
            selector,
        })
    }
}

fn generate_call(args: CallArgs, input: ImplItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let inputs = &input.sig.inputs;
    let output = &input.sig.output;

    // Check for &mut self
    let has_mut_self = inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(r) if r.mutability.is_some()));

    if !has_mut_self {
        return Err(syn::Error::new(
            input.sig.span(),
            "call functions must take &mut self",
        ));
    }

    // Calculate selector (first 4 bytes of keccak256 of function signature)
    let selector = args
        .selector
        .unwrap_or_else(|| calculate_selector(&fn_name_str));

    // Extract parameter types for ABI
    let params = extract_params(inputs)?;
    let param_decodes = generate_param_decodes(&params)?;
    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();

    // Generate guards
    let payable_check = if !args.payable {
        quote! {
            if ::infrix_sdk::env::value() != ::infrix_sdk::infrix_types::U256::ZERO {
                return Err(::infrix_sdk::infrix_types::Error::NotPayable);
            }
        }
    } else {
        quote! {}
    };

    let owner_check = if args.only_owner {
        quote! {
            if ::infrix_sdk::env::caller() != ::infrix_sdk::env::owner() {
                return Err(::infrix_sdk::infrix_types::Error::Unauthorized);
            }
        }
    } else {
        quote! {}
    };

    let wrapper_name = format_ident!("__call_{}", fn_name);
    let selector_const = format_ident!("{}_SELECTOR", fn_name.to_string().to_uppercase());

    Ok(quote! {
        #(#attrs)*
        #visibility fn #fn_name(#inputs) #output #block

        /// Function selector
        pub const #selector_const: u32 = #selector;

        /// WASM entry point for call
        #[doc(hidden)]
        pub fn #wrapper_name(&mut self, input: &[u8]) -> Result<::infrix_sdk::infrix_types::CallResult, ::infrix_sdk::infrix_types::Error> {
            #payable_check
            #owner_check

            let mut offset = 0;
            #param_decodes

            let result = self.#fn_name(#(#param_names),*)?;

            // Encode result
            let mut buffer = [0u8; 4096];
            let len = ::infrix_sdk::infrix_types::Encode::encode(&result, &mut buffer)?;

            // Save state
            ::infrix_sdk::infrix_types::ContractInstance::save(self)?;

            Ok(::infrix_sdk::infrix_types::CallResult {
                data: buffer,
                data_len: len,
            })
        }
    })
}

/// Marks a function as view-only (read-only).
///
/// View functions cannot modify state and don't require a transaction.
/// They can be called for free via queries.
///
/// # Example
///
/// ```ignore
/// #[view]
/// pub fn balance_of(&self, owner: Address) -> U256 {
///     self.balances.get(&owner).unwrap_or(U256::ZERO)
/// }
/// ```
#[proc_macro_attribute]
pub fn view(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ImplItemFn);

    match generate_view(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_view(input: ImplItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let visibility = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let inputs = &input.sig.inputs;
    let output = &input.sig.output;

    // Check for &self (not &mut self)
    let has_self = inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));
    let has_mut_self = inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(r) if r.mutability.is_some()));

    if has_mut_self {
        return Err(syn::Error::new(
            input.sig.span(),
            "view functions must take &self, not &mut self",
        ));
    }

    if !has_self {
        return Err(syn::Error::new(
            input.sig.span(),
            "view functions must take &self",
        ));
    }

    // Calculate selector
    let selector = calculate_selector(&fn_name_str);

    // Extract parameter types for ABI
    let params = extract_params(inputs)?;
    let param_decodes = generate_param_decodes(&params)?;
    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();

    let wrapper_name = format_ident!("__view_{}", fn_name);
    let selector_const = format_ident!("{}_SELECTOR", fn_name.to_string().to_uppercase());

    Ok(quote! {
        #(#attrs)*
        #visibility fn #fn_name(#inputs) #output #block

        /// Function selector
        pub const #selector_const: u32 = #selector;

        /// WASM entry point for view
        #[doc(hidden)]
        pub fn #wrapper_name(&self, input: &[u8]) -> Result<::infrix_sdk::infrix_types::CallResult, ::infrix_sdk::infrix_types::Error> {
            let mut offset = 0;
            #param_decodes

            let raw_result = self.#fn_name(#(#param_names),*);
            let result = ::infrix_sdk::infrix_types::IntoResult::into_result(raw_result)?;

            // Encode result
            let mut buffer = [0u8; 4096];
            let len = ::infrix_sdk::infrix_types::Encode::encode(&result, &mut buffer)?;

            Ok(::infrix_sdk::infrix_types::CallResult {
                data: buffer,
                data_len: len,
            })
        }
    })
}

/// Generates an event struct with emission helpers.
///
/// Events are indexed and stored in the contract's event log.
/// They can be queried via the API.
///
/// # Attributes
///
/// - `#[indexed]` on fields - Mark field as indexed for filtering
///
/// # Example
///
/// ```ignore
/// #[event]
/// pub struct Transfer {
///     #[indexed]
///     from: Address,
///     #[indexed]
///     to: Address,
///     amount: U256,
/// }
/// ```
#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    match generate_event(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_event(input: ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    let visibility = &input.vis;

    // Calculate event signature hash
    let event_signature = calculate_selector(&struct_name_str);

    // Find indexed fields (and record every field, in declaration
    // order, for the event schema JSON).
    let mut indexed_fields = Vec::new();
    let mut data_fields = Vec::new();
    let mut schema_fields: Vec<(String, String, bool)> = Vec::new();

    if let syn::Fields::Named(fields) = &input.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;

            let is_indexed = field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("indexed"));

            schema_fields.push((
                field_name.to_string(),
                rust_type_to_schema(field_type),
                is_indexed,
            ));

            if is_indexed {
                indexed_fields.push((field_name.clone(), field_type.clone()));
            } else {
                data_fields.push((field_name.clone(), field_type.clone()));
            }
        }
    }

    // MARKER-AUDIT 2026-06-10 closure: pre-audit, #[event] structs were
    // silently absent from the embedded contract schema ("We pass empty
    // for Phase 2"). A proc macro cannot see sibling items, so each
    // event embeds its own schema object into the shared
    // `infrix:events` WASM custom section (the linker concatenates
    // same-named sections → newline-delimited JSON). Schema consumers
    // (cmd/infrix abi extraction) merge these into the contract ABI's
    // events list alongside the `infrix:schema` section.
    let event_schema_json = build_event_schema_json(&struct_name_str, &schema_fields);
    let mut event_schema_line = event_schema_json.clone();
    event_schema_line.push('\n');
    let event_schema_bytes = event_schema_line.as_bytes();
    let event_schema_len = event_schema_bytes.len();
    let event_section_static = format_ident!(
        "__INFRIX_EVENT_SCHEMA_{}",
        struct_name_str.to_uppercase()
    );

    // Generate topic encoding
    let topic_count = indexed_fields.len() + 1; // +1 for event signature
    let mut topic_encodings = vec![quote! {
        topics[0] = ::infrix_sdk::infrix_types::Topic::from_u32(#event_signature);
    }];

    for (i, (field_name, _)) in indexed_fields.iter().enumerate() {
        let topic_idx = i + 1;
        topic_encodings.push(quote! {
            topics[#topic_idx] = ::infrix_sdk::infrix_types::Topic::from_hash(
                ::infrix_sdk::infrix_types::Hash::hash_value(&self.#field_name)
            );
        });
    }

    // Generate data encoding
    let data_encodings: Vec<_> = data_fields.iter().map(|(field_name, _)| {
        quote! {
            offset += ::infrix_sdk::infrix_types::Encode::encode(&self.#field_name, &mut data[offset..])?;
        }
    }).collect();

    // Filter out indexed attribute from fields
    let filtered_fields: Vec<_> = if let syn::Fields::Named(fields) = &input.fields {
        fields
            .named
            .iter()
            .map(|f| {
                let mut f = f.clone();
                f.attrs.retain(|attr| !attr.path().is_ident("indexed"));
                f
            })
            .collect()
    } else {
        vec![]
    };

    Ok(quote! {
        #visibility struct #struct_name {
            #(#filtered_fields),*
        }

        #[cfg(target_arch = "wasm32")]
        #[link_section = "infrix:events"]
        #[used]
        #[doc(hidden)]
        #visibility static #event_section_static: [u8; #event_schema_len] = [#(#event_schema_bytes),*];

        impl #struct_name {
            /// Event signature hash
            pub const SIGNATURE: u32 = #event_signature;

            /// Number of indexed topics
            pub const TOPIC_COUNT: usize = #topic_count;

            /// Canonical event schema JSON — the same object shape as
            /// the `events` entries of the `infrix:schema` section,
            /// embedded into the `infrix:events` WASM custom section.
            pub const EVENT_SCHEMA_JSON: &'static str = #event_schema_json;

            /// Emit this event
            pub fn emit(&self) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                let mut topics = [::infrix_sdk::infrix_types::Topic::EMPTY; 4];
                #(#topic_encodings)*

                let mut data = [0u8; 1024];
                let mut offset = 0;
                #(#data_encodings)*

                ::infrix_sdk::events::emit(&topics[..#topic_count], &data[..offset]);
                Ok(())
            }
        }

        impl ::infrix_sdk::infrix_types::EventTrait for #struct_name {
            fn signature() -> u32 {
                Self::SIGNATURE
            }

            fn emit(&self) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                self.emit()
            }
        }
    })
}

/// Generates storage mapping helpers.
///
/// The annotated alias must have the exact shape
/// `type Name = StorageMap<K, V>;` — the declared key and value types
/// are bound into the generated accessors (`get(&K) -> Option<V>`,
/// `set(&K, &V)`, `remove(&K)`, `contains(&K)`), and any other shape
/// is a compile error. The alias target is a declaration-only marker;
/// the macro replaces it with a zero-sized struct carrying the typed
/// accessors.
///
/// # Example
///
/// ```ignore
/// #[storage_map(name = "balances")]
/// type Balances = StorageMap<Address, U256>;
///
/// Balances::set(&owner, &amount)?;
/// let balance: Option<U256> = Balances::get(&owner);
/// ```
#[proc_macro_attribute]
pub fn storage_map(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as StorageMapArgs);
    let input = parse_macro_input!(item as syn::ItemType);

    match generate_storage_map(args, input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct StorageMapArgs {
    name: String,
}

impl Parse for StorageMapArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = String::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "name" {
                input.parse::<Token![=]>()?;
                let lit: Lit = input.parse()?;
                if let Lit::Str(s) = lit {
                    name = s.value();
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if name.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "storage_map requires 'name' attribute",
            ));
        }

        Ok(StorageMapArgs { name })
    }
}

// extract_storage_map_kv pulls the declared key and value types out of
// a `type Name = StorageMap<K, V>;` alias. MARKER-AUDIT 2026-06-10
// closure: pre-audit the macro silently ignored the declared generics
// and generated an untyped `impl Encode`/`impl Decode` wrapper — a
// caller could pass any key type and the value type was uninferable.
// A declaration that is not exactly `StorageMap<K, V>` is now a
// compile error rather than a silently weaker wrapper.
fn extract_storage_map_kv(input: &syn::ItemType) -> syn::Result<(Type, Type)> {
    let err = || {
        syn::Error::new_spanned(
            &input.ty,
            "storage_map: the aliased type must be StorageMap<K, V> \
             (e.g. `type Balances = StorageMap<Address, U256>;`)",
        )
    };
    let Type::Path(type_path) = input.ty.as_ref() else {
        return Err(err());
    };
    let Some(last) = type_path.path.segments.last() else {
        return Err(err());
    };
    if last.ident != "StorageMap" {
        return Err(err());
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(err());
    };
    let mut types = args.args.iter().filter_map(|a| match a {
        syn::GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    let (Some(key_ty), Some(value_ty), None) = (types.next(), types.next(), types.next()) else {
        return Err(err());
    };
    Ok((key_ty, value_ty))
}

fn generate_storage_map(args: StorageMapArgs, input: syn::ItemType) -> syn::Result<TokenStream2> {
    let type_name = &input.ident;
    let visibility = &input.vis;
    let storage_prefix = args.name;

    // Bind the declared StorageMap<K, V> generics (fail-loud on any
    // other shape) so the generated accessors are typed end-to-end.
    let (key_ty, value_ty) = extract_storage_map_kv(&input)?;

    // The alias target `StorageMap<K, V>` is a declaration-only marker
    // (no such generic runtime type exists); emit a zero-sized struct
    // carrying the typed accessors instead of re-emitting the alias.
    Ok(quote! {
        #visibility struct #type_name;

        impl #type_name {
            /// Storage prefix for this map
            pub const PREFIX: &'static str = #storage_prefix;

            fn __build_key(key: &#key_ty, storage_key: &mut [u8; 512]) -> Result<usize, ::infrix_sdk::infrix_types::Error> {
                let mut key_buf = [0u8; 256];
                let key_len = ::infrix_sdk::infrix_types::Encode::encode(key, &mut key_buf)?;
                let prefix_bytes = Self::PREFIX.as_bytes();
                storage_key[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                storage_key[prefix_bytes.len()..prefix_bytes.len() + key_len]
                    .copy_from_slice(&key_buf[..key_len]);
                Ok(prefix_bytes.len() + key_len)
            }

            /// Get a value from the map
            pub fn get(key: &#key_ty) -> Option<#value_ty> {
                let mut storage_key = [0u8; 512];
                let total_len = Self::__build_key(key, &mut storage_key).ok()?;
                ::infrix_sdk::storage::get_decoded::<#value_ty>(&storage_key[..total_len])
            }

            /// Set a value in the map
            pub fn set(key: &#key_ty, value: &#value_ty) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                let mut storage_key = [0u8; 512];
                let total_len = Self::__build_key(key, &mut storage_key)?;
                ::infrix_sdk::storage::set_encoded(&storage_key[..total_len], value)
            }

            /// Remove a value from the map
            pub fn remove(key: &#key_ty) -> Result<(), ::infrix_sdk::infrix_types::Error> {
                let mut storage_key = [0u8; 512];
                let total_len = Self::__build_key(key, &mut storage_key)?;
                ::infrix_sdk::storage::delete(&storage_key[..total_len]);
                Ok(())
            }

            /// Check whether a key exists in the map
            pub fn contains(key: &#key_ty) -> bool {
                let mut storage_key = [0u8; 512];
                match Self::__build_key(key, &mut storage_key) {
                    Ok(total_len) => ::infrix_sdk::storage::has(&storage_key[..total_len]),
                    Err(_) => false,
                }
            }
        }
    })
}

/// Generates the WASM entry points and dispatcher for a contract implementation.
///
/// This should be applied to the impl block containing all contract methods.
///
/// # Example
///
/// ```ignore
/// #[contract_impl]
/// impl MyContract {
///     #[init]
///     pub fn new() -> Self { ... }
///
///     #[call]
///     pub fn do_something(&mut self) { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn contract_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    match generate_contract_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_contract_impl(input: ItemImpl) -> syn::Result<TokenStream2> {
    let self_ty = &input.self_ty;

    // Collect function metadata
    let mut init_fn: Option<Ident> = None;
    let mut call_fns: Vec<(Ident, u32)> = Vec::new();
    let mut view_fns: Vec<(Ident, u32)> = Vec::new();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = &method.sig.ident;

            for attr in &method.attrs {
                if attr.path().is_ident("init") {
                    init_fn = Some(fn_name.clone());
                } else if attr.path().is_ident("call") {
                    let selector = calculate_selector(&fn_name.to_string());
                    call_fns.push((fn_name.clone(), selector));
                } else if attr.path().is_ident("view") {
                    let selector = calculate_selector(&fn_name.to_string());
                    view_fns.push((fn_name.clone(), selector));
                }
            }
        }
    }

    // Generate dispatcher
    let call_arms: Vec<_> = call_fns.iter().map(|(name, selector)| {
        let wrapper_name = format_ident!("__call_{}", name);
        quote! {
            #selector => {
                let mut contract = match <#self_ty as ::infrix_sdk::infrix_types::ContractInstance>::load() {
                    Some(c) => c,
                    None => return Err(::infrix_sdk::infrix_types::Error::ContractNotInitialized),
                };
                contract.#wrapper_name(&input[4..])
            }
        }
    }).collect();

    let view_arms: Vec<_> = view_fns.iter().map(|(name, selector)| {
        let wrapper_name = format_ident!("__view_{}", name);
        quote! {
            #selector => {
                let contract = match <#self_ty as ::infrix_sdk::infrix_types::ContractInstance>::load() {
                    Some(c) => c,
                    None => return Err(::infrix_sdk::infrix_types::Error::ContractNotInitialized),
                };
                contract.#wrapper_name(&input[4..])
            }
        }
    }).collect();

    let init_dispatch = if init_fn.is_some() {
        quote! {
            if selector == 0 {
                return <#self_ty>::__init_wrapper(&input[4..]).map(|_| ::infrix_sdk::infrix_types::CallResult::empty());
            }
        }
    } else {
        quote! {}
    };

    // Generate ABI
    let call_abi: Vec<_> = call_fns
        .iter()
        .map(|(name, selector)| {
            let name_str = name.to_string();
            quote! {
                ::infrix_sdk::infrix_types::FunctionAbi {
                    name: #name_str,
                    selector: #selector,
                    mutability: ::infrix_sdk::infrix_types::Mutability::Mutable,
                }
            }
        })
        .collect();

    let view_abi: Vec<_> = view_fns
        .iter()
        .map(|(name, selector)| {
            let name_str = name.to_string();
            quote! {
                ::infrix_sdk::infrix_types::FunctionAbi {
                    name: #name_str,
                    selector: #selector,
                    mutability: ::infrix_sdk::infrix_types::Mutability::View,
                }
            }
        })
        .collect();

    // Generate schema section.
    let schema_section = generate_schema_section(self_ty, &input);

    Ok(quote! {
        #input

        #schema_section

        impl #self_ty {
            /// Dispatch a contract call based on selector
            pub fn __dispatch(input: &[u8]) -> Result<::infrix_sdk::infrix_types::CallResult, ::infrix_sdk::infrix_types::Error> {
                if input.len() < 4 {
                    return Err(::infrix_sdk::infrix_types::Error::InvalidInput);
                }

                let selector = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);

                #init_dispatch

                match selector {
                    #(#call_arms)*
                    #(#view_arms)*
                    _ => Err(::infrix_sdk::infrix_types::Error::UnknownFunction),
                }
            }

            /// Get the contract ABI
            pub fn __abi() -> &'static [::infrix_sdk::infrix_types::FunctionAbi] {
                const ABI: &[::infrix_sdk::infrix_types::FunctionAbi] = &[
                    #(#call_abi,)*
                    #(#view_abi,)*
                ];
                ABI
            }
        }

        /// WASM entry point - called by the runtime
        #[no_mangle]
        pub extern "C" fn __infrix_call(input_ptr: *const u8, input_len: usize, output_ptr: *mut u8) -> i32 {
            // Safety: The runtime guarantees valid pointers
            let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

            match <#self_ty>::__dispatch(input) {
                Ok(result) => {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            result.data.as_ptr(),
                            output_ptr,
                            result.data_len
                        );
                    }
                    result.data_len as i32
                }
                Err(e) => -(e as i32)
            }
        }

        /// WASM entry point - get contract ABI
        #[no_mangle]
        pub extern "C" fn __infrix_abi(output_ptr: *mut u8) -> i32 {
            let abi = <#self_ty>::__abi();
            let mut offset = 0;

            for func in abi {
                unsafe {
                    // Write function entry
                    let name_bytes = func.name.as_bytes();
                    *output_ptr.add(offset) = name_bytes.len() as u8;
                    offset += 1;
                    core::ptr::copy_nonoverlapping(
                        name_bytes.as_ptr(),
                        output_ptr.add(offset),
                        name_bytes.len()
                    );
                    offset += name_bytes.len();

                    // Write selector
                    let selector_bytes = func.selector.to_be_bytes();
                    core::ptr::copy_nonoverlapping(
                        selector_bytes.as_ptr(),
                        output_ptr.add(offset),
                        4
                    );
                    offset += 4;

                    // Write mutability
                    *output_ptr.add(offset) = func.mutability as u8;
                    offset += 1;
                }
            }

            offset as i32
        }
    })
}

// Helper functions

fn extract_params(inputs: &Punctuated<FnArg, Token![,]>) -> syn::Result<Vec<(Ident, Type)>> {
    let mut params = Vec::new();

    for arg in inputs {
        match arg {
            FnArg::Receiver(_) => continue, // Skip self
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    params.push((pat_ident.ident.clone(), (*pat_type.ty).clone()));
                } else {
                    return Err(syn::Error::new(
                        pat_type.pat.span(),
                        "expected identifier pattern",
                    ));
                }
            }
        }
    }

    Ok(params)
}

fn generate_param_decodes(params: &[(Ident, Type)]) -> syn::Result<TokenStream2> {
    let decodes: Vec<_> = params.iter().map(|(name, ty)| {
        quote! {
            let (#name, consumed) = <#ty as ::infrix_sdk::infrix_types::Decode>::decode_with_len(&input[offset..])?;
            offset += consumed;
        }
    }).collect();

    Ok(quote! { #(#decodes)* })
}

fn calculate_selector(name: &str) -> u32 {
    // Simple hash function for selector calculation
    // In production, this would use keccak256
    let bytes = name.as_bytes();
    let mut hash: u32 = 0x811c9dc5; // FNV offset basis
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193); // FNV prime
    }
    hash
}

// ---- Schema Generation Helpers ----

/// Converts a Rust type to a schema type string.
fn rust_type_to_schema(ty: &Type) -> String {
    let s = quote!(#ty).to_string().replace(' ', "");
    match s.as_str() {
        "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "f32" => "f32".into(),
        "f64" => "f64".into(),
        "bool" => "bool".into(),
        "String" | "&str" => "string".into(),
        "Vec<u8>" | "&[u8]" => "bytes".into(),
        "Address" | "::infrix_sdk::infrix_types::Address" => "address".into(),
        "U256" | "::infrix_sdk::infrix_types::U256" => "u256".into(),
        "Hash" | "::infrix_sdk::infrix_types::Hash" | "[u8;32]" => "bytes32".into(),
        "[u8;20]" => "bytes20".into(),
        "u128" => "u128".into(),
        _ if s.starts_with("Option<") => {
            let inner = &s[7..s.len() - 1];
            format!("option<{}>", inner)
        }
        _ if s.starts_with("Vec<") => {
            let inner = &s[4..s.len() - 1];
            format!("array<{}>", inner)
        }
        _ => s,
    }
}

/// Extracts the return type from a function signature for schema purposes.
fn extract_return_schema(output: &ReturnType) -> Vec<(String, String)> {
    match output {
        ReturnType::Default => vec![],
        ReturnType::Type(_, ty) => {
            let type_str = quote!(#ty).to_string().replace(' ', "");
            // Handle Result<T, Error> → extract T
            if type_str.starts_with("Result<") {
                let inner = &type_str[7..];
                if let Some(comma_pos) = inner.find(',') {
                    let ok_type = inner[..comma_pos].trim();
                    if ok_type == "()" {
                        return vec![];
                    }
                    return vec![("result".into(), rust_type_to_schema_str(ok_type))];
                }
            }
            if type_str == "()" {
                return vec![];
            }
            // Handle Self → skip (constructor)
            if type_str == "Self" {
                return vec![];
            }
            vec![("result".into(), rust_type_to_schema(ty))]
        }
    }
}

/// Converts a type string (not AST) to a schema type.
fn rust_type_to_schema_str(s: &str) -> String {
    match s.trim() {
        "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "bool" => "bool".into(),
        "String" | "&str" => "string".into(),
        "U256" => "u256".into(),
        "Address" => "address".into(),
        other => other.to_string(),
    }
}

/// Extracts doc comments from attributes (/// comments become #[doc = "..."])
fn extract_doc_comment(attrs: &[Attribute]) -> String {
    let mut docs = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(s) = &expr_lit.lit {
                        docs.push(s.value().trim().to_string());
                    }
                }
            }
        }
    }
    docs.join(" ")
}

/// Builds a JSON string representing the contract schema from collected metadata.
fn build_schema_json(
    contract_name: &str,
    functions: &[(
        String,
        String,
        Vec<(String, String)>,
        Vec<(String, String)>,
        String,
    )],
    // (name, mutability, params[(name,type)], returns[(name,type)], doc)
    events: &[(String, Vec<(String, String, bool)>)],
    // (name, fields[(name, type, indexed)])
) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"schema_version\": 1,\n"));
    json.push_str(&format!(
        "  \"name\": \"{}\",\n",
        escape_json(contract_name)
    ));
    json.push_str("  \"functions\": [\n");

    for (i, (name, mutability, params, returns, doc)) in functions.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"name\": \"{}\",\n", escape_json(name)));
        json.push_str(&format!(
            "      \"mutability\": \"{}\",\n",
            escape_json(mutability)
        ));

        json.push_str("      \"params\": [");
        for (j, (pname, ptype)) in params.iter().enumerate() {
            if j > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"name\":\"{}\",\"type\":\"{}\"}}",
                escape_json(pname),
                escape_json(ptype)
            ));
        }
        json.push_str("],\n");

        json.push_str("      \"returns\": [");
        for (j, (rname, rtype)) in returns.iter().enumerate() {
            if j > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"name\":\"{}\",\"type\":\"{}\"}}",
                escape_json(rname),
                escape_json(rtype)
            ));
        }
        json.push_str("]");

        if !doc.is_empty() {
            json.push_str(&format!(",\n      \"doc\": \"{}\"", escape_json(doc)));
        }

        json.push_str("\n    }");
        if i < functions.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]");

    if !events.is_empty() {
        json.push_str(",\n  \"events\": [\n");
        for (i, (name, fields)) in events.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"name\": \"{}\",\n", escape_json(name)));
            json.push_str("      \"fields\": [");
            for (j, (fname, ftype, indexed)) in fields.iter().enumerate() {
                if j > 0 {
                    json.push_str(", ");
                }
                json.push_str(&format!(
                    "{{\"name\":\"{}\",\"type\":\"{}\",\"indexed\":{}}}",
                    escape_json(fname),
                    escape_json(ftype),
                    indexed
                ));
            }
            json.push_str("]\n    }");
            if i < events.len() - 1 {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("  ]");
    }

    json.push_str("\n}");
    json
}

/// Builds a single-line JSON object describing one event — identical
/// in shape to the `events` entries of `build_schema_json`, so the
/// `infrix:events` custom section (one object per line) merges
/// directly into the contract ABI's events list.
fn build_event_schema_json(name: &str, fields: &[(String, String, bool)]) -> String {
    let mut json = String::new();
    json.push_str(&format!("{{\"name\":\"{}\",\"fields\":[", escape_json(name)));
    for (i, (fname, ftype, indexed)) in fields.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{}\",\"type\":\"{}\",\"indexed\":{}}}",
            escape_json(fname),
            escape_json(ftype),
            indexed
        ));
    }
    json.push_str("]}");
    json
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Collects schema metadata from a contract_impl block and generates the
/// schema JSON + link_section static.
fn generate_schema_section(self_ty: &Type, input: &ItemImpl) -> TokenStream2 {
    let contract_name = quote!(#self_ty).to_string();
    let mut functions = Vec::new();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();
            let doc = extract_doc_comment(&method.attrs);

            let mut mutability = None;
            for attr in &method.attrs {
                if attr.path().is_ident("init") {
                    mutability = Some("init".to_string());
                } else if attr.path().is_ident("call") {
                    mutability = Some("mutable".to_string());
                    // Check for payable
                    if let Ok(args) = attr.parse_args::<proc_macro2::TokenStream>() {
                        if args.to_string().contains("payable") {
                            mutability = Some("payable".to_string());
                        }
                    }
                } else if attr.path().is_ident("view") {
                    mutability = Some("view".to_string());
                }
            }

            if let Some(mut_str) = mutability {
                let params_raw = extract_params(&method.sig.inputs).unwrap_or_default();
                let params: Vec<(String, String)> = params_raw
                    .iter()
                    .map(|(name, ty)| (name.to_string(), rust_type_to_schema(ty)))
                    .collect();

                let returns = extract_return_schema(&method.sig.output);

                functions.push((fn_name, mut_str, params, returns, doc));
            }
        }
    }

    // Events are declared as separate #[event] structs, and a proc
    // macro cannot see sibling items — so events are NOT embedded here.
    // MARKER-AUDIT 2026-06-10 closure: each #[event] struct embeds its
    // own schema object into the `infrix:events` WASM custom section
    // (see generate_event); schema consumers merge that section's
    // newline-delimited objects into this schema's events list. The
    // `events` key is therefore intentionally absent from the
    // `infrix:schema` payload emitted by this macro.
    let events: Vec<(String, Vec<(String, String, bool)>)> = Vec::new();

    let schema_json = build_schema_json(&contract_name, &functions, &events);
    let schema_bytes = schema_json.as_bytes();
    let schema_len = schema_bytes.len();

    // Generate a static that embeds the schema in a WASM custom section.
    quote! {
        #[cfg(target_arch = "wasm32")]
        #[link_section = "infrix:schema"]
        #[used]
        static __INFRIX_SCHEMA: [u8; #schema_len] = [#(#schema_bytes),*];

        /// Returns the embedded contract schema as a JSON string.
        impl #self_ty {
            pub fn __schema_json() -> &'static str {
                const SCHEMA: &str = #schema_json;
                SCHEMA
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_selector() {
        let selector1 = calculate_selector("transfer");
        let selector2 = calculate_selector("balance_of");

        assert_ne!(selector1, selector2);

        // Same name should give same selector
        assert_eq!(selector1, calculate_selector("transfer"));
    }

    #[test]
    fn test_contract_args_parse() {
        // Test parsing would require setting up proc_macro2 test infrastructure
    }

    #[test]
    fn test_rust_type_to_schema() {
        let cases = vec![
            ("i32", "i32"),
            ("i64", "i64"),
            ("u32", "u32"),
            ("u64", "u64"),
            ("bool", "bool"),
            ("String", "string"),
            ("u128", "u128"),
        ];
        for (input, expected) in cases {
            let ty: Type = syn::parse_str(input).unwrap();
            assert_eq!(rust_type_to_schema(&ty), expected, "for type {}", input);
        }
    }

    #[test]
    fn test_build_schema_json() {
        let functions = vec![
            (
                "increment".into(),
                "mutable".into(),
                vec![],
                vec![("count".into(), "i32".into())],
                "Increments the counter.".into(),
            ),
            (
                "set".into(),
                "mutable".into(),
                vec![("value".into(), "i32".into())],
                vec![],
                String::new(),
            ),
        ];
        let events: Vec<(String, Vec<(String, String, bool)>)> = vec![];
        let json = build_schema_json("Counter", &functions, &events);

        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"name\": \"Counter\""));
        assert!(json.contains("\"name\": \"increment\""));
        assert!(json.contains("\"mutability\": \"mutable\""));
        assert!(json.contains("\"doc\": \"Increments the counter.\""));
        assert!(json.contains("\"name\": \"set\""));
        assert!(json.contains("\"type\":\"i32\""));
    }

    #[test]
    fn test_build_schema_json_with_events() {
        let functions = vec![];
        let events = vec![(
            "Transfer".into(),
            vec![
                ("from".into(), "address".into(), true),
                ("to".into(), "address".into(), true),
                ("amount".into(), "u256".into(), false),
            ],
        )];
        let json = build_schema_json("Token", &functions, &events);

        assert!(json.contains("\"events\""));
        assert!(json.contains("\"name\": \"Transfer\""));
        assert!(json.contains("\"indexed\":true"));
        assert!(json.contains("\"indexed\":false"));
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    // MARKER-AUDIT 2026-06-10 closure: per-event schema objects must be
    // valid single-line JSON in the same shape as the schema's events
    // entries, so the infrix:events section merges into the ABI.
    #[test]
    fn test_build_event_schema_json() {
        let fields = vec![
            ("from".to_string(), "address".to_string(), true),
            ("to".to_string(), "address".to_string(), true),
            ("amount".to_string(), "u256".to_string(), false),
        ];
        let json = build_event_schema_json("Transfer", &fields);
        assert_eq!(
            json,
            "{\"name\":\"Transfer\",\"fields\":[\
             {\"name\":\"from\",\"type\":\"address\",\"indexed\":true},\
             {\"name\":\"to\",\"type\":\"address\",\"indexed\":true},\
             {\"name\":\"amount\",\"type\":\"u256\",\"indexed\":false}]}"
        );
        assert!(!json.contains('\n'), "must be single-line for NDJSON merging");
    }

    // MARKER-AUDIT 2026-06-10 closure: the declared StorageMap<K, V>
    // generics must be extracted (typed accessors), and any other alias
    // shape must be a compile error rather than an untyped wrapper.
    #[test]
    fn test_extract_storage_map_kv() {
        let item: syn::ItemType =
            syn::parse_str("type Balances = StorageMap<Address, U256>;").unwrap();
        let (k, v) = extract_storage_map_kv(&item).unwrap();
        assert_eq!(quote!(#k).to_string(), "Address");
        assert_eq!(quote!(#v).to_string(), "U256");

        // Fully-qualified path also works (last segment matters).
        let item: syn::ItemType =
            syn::parse_str("type B = infrix_sdk::StorageMap<u64, String>;").unwrap();
        let (k, v) = extract_storage_map_kv(&item).unwrap();
        assert_eq!(quote!(#k).to_string(), "u64");
        assert_eq!(quote!(#v).to_string(), "String");

        // Wrong shapes fail loud.
        for bad in [
            "type B = StorageMap;",
            "type B = StorageMap<u64>;",
            "type B = StorageMap<u64, String, bool>;",
            "type B = Vec<u8>;",
            "type B = (u64, String);",
        ] {
            let item: syn::ItemType = syn::parse_str(bad).unwrap();
            assert!(
                extract_storage_map_kv(&item).is_err(),
                "expected error for {bad}"
            );
        }
    }
}

// ---- Test Framework Macros ----

/// Marks a function as an Infrix contract test.
///
/// The test function receives a mutable reference to a `TestContext` and should
/// use assertions to verify contract behavior. When compiled to WASM, the
/// function is exported with its original name (which must start with `test_`)
/// so the `infrix test` CLI discovers and executes it.
///
/// In `cargo test` mode the function runs with a mock `TestContext`.
///
/// # Example
///
/// ```ignore
/// use ::infrix_sdk::testing::*;
///
/// #[infrix_test]
/// fn test_increment(ctx: &mut TestContext) {
///     let counter = ctx.deploy("acc://test/counter");
///     let receipt = ctx.call(&counter, "increment", &[]);
///     assert!(receipt.is_success());
/// }
/// ```
#[proc_macro_attribute]
pub fn infrix_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    match generate_infrix_test(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_infrix_test(input: ItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_body = &input.block;
    let fn_vis = &input.vis;

    // Validate the function name starts with test_.
    let name_str = fn_name.to_string();
    if !name_str.starts_with("test_") {
        return Err(syn::Error::new(
            fn_name.span(),
            "#[infrix_test] functions must be named test_*",
        ));
    }

    // Generate a WASM-exported wrapper that:
    // 1. Creates a TestContext
    // 2. Calls the user's test function
    // 3. Returns 0 on success (panic = trap = failure in WASM)
    //
    // Also generate a #[test] function for `cargo test`.
    let wasm_export_name = format_ident!("{}", name_str);
    let cargo_test_name = format_ident!("cargo_{}", name_str);

    Ok(quote! {
        // WASM export: discovered by `infrix test` CLI.
        #[no_mangle]
        #fn_vis extern "C" fn #wasm_export_name() -> i32 {
            let mut ctx = ::infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut ::infrix_sdk::testing::TestContext| #fn_body;
            inner(&mut ctx);
            0 // success
        }

        // `cargo test` runner.
        #[cfg(test)]
        #[test]
        fn #cargo_test_name() {
            let mut ctx = ::infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut ::infrix_sdk::testing::TestContext| #fn_body;
            inner(&mut ctx);
        }
    })
}

/// Marks a function as a fuzz test.
///
/// Fuzz test functions receive a `TestContext` and one or more random input
/// parameters. The `infrix test` CLI generates random inputs and calls the
/// function repeatedly.
///
/// # Example
///
/// ```ignore
/// #[infrix_fuzz(runs = 1000)]
/// fn fuzz_set_get(ctx: &mut TestContext, value: i32) {
///     let contract = ctx.deploy("acc://test/counter");
///     ctx.call(&contract, "set", &[value as i64]);
///     let result = ctx.query(&contract, "get_count", &[]);
///     assert_eq!(result.return_i32(), value);
/// }
/// ```
#[proc_macro_attribute]
pub fn infrix_fuzz(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _ = attr; // Fuzz runs count parsed at runtime by the CLI.

    match generate_infrix_fuzz(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_infrix_fuzz(input: ItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_body = &input.block;
    let fn_vis = &input.vis;

    let name_str = fn_name.to_string();
    if !name_str.starts_with("fuzz_") {
        return Err(syn::Error::new(
            fn_name.span(),
            "#[infrix_fuzz] functions must be named fuzz_*",
        ));
    }

    // For WASM export, the fuzz function takes an i32 parameter that the
    // Go runner supplies with random values.
    let wasm_export_name = format_ident!("{}", name_str);
    let cargo_test_name = format_ident!("cargo_{}", name_str);

    Ok(quote! {
        // WASM export: called by `infrix test` CLI with random values.
        #[no_mangle]
        #fn_vis extern "C" fn #wasm_export_name(fuzz_input: i32) -> i32 {
            let mut ctx = ::infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut ::infrix_sdk::testing::TestContext, value: i32| #fn_body;
            inner(&mut ctx, fuzz_input);
            0 // success
        }

        // `cargo test` runner with a fixed input.
        #[cfg(test)]
        #[test]
        fn #cargo_test_name() {
            let mut ctx = ::infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut ::infrix_sdk::testing::TestContext, value: i32| #fn_body;
            // Test with a few representative values.
            for v in [0, 1, -1, 42, i32::MAX, i32::MIN] {
                inner(&mut ctx, v);
            }
        }
    })
}

// =============================================================================
// Shape-Shifting Contract Macros
// =============================================================================

mod shapes;

/// Declares shape definitions for a shape-shifting contract.
///
/// Applied to an enum where each variant is a shape with `#[infrix::shape(...)]`.
/// Generates the `infrix:shapes` WASM custom section and type-safe parameter
/// accessor functions.
///
/// # Example
///
/// ```ignore
/// #[infrix::shapes(default = "conservative")]
/// pub enum Shape {
///     #[infrix::shape(params(ltv_ratio: u64 = 5000))]
///     Conservative,
/// }
/// ```
#[proc_macro_attribute]
pub fn shapes(attr: TokenStream, item: TokenStream) -> TokenStream {
    shapes::shapes_impl(attr, item)
}

/// Declares evolution rules for shape transitions.
///
/// Applied to a module containing `#[infrix::rule(...)]` functions.
#[proc_macro_attribute]
pub fn evolution_rules(attr: TokenStream, item: TokenStream) -> TokenStream {
    shapes::evolution_rules_impl(attr, item)
}

/// Declares a single shape within a `#[infrix::shapes]` enum.
#[proc_macro_attribute]
pub fn shape(attr: TokenStream, item: TokenStream) -> TokenStream {
    shapes::shape_impl(attr, item)
}

/// Declares a single evolution rule within a rules module.
#[proc_macro_attribute]
pub fn rule(attr: TokenStream, item: TokenStream) -> TokenStream {
    shapes::rule_impl(attr, item)
}

// =============================================================================
// Governance Macros
// =============================================================================

/// Require the caller to have a specific role.
///
/// Reverts with `Error::RoleRequired` if the check fails.
///
/// # Example
/// ```ignore
/// #[require_role("admin")]
/// pub fn admin_only(&self) -> Result<(), Error> { Ok(()) }
/// ```
#[proc_macro_attribute]
pub fn require_role(attr: TokenStream, item: TokenStream) -> TokenStream {
    let role = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);
    governance_macros::generate_require_role(&role.value(), &input).into()
}

/// Require the caller to have a specific capability.
///
/// Reverts with `Error::CapabilityDenied` if the check fails.
///
/// # Example
/// ```ignore
/// #[require_capability("token:transfer")]
/// pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> { Ok(()) }
/// ```
#[proc_macro_attribute]
pub fn require_capability(attr: TokenStream, item: TokenStream) -> TokenStream {
    let cap = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);
    governance_macros::generate_require_capability(&cap.value(), &input).into()
}

/// Require multi-party approval before execution.
///
/// # Attributes
/// - `threshold = N` (required): number of approvals needed
/// - `role = "..."` (optional): specific role required for approvers
///
/// # Example
/// ```ignore
/// #[require_approval(threshold = 2)]
/// pub fn withdraw(&mut self, amount: U256) -> Result<(), Error> { Ok(()) }
///
/// #[require_approval(threshold = 3, role = "board_member")]
/// pub fn dissolve(&mut self) -> Result<(), Error> { Ok(()) }
/// ```
#[proc_macro_attribute]
pub fn require_approval(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ApprovalArgs);
    let input = parse_macro_input!(item as ItemFn);
    governance_macros::generate_require_approval(args.threshold, args.role.as_deref(), &input)
        .into()
}

/// Route function execution through the intent pipeline.
///
/// When called outside a governed execution context, the function
/// submits an intent instead of executing directly.
///
/// # Example
/// ```ignore
/// #[governed]
/// pub fn large_transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
///     // body becomes the execution target of an intent
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn governed(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    governance_macros::generate_governed(&input).into()
}

/// Automatically generate evidence links for function execution.
///
/// Wraps the function so that an evidence event is emitted after
/// execution completes (whether success or failure).
///
/// # Example
/// ```ignore
/// #[evidenced]
/// pub fn sensitive_operation(&mut self) -> Result<(), Error> { Ok(()) }
/// ```
#[proc_macro_attribute]
pub fn evidenced(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    governance_macros::generate_evidenced(&input).into()
}

/// Helper struct for parsing `#[require_approval(...)]` attributes.
struct ApprovalArgs {
    threshold: u32,
    role: Option<String>,
}

impl Parse for ApprovalArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut threshold = 1u32;
        let mut role = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "threshold" => {
                    let lit: LitInt = input.parse()?;
                    threshold = lit.base10_parse()?;
                }
                "role" => {
                    let lit: LitStr = input.parse()?;
                    role = Some(lit.value());
                }
                _ => return Err(syn::Error::new(ident.span(), "unknown attribute")),
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ApprovalArgs { threshold, role })
    }
}
