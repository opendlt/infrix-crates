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
//! use infrix_sdk::prelude::*;
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
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Block, Expr, FnArg, Ident, ImplItem, ImplItemFn, ItemFn, ItemImpl, ItemStruct,
    Lit, Meta, Pat, PatType, ReturnType, Signature, Token, Type, Visibility,
};

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
        impl infrix_types::ContractInstance for #struct_name {
            fn load() -> Option<Self> {
                let key = Self::STORAGE_KEY.as_bytes();
                let data = infrix_sdk::storage::get(key)?;
                Self::decode(&data).ok()
            }

            fn save(&self) -> Result<(), infrix_types::Error> {
                let key = Self::STORAGE_KEY.as_bytes();
                let mut buffer = [0u8; 4096];
                let len = self.encode(&mut buffer)?;
                infrix_sdk::storage::set(key, &buffer[..len]);
                Ok(())
            }

            fn delete() {
                let key = Self::STORAGE_KEY.as_bytes();
                infrix_sdk::storage::delete(key);
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
                    let data = infrix_sdk::storage::get(key)?;
                    <#field_type as infrix_types::Decode>::decode(&data).ok()
                }
            });

            // Generate setter
            let setter_name = format_ident!("set_{}", field_name);
            accessors.push(quote! {
                /// Set the value of #field_name in storage
                pub fn #setter_name(value: &#field_type) -> Result<(), infrix_types::Error> {
                    let key = #storage_key.as_bytes();
                    let mut buffer = [0u8; 1024];
                    let len = <#field_type as infrix_types::Encode>::encode(value, &mut buffer)?;
                    infrix_sdk::storage::set(key, &buffer[..len]);
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
        for (i, field) in fields.named.iter().enumerate() {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            field_names.push(field_name.clone());

            encode_fields.push(quote! {
                offset += <#field_type as infrix_types::Encode>::encode(&self.#field_name, &mut buffer[offset..])?;
            });

            decode_fields.push(quote! {
                let (#field_name, consumed) = <#field_type as infrix_types::Decode>::decode_with_len(&data[offset..])?;
                offset += consumed;
            });
        }
    }

    Ok(quote! {
        impl infrix_types::Encode for #struct_name {
            fn encode(&self, buffer: &mut [u8]) -> Result<usize, infrix_types::Error> {
                let mut offset = 0;
                #(#encode_fields)*
                Ok(offset)
            }
        }

        impl infrix_types::Decode for #struct_name {
            fn decode(data: &[u8]) -> Result<Self, infrix_types::Error> {
                let (result, _) = Self::decode_with_len(data)?;
                Ok(result)
            }

            fn decode_with_len(data: &[u8]) -> Result<(Self, usize), infrix_types::Error> {
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
        pub fn #wrapper_name(input: &[u8]) -> Result<Self, infrix_types::Error> {
            let mut offset = 0;
            #param_decodes
            let instance = Self::#fn_name(#(#param_names),*);

            // Handle Result return type
            let instance = match core::convert::Into::<Result<Self, infrix_types::Error>>::into(
                infrix_types::IntoResult::into_result(instance)
            ) {
                Ok(i) => i,
                Err(e) => return Err(e),
            };

            // Save the contract state
            instance.save()?;
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
    let has_mut_self = inputs.iter().any(|arg| {
        matches!(arg, FnArg::Receiver(r) if r.mutability.is_some())
    });

    if !has_mut_self {
        return Err(syn::Error::new(
            input.sig.span(),
            "call functions must take &mut self",
        ));
    }

    // Calculate selector (first 4 bytes of keccak256 of function signature)
    let selector = args.selector.unwrap_or_else(|| {
        calculate_selector(&fn_name_str)
    });

    // Extract parameter types for ABI
    let params = extract_params(inputs)?;
    let param_decodes = generate_param_decodes(&params)?;
    let param_names: Vec<_> = params.iter().map(|(name, _)| name.clone()).collect();

    // Generate guards
    let payable_check = if !args.payable {
        quote! {
            if infrix_sdk::env::value() != infrix_types::U256::ZERO {
                return Err(infrix_types::Error::NotPayable);
            }
        }
    } else {
        quote! {}
    };

    let owner_check = if args.only_owner {
        quote! {
            if infrix_sdk::env::caller() != infrix_sdk::env::owner() {
                return Err(infrix_types::Error::Unauthorized);
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
        pub fn #wrapper_name(&mut self, input: &[u8]) -> Result<infrix_types::CallResult, infrix_types::Error> {
            #payable_check
            #owner_check

            let mut offset = 0;
            #param_decodes

            let result = self.#fn_name(#(#param_names),*);

            // Handle Result return type
            let result = match infrix_types::IntoResult::into_result(result) {
                Ok(v) => v,
                Err(e) => return Err(e),
            };

            // Encode result
            let mut buffer = [0u8; 4096];
            let len = infrix_types::Encode::encode(&result, &mut buffer)?;

            // Save state
            self.save()?;

            Ok(infrix_types::CallResult {
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
    let has_mut_self = inputs.iter().any(|arg| {
        matches!(arg, FnArg::Receiver(r) if r.mutability.is_some())
    });

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
        pub fn #wrapper_name(&self, input: &[u8]) -> Result<infrix_types::CallResult, infrix_types::Error> {
            let mut offset = 0;
            #param_decodes

            let result = self.#fn_name(#(#param_names),*);

            // Handle Result return type
            let result = match infrix_types::IntoResult::into_result(result) {
                Ok(v) => v,
                Err(e) => return Err(e),
            };

            // Encode result
            let mut buffer = [0u8; 4096];
            let len = infrix_types::Encode::encode(&result, &mut buffer)?;

            Ok(infrix_types::CallResult {
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

    // Find indexed fields
    let mut indexed_fields = Vec::new();
    let mut data_fields = Vec::new();

    if let syn::Fields::Named(fields) = &input.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;

            let is_indexed = field.attrs.iter().any(|attr| {
                attr.path().is_ident("indexed")
            });

            if is_indexed {
                indexed_fields.push((field_name.clone(), field_type.clone()));
            } else {
                data_fields.push((field_name.clone(), field_type.clone()));
            }
        }
    }

    // Generate topic encoding
    let topic_count = indexed_fields.len() + 1; // +1 for event signature
    let mut topic_encodings = vec![quote! {
        topics[0] = infrix_types::Topic::from_u32(#event_signature);
    }];

    for (i, (field_name, _)) in indexed_fields.iter().enumerate() {
        let topic_idx = i + 1;
        topic_encodings.push(quote! {
            topics[#topic_idx] = infrix_types::Topic::from_hash(
                infrix_types::Hash::hash_value(&self.#field_name)
            );
        });
    }

    // Generate data encoding
    let data_encodings: Vec<_> = data_fields.iter().map(|(field_name, _)| {
        quote! {
            offset += infrix_types::Encode::encode(&self.#field_name, &mut data[offset..])?;
        }
    }).collect();

    // Filter out indexed attribute from fields
    let filtered_fields: Vec<_> = if let syn::Fields::Named(fields) = &input.fields {
        fields.named.iter().map(|f| {
            let mut f = f.clone();
            f.attrs.retain(|attr| !attr.path().is_ident("indexed"));
            f
        }).collect()
    } else {
        vec![]
    };

    Ok(quote! {
        #visibility struct #struct_name {
            #(#filtered_fields),*
        }

        impl #struct_name {
            /// Event signature hash
            pub const SIGNATURE: u32 = #event_signature;

            /// Number of indexed topics
            pub const TOPIC_COUNT: usize = #topic_count;

            /// Emit this event
            pub fn emit(&self) -> Result<(), infrix_types::Error> {
                let mut topics = [infrix_types::Topic::EMPTY; 4];
                #(#topic_encodings)*

                let mut data = [0u8; 1024];
                let mut offset = 0;
                #(#data_encodings)*

                infrix_sdk::events::emit(&topics[..#topic_count], &data[..offset]);
                Ok(())
            }
        }

        impl infrix_types::EventTrait for #struct_name {
            fn signature() -> u32 {
                Self::SIGNATURE
            }

            fn emit(&self) -> Result<(), infrix_types::Error> {
                self.emit()
            }
        }
    })
}

/// Generates storage mapping helpers.
///
/// This attribute generates getter/setter functions for mapping storage.
///
/// # Example
///
/// ```ignore
/// #[storage_map(name = "balances")]
/// type Balances = StorageMap<Address, U256>;
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

fn generate_storage_map(args: StorageMapArgs, input: syn::ItemType) -> syn::Result<TokenStream2> {
    let type_name = &input.ident;
    let storage_prefix = args.name;

    // Extract key and value types from StorageMap<K, V>
    // For now, generate a simple wrapper

    Ok(quote! {
        #input

        impl #type_name {
            /// Storage prefix for this map
            pub const PREFIX: &'static str = #storage_prefix;

            /// Get a value from the map
            pub fn get(key: &impl infrix_types::Encode) -> Option<impl infrix_types::Decode> {
                let mut key_buf = [0u8; 256];
                let key_len = infrix_types::Encode::encode(key, &mut key_buf).ok()?;

                let mut storage_key = [0u8; 512];
                let prefix_bytes = Self::PREFIX.as_bytes();
                storage_key[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                storage_key[prefix_bytes.len()..prefix_bytes.len() + key_len]
                    .copy_from_slice(&key_buf[..key_len]);

                let data = infrix_sdk::storage::get(&storage_key[..prefix_bytes.len() + key_len])?;
                infrix_types::Decode::decode(&data).ok()
            }

            /// Set a value in the map
            pub fn set(key: &impl infrix_types::Encode, value: &impl infrix_types::Encode) -> Result<(), infrix_types::Error> {
                let mut key_buf = [0u8; 256];
                let key_len = infrix_types::Encode::encode(key, &mut key_buf)?;

                let mut storage_key = [0u8; 512];
                let prefix_bytes = Self::PREFIX.as_bytes();
                storage_key[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                storage_key[prefix_bytes.len()..prefix_bytes.len() + key_len]
                    .copy_from_slice(&key_buf[..key_len]);

                let mut value_buf = [0u8; 4096];
                let value_len = infrix_types::Encode::encode(value, &mut value_buf)?;

                infrix_sdk::storage::set(
                    &storage_key[..prefix_bytes.len() + key_len],
                    &value_buf[..value_len]
                );
                Ok(())
            }

            /// Remove a value from the map
            pub fn remove(key: &impl infrix_types::Encode) -> Result<(), infrix_types::Error> {
                let mut key_buf = [0u8; 256];
                let key_len = infrix_types::Encode::encode(key, &mut key_buf)?;

                let mut storage_key = [0u8; 512];
                let prefix_bytes = Self::PREFIX.as_bytes();
                storage_key[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                storage_key[prefix_bytes.len()..prefix_bytes.len() + key_len]
                    .copy_from_slice(&key_buf[..key_len]);

                infrix_sdk::storage::delete(&storage_key[..prefix_bytes.len() + key_len]);
                Ok(())
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
                let mut contract = match <#self_ty as infrix_types::ContractInstance>::load() {
                    Some(c) => c,
                    None => return Err(infrix_types::Error::ContractNotInitialized),
                };
                contract.#wrapper_name(&input[4..])
            }
        }
    }).collect();

    let view_arms: Vec<_> = view_fns.iter().map(|(name, selector)| {
        let wrapper_name = format_ident!("__view_{}", name);
        quote! {
            #selector => {
                let contract = match <#self_ty as infrix_types::ContractInstance>::load() {
                    Some(c) => c,
                    None => return Err(infrix_types::Error::ContractNotInitialized),
                };
                contract.#wrapper_name(&input[4..])
            }
        }
    }).collect();

    let init_dispatch = if let Some(init_name) = init_fn {
        quote! {
            if selector == 0 {
                return <#self_ty>::__init_wrapper(&input[4..]).map(|_| infrix_types::CallResult::empty());
            }
        }
    } else {
        quote! {}
    };

    // Generate ABI
    let call_abi: Vec<_> = call_fns.iter().map(|(name, selector)| {
        let name_str = name.to_string();
        quote! {
            infrix_types::FunctionAbi {
                name: #name_str,
                selector: #selector,
                mutability: infrix_types::Mutability::Mutable,
            }
        }
    }).collect();

    let view_abi: Vec<_> = view_fns.iter().map(|(name, selector)| {
        let name_str = name.to_string();
        quote! {
            infrix_types::FunctionAbi {
                name: #name_str,
                selector: #selector,
                mutability: infrix_types::Mutability::View,
            }
        }
    }).collect();

    Ok(quote! {
        #input

        impl #self_ty {
            /// Dispatch a contract call based on selector
            pub fn __dispatch(input: &[u8]) -> Result<infrix_types::CallResult, infrix_types::Error> {
                if input.len() < 4 {
                    return Err(infrix_types::Error::InvalidInput);
                }

                let selector = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);

                #init_dispatch

                match selector {
                    #(#call_arms)*
                    #(#view_arms)*
                    _ => Err(infrix_types::Error::UnknownFunction),
                }
            }

            /// Get the contract ABI
            pub fn __abi() -> &'static [infrix_types::FunctionAbi] {
                const ABI: &[infrix_types::FunctionAbi] = &[
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
            let (#name, consumed) = <#ty as infrix_types::Decode>::decode_with_len(&input[offset..])?;
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
/// use infrix_sdk::testing::*;
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
            let mut ctx = infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut infrix_sdk::testing::TestContext| #fn_body;
            inner(&mut ctx);
            0 // success
        }

        // `cargo test` runner.
        #[cfg(test)]
        #[test]
        fn #cargo_test_name() {
            let mut ctx = infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut infrix_sdk::testing::TestContext| #fn_body;
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
            let mut ctx = infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut infrix_sdk::testing::TestContext, value: i32| #fn_body;
            inner(&mut ctx, fuzz_input);
            0 // success
        }

        // `cargo test` runner with a fixed input.
        #[cfg(test)]
        #[test]
        fn #cargo_test_name() {
            let mut ctx = infrix_sdk::testing::TestContext::new();
            let inner = |ctx: &mut infrix_sdk::testing::TestContext, value: i32| #fn_body;
            // Test with a few representative values.
            for v in [0, 1, -1, 42, i32::MAX, i32::MIN] {
                inner(&mut ctx, v);
            }
        }
    })
}
