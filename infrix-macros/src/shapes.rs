//! Proc macro stubs for shape-shifting contract definitions.
//!
//! These macros generate the `infrix:shapes` WASM custom section from
//! declarative shape definitions in Rust contract code.
//!
//! # Usage
//!
//! ```ignore
//! #[infrix::shapes(default = "conservative")]
//! pub enum Shape {
//!     #[infrix::shape(
//!         description = "Low-risk parameters",
//!         color = "green",
//!         params(ltv_ratio: u64 = 5000, borrow_enabled: bool = true),
//!     )]
//!     Conservative,
//!
//!     #[infrix::shape(
//!         description = "High-growth parameters",
//!         color = "blue",
//!         params(ltv_ratio: u64 = 7500, borrow_enabled: bool = true),
//!     )]
//!     Growth,
//! }
//!
//! #[infrix::evolution_rules(cooldown_blocks = 5)]
//! pub mod rules {
//!     #[infrix::rule(
//!         priority = 0,
//!         target = "growth",
//!         condition = r#"env::price() > 200"#,
//!     )]
//!     pub fn grow_on_price() {}
//! }
//! ```
//!
//! # Generated Output
//!
//! The `#[infrix::shapes]` macro expands to a `#[link_section = "infrix:shapes"]`
//! static containing a JSON-encoded `ShapeSet`. The `#[infrix::evolution_rules]`
//! macro appends its rules to the same section.
//!
//! The `#[infrix::shape]` attribute generates per-shape parameter accessors
//! that call the `env_shape_param_*` host functions with type safety.

use proc_macro::TokenStream;

/// Declares the set of shapes for a shape-shifting contract.
///
/// The attribute takes a `default = "shape_name"` argument specifying
/// which shape is active at deploy time.
///
/// Applied to an enum where each variant represents one shape.
pub fn shapes_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Phase E stub: In a full implementation, this proc macro would:
    // 1. Parse each enum variant's #[infrix::shape(...)] attributes
    // 2. Extract parameter names, types, default values, immune configs
    // 3. Build a ShapeSet JSON structure
    // 4. Emit a #[link_section = "infrix:shapes"] static with the JSON bytes
    // 5. Generate type-safe accessor functions for each parameter
    //
    // For now, pass through the item unchanged. The actual section injection
    // is handled by the build toolchain (infrix build) which calls
    // InjectShapesSection() from the Go side.
    item
}

/// Declares evolution rules for shape transitions.
///
/// Applied to a module containing `#[infrix::rule(...)]` functions.
/// Attributes: `cooldown_blocks`, `evaluation_interval`, `max_transitions_per_day`.
pub fn evolution_rules_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Phase E stub: In a full implementation, this proc macro would:
    // 1. Parse each #[infrix::rule(...)] function in the module
    // 2. Extract rule name, priority, condition, target, source shapes, duration
    // 3. Append EvolutionRule entries to the ShapeSet JSON
    // 4. Validate condition syntax at compile time (optional, can defer to deploy)
    item
}

/// Declares a single evolution rule within an `#[infrix::evolution_rules]` module.
///
/// Attributes: `priority`, `target`, `source` (optional), `duration_blocks` (optional),
/// `condition` (ICEL expression string).
pub fn rule_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Declares a single shape variant within an `#[infrix::shapes]` enum.
///
/// Attributes: `description`, `color`, `params(...)`, `immune(...)`.
pub fn shape_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
