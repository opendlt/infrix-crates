//! Governance procedural macros for Infrix smart contracts.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

/// Generates the `#[require_role("role_name")]` attribute macro expansion.
///
/// This macro wraps the function body in a role check. If the caller
/// does not have the specified role, the function reverts.
///
/// # Example
///
/// ```ignore
/// #[require_role("admin")]
/// pub fn set_config(&mut self, key: String, value: String) -> Result<(), Error> {
///     // Only reachable if caller has "admin" role
///     self.config.insert(key, value);
///     Ok(())
/// }
/// ```
///
/// # Expansion
///
/// ```ignore
/// pub fn set_config(&mut self, key: String, value: String) -> Result<(), Error> {
///     let __caller = infrix_sdk::env::caller();
///     if !infrix_sdk::governance::has_role(&__caller.to_string(), "admin") {
///         return Err(infrix_types::Error::RoleRequired);
///     }
///     // original body
///     self.config.insert(key, value);
///     Ok(())
/// }
/// ```
pub fn generate_require_role(role: &str, input: &ItemFn) -> TokenStream {
    let vis = &input.vis;
    let sig = &input.sig;
    let body = &input.block;
    let attrs: Vec<_> = input.attrs.iter().collect();

    quote! {
        #(#attrs)*
        #vis #sig {
            let __caller = infrix_sdk::env::caller();
            let __caller_str = __caller.to_string().unwrap_or_default();
            if !infrix_sdk::governance::has_role(&__caller_str, #role) {
                return Err(infrix_types::Error::RoleRequired);
            }
            #body
        }
    }
}

/// Generates the `#[require_capability("cap_name")]` attribute macro expansion.
///
/// # Example
///
/// ```ignore
/// #[require_capability("token:transfer")]
/// pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
///     // Only reachable if caller has "token:transfer" capability
///     // ...
/// }
/// ```
///
/// # Expansion
///
/// ```ignore
/// pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
///     let __caller = infrix_sdk::env::caller();
///     if !infrix_sdk::governance::has_capability(&__caller.to_string(), "token:transfer") {
///         return Err(infrix_types::Error::CapabilityDenied);
///     }
///     // original body
/// }
/// ```
pub fn generate_require_capability(cap: &str, input: &ItemFn) -> TokenStream {
    let vis = &input.vis;
    let sig = &input.sig;
    let body = &input.block;
    let attrs: Vec<_> = input.attrs.iter().collect();

    quote! {
        #(#attrs)*
        #vis #sig {
            let __caller = infrix_sdk::env::caller();
            let __caller_str = __caller.to_string().unwrap_or_default();
            if !infrix_sdk::governance::has_capability(&__caller_str, #cap) {
                return Err(infrix_types::Error::CapabilityDenied);
            }
            #body
        }
    }
}

/// Generates the `#[require_approval(threshold = N)]` attribute macro expansion.
///
/// # Example
///
/// ```ignore
/// #[require_approval(threshold = 2)]
/// pub fn withdraw(&mut self, amount: U256) -> Result<(), Error> {
///     // Only proceeds after 2 approvals are received
/// }
/// ```
///
/// # Expansion
///
/// ```ignore
/// pub fn withdraw(&mut self, amount: U256) -> Result<(), Error> {
///     infrix_sdk::governance::require_approval("any", 2)?;
///     // original body
/// }
/// ```
pub fn generate_require_approval(
    threshold: u32,
    role: Option<&str>,
    input: &ItemFn,
) -> TokenStream {
    let vis = &input.vis;
    let sig = &input.sig;
    let body = &input.block;
    let attrs: Vec<_> = input.attrs.iter().collect();
    let role_str = role.unwrap_or("any");

    quote! {
        #(#attrs)*
        #vis #sig {
            infrix_sdk::governance::require_approval(#role_str, #threshold)?;
            #body
        }
    }
}

/// Generates the `#[governed]` attribute macro expansion.
///
/// A `#[governed]` function is wrapped so that its execution is routed
/// through the intent pipeline. Instead of executing directly, it
/// submits an intent representing the operation.
///
/// # Example
///
/// ```ignore
/// #[governed]
/// pub fn large_transfer(&mut self, to: Address, amount: U256) -> Result<(), Error> {
///     // This body becomes the execution target of an intent
/// }
/// ```
pub fn generate_governed(input: &ItemFn) -> TokenStream {
    let vis = &input.vis;
    let sig = &input.sig;
    let body = &input.block;
    let fn_name = &sig.ident;
    let fn_name_str = fn_name.to_string();
    let attrs: Vec<_> = input.attrs.iter().collect();

    quote! {
        #(#attrs)*
        #vis #sig {
            // Check if we are already inside a governed execution context
            // (i.e., the runtime is calling us as part of an intent execution).
            // If so, proceed normally. If not, submit an intent.
            if infrix_sdk::env::is_governed_context() {
                #body
            } else {
                // Serialize arguments and submit as intent
                let __self_addr = infrix_sdk::env::self_address().to_string().unwrap_or_default();
                let __goal_json = infrix_sdk::governance::serialize_call_goal(
                    &__self_addr,
                    #fn_name_str,
                );
                let _ = infrix_sdk::governance::submit_intent(&__goal_json)?;
                // The actual execution will happen when the intent is processed
                Ok(())
            }
        }
    }
}

/// Generates the `#[evidenced]` attribute macro expansion.
///
/// An `#[evidenced]` function automatically generates an evidence link
/// after execution completes (success or failure).
///
/// # Example
///
/// ```ignore
/// #[evidenced]
/// pub fn sensitive_operation(&mut self) -> Result<(), Error> {
///     // Evidence link is automatically appended after execution
/// }
/// ```
pub fn generate_evidenced(input: &ItemFn) -> TokenStream {
    let vis = &input.vis;
    let sig = &input.sig;
    let body = &input.block;
    let fn_name = &sig.ident;
    let fn_name_str = fn_name.to_string();
    let attrs: Vec<_> = input.attrs.iter().collect();

    quote! {
        #(#attrs)*
        #vis #sig {
            let __pre_hash = infrix_sdk::crypto::sha256(
                #fn_name_str.as_bytes()
            );
            let __result = (|| { #body })();
            // Emit evidence event regardless of success/failure
            let __status = if __result.is_ok() { "success" } else { "failure" };
            infrix_sdk::events::emit_raw(
                b"governance.evidence",
                &[
                    __pre_hash.as_ref(),
                    __status.as_bytes(),
                ],
            );
            __result
        }
    }
}
