//! Shape-shifting contract definition macros.
//!
//! These macros parse declarative Rust contract metadata into the same JSON
//! shape model consumed by the Go runtime. The `#[infrix::shapes]` macro emits
//! deterministic `infrix:shapes` WASM custom-section bytes and typed parameter
//! accessors. The `#[infrix::evolution_rules]` macro emits a companion
//! `infrix:shape-rules` section that the runtime merges into the shape set at
//! deploy-time extraction.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::{BTreeSet, HashSet};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
    Expr, Ident, Item, ItemEnum, ItemFn, ItemMod, LitBool, LitInt, LitStr, Token, Type,
};

const SHAPES_SECTION: &str = "infrix:shapes";
const RULES_SECTION: &str = "infrix:shape-rules";

#[derive(Default)]
struct ShapeSetArgs {
    contract_name: Option<String>,
    default_shape: Option<String>,
    evaluation_interval: Option<u64>,
    cooldown_blocks: Option<u64>,
    max_transitions_per_day: Option<u32>,
}

#[derive(Default)]
struct ShapeAttr {
    name: Option<String>,
    description: Option<String>,
    color: Option<String>,
    priority: Option<u8>,
    params: Vec<ShapeParam>,
}

#[derive(Clone, Eq, PartialEq)]
struct ShapeParam {
    name: String,
    rust_type: String,
    shape_type: String,
    value_json: String,
    description: Option<String>,
}

struct ParsedShape {
    name: String,
    description: Option<String>,
    color: Option<String>,
    priority: Option<u8>,
    params: Vec<ShapeParam>,
}

#[derive(Default)]
struct RuleSetArgs {
    evaluation_interval: Option<u64>,
    cooldown_blocks: Option<u64>,
    max_transitions_per_day: Option<u32>,
}

#[derive(Default)]
struct RuleAttr {
    name: Option<String>,
    description: Option<String>,
    condition: Option<String>,
    target_shape: Option<String>,
    source_shapes: Vec<String>,
    priority: Option<u8>,
    duration_blocks: Option<u64>,
    enabled: Option<bool>,
}

struct ParsedRule {
    name: String,
    description: Option<String>,
    condition: String,
    target_shape: String,
    source_shapes: Vec<String>,
    priority: u8,
    duration_blocks: Option<u64>,
    enabled: bool,
}

struct ParamSpec {
    name: Ident,
    ty: Type,
    value: Expr,
}

impl Parse for ParamSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(Self { name, ty, value })
    }
}

pub fn shapes_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = match syn::parse::<ItemEnum>(item) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    match expand_shapes(attr.into(), &mut input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub fn evolution_rules_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = match syn::parse::<ItemMod>(item) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    match expand_evolution_rules(attr.into(), &mut input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

pub fn rule_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    match validate_rule_attr(attr.into(), None) {
        Ok(()) => item,
        Err(err) => err.to_compile_error().into(),
    }
}

pub fn shape_impl(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    syn::Error::new(
        Span::call_site(),
        "#[shape] is a variant helper for #[shapes] enums and must be consumed by #[shapes]",
    )
    .to_compile_error()
    .into()
}

fn expand_shapes(attr: TokenStream2, input: &mut ItemEnum) -> syn::Result<TokenStream2> {
    let args = parse_shapes_args(attr)?;
    let enum_ident = input.ident.clone();
    let contract_name = args
        .contract_name
        .clone()
        .unwrap_or_else(|| enum_ident.to_string());
    let default_shape = args.default_shape.clone().ok_or_else(|| {
        syn::Error::new(
            enum_ident.span(),
            "#[shapes] requires default = \"shape_name\"",
        )
    })?;

    let mut parsed = Vec::new();
    for variant in input.variants.iter_mut() {
        let mut shape_attrs = Vec::new();
        let mut retained_attrs = Vec::new();
        for attr in variant.attrs.drain(..) {
            if is_attr_named(&attr, "shape") {
                shape_attrs.push(attr);
            } else {
                retained_attrs.push(attr);
            }
        }
        variant.attrs = retained_attrs;

        if shape_attrs.len() != 1 {
            return Err(syn::Error::new(
                variant.ident.span(),
                "each #[shapes] enum variant must have exactly one #[shape(...)] attribute",
            ));
        }

        let shape_attr = parse_shape_attr(&shape_attrs[0])?;
        let name = shape_attr
            .name
            .unwrap_or_else(|| to_snake_case(&variant.ident.to_string()));
        validate_name(&name, "shape", 32, variant.ident.span())?;
        if let Some(color) = &shape_attr.color {
            validate_color(color, variant.ident.span())?;
        }
        parsed.push(ParsedShape {
            name,
            description: shape_attr.description,
            color: shape_attr.color,
            priority: shape_attr.priority,
            params: shape_attr.params,
        });
    }

    if parsed.is_empty() {
        return Err(syn::Error::new(
            enum_ident.span(),
            "#[shapes] requires at least one shape variant",
        ));
    }

    validate_shape_set(&parsed, &default_shape, enum_ident.span())?;
    let json = build_shape_set_json(&contract_name, &default_shape, &parsed, &args);
    let json_bytes = json.as_bytes();
    let json_len = json_bytes.len();
    let section_ident = format_ident!("__INFRIX_SHAPES_{}", enum_ident.to_string().to_uppercase());
    let accessor_tokens = build_shape_accessors(&enum_ident, &parsed[0].params)?;

    Ok(quote! {
        #input

        #[cfg(target_arch = "wasm32")]
        #[link_section = #SHAPES_SECTION]
        #[used]
        static #section_ident: [u8; #json_len] = [#(#json_bytes),*];

        impl #enum_ident {
            pub fn __shape_json() -> &'static str {
                const SHAPES: &str = #json;
                SHAPES
            }

            #accessor_tokens
        }
    })
}

fn expand_evolution_rules(attr: TokenStream2, input: &mut ItemMod) -> syn::Result<TokenStream2> {
    let args = parse_rule_set_args(attr)?;
    let mod_ident = input.ident.clone();
    let Some((_, items)) = input.content.as_mut() else {
        return Err(syn::Error::new(
            mod_ident.span(),
            "#[evolution_rules] requires an inline module body",
        ));
    };

    let mut rules = Vec::new();
    for item in items.iter_mut() {
        let Item::Fn(func) = item else {
            continue;
        };
        let mut rule_attrs = Vec::new();
        let mut retained_attrs = Vec::new();
        for attr in func.attrs.drain(..) {
            if is_attr_named(&attr, "rule") {
                rule_attrs.push(attr);
            } else {
                retained_attrs.push(attr);
            }
        }
        func.attrs = retained_attrs;

        if rule_attrs.len() > 1 {
            return Err(syn::Error::new(
                func.sig.ident.span(),
                "each rule function may have at most one #[rule(...)] attribute",
            ));
        }
        if let Some(attr) = rule_attrs.first() {
            let rule_attr = parse_rule_attr(attr)?;
            rules.push(rule_from_attr(&func.sig.ident, rule_attr)?);
        }
    }

    if rules.is_empty() {
        return Err(syn::Error::new(
            mod_ident.span(),
            "#[evolution_rules] requires at least one #[rule(...)] function",
        ));
    }
    validate_rules(&rules, mod_ident.span())?;

    let json = build_rules_fragment_json(&rules, &args);
    let json_bytes = json.as_bytes();
    let json_len = json_bytes.len();
    items.push(syn::parse2(quote! {
        pub const __INFRIX_SHAPE_RULES_JSON: &str = #json;
    })?);
    let section_ident = format_ident!(
        "__INFRIX_SHAPE_RULES_{}",
        mod_ident.to_string().to_uppercase()
    );

    Ok(quote! {
        #input

        #[cfg(target_arch = "wasm32")]
        #[link_section = #RULES_SECTION]
        #[used]
        static #section_ident: [u8; #json_len] = [#(#json_bytes),*];
    })
}

fn parse_shapes_args(attr: TokenStream2) -> syn::Result<ShapeSetArgs> {
    let mut args = ShapeSetArgs::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("default") {
            args.default_shape = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("contract") || meta.path.is_ident("contract_name") {
            args.contract_name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("evaluation_interval") {
            args.evaluation_interval = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("cooldown_blocks") {
            args.cooldown_blocks = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("max_transitions_per_day") {
            args.max_transitions_per_day = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else {
            return Err(meta.error("unknown #[shapes] attribute"));
        }
        Ok(())
    });
    parser.parse2(attr)?;
    Ok(args)
}

fn parse_rule_set_args(attr: TokenStream2) -> syn::Result<RuleSetArgs> {
    let mut args = RuleSetArgs::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("evaluation_interval") {
            args.evaluation_interval = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("cooldown_blocks") {
            args.cooldown_blocks = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("max_transitions_per_day") {
            args.max_transitions_per_day = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else {
            return Err(meta.error("unknown #[evolution_rules] attribute"));
        }
        Ok(())
    });
    parser.parse2(attr)?;
    Ok(args)
}

fn parse_shape_attr(attr: &syn::Attribute) -> syn::Result<ShapeAttr> {
    let mut shape = ShapeAttr::default();
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            shape.name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("description") {
            shape.description = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("color") {
            shape.color = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("priority") {
            shape.priority = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("params") {
            let content;
            parenthesized!(content in meta.input);
            let params = Punctuated::<ParamSpec, Token![,]>::parse_terminated(&content)?;
            for param in params {
                let name = param.name.to_string();
                validate_name(&name, "parameter", 64, param.name.span())?;
                let shape_type = rust_type_to_shape_type(&param.ty)?;
                let value_json = expr_to_json_value(&param.value, &shape_type)?;
                let rust_type = {
                    let ty = &param.ty;
                    quote!(#ty).to_string()
                };
                shape.params.push(ShapeParam {
                    name,
                    rust_type,
                    shape_type,
                    value_json,
                    description: None,
                });
            }
        } else {
            return Err(meta.error("unknown #[shape] attribute"));
        }
        Ok(())
    })?;
    Ok(shape)
}

fn parse_rule_attr(attr: &syn::Attribute) -> syn::Result<RuleAttr> {
    let mut rule = RuleAttr::default();
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            rule.name = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("description") {
            rule.description = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("condition") {
            rule.condition = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("target") || meta.path.is_ident("target_shape") {
            rule.target_shape = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("source") || meta.path.is_ident("source_shape") {
            rule.source_shapes
                .push(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("sources") || meta.path.is_ident("source_shapes") {
            let content;
            parenthesized!(content in meta.input);
            let sources = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
            rule.source_shapes
                .extend(sources.into_iter().map(|source| source.value()));
        } else if meta.path.is_ident("priority") {
            rule.priority = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("duration_blocks") {
            rule.duration_blocks = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("enabled") {
            rule.enabled = Some(meta.value()?.parse::<LitBool>()?.value);
        } else {
            return Err(meta.error("unknown #[rule] attribute"));
        }
        Ok(())
    })?;
    Ok(rule)
}

fn validate_rule_attr(attr: TokenStream2, fn_name: Option<&Ident>) -> syn::Result<()> {
    let item: ItemFn = parse_quote! { fn placeholder() {} };
    let ident = fn_name.unwrap_or(&item.sig.ident);
    let synthetic_attr: syn::Attribute = syn::parse_quote! { #[rule(#attr)] };
    let parsed = parse_rule_attr(&synthetic_attr)?;
    let rule = rule_from_attr(ident, parsed)?;
    validate_rules(&[rule], Span::call_site())
}

fn rule_from_attr(fn_ident: &Ident, attr: RuleAttr) -> syn::Result<ParsedRule> {
    let name = attr
        .name
        .unwrap_or_else(|| to_snake_case(&fn_ident.to_string()));
    validate_name(&name, "rule", 64, fn_ident.span())?;
    let target_shape = attr.target_shape.ok_or_else(|| {
        syn::Error::new(fn_ident.span(), "#[rule] requires target = \"shape_name\"")
    })?;
    validate_name(&target_shape, "target shape", 32, fn_ident.span())?;
    let condition = attr
        .condition
        .ok_or_else(|| syn::Error::new(fn_ident.span(), "#[rule] requires condition = \"...\""))?;
    if condition.trim().is_empty() {
        return Err(syn::Error::new(
            fn_ident.span(),
            "#[rule] condition cannot be empty",
        ));
    }
    for source in &attr.source_shapes {
        validate_name(source, "source shape", 32, fn_ident.span())?;
    }

    Ok(ParsedRule {
        name,
        description: attr.description,
        condition,
        target_shape,
        source_shapes: attr.source_shapes,
        priority: attr.priority.unwrap_or(100),
        duration_blocks: attr.duration_blocks,
        enabled: attr.enabled.unwrap_or(true),
    })
}

fn validate_shape_set(shapes: &[ParsedShape], default_shape: &str, span: Span) -> syn::Result<()> {
    validate_name(default_shape, "default shape", 32, span)?;
    let mut seen = HashSet::new();
    for shape in shapes {
        if !seen.insert(shape.name.clone()) {
            return Err(syn::Error::new(
                span,
                format!("duplicate shape name: {}", shape.name),
            ));
        }
        let mut params = HashSet::new();
        for param in &shape.params {
            if !params.insert(param.name.clone()) {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "duplicate parameter name in shape {}: {}",
                        shape.name, param.name
                    ),
                ));
            }
        }
    }
    if !seen.contains(default_shape) {
        return Err(syn::Error::new(
            span,
            format!("default shape {} is not declared", default_shape),
        ));
    }

    let reference = &shapes[0].params;
    for shape in &shapes[1..] {
        if shape.params.len() != reference.len() {
            return Err(syn::Error::new(
                span,
                format!("shape {} does not match the parameter schema", shape.name),
            ));
        }
        for (actual, expected) in shape.params.iter().zip(reference.iter()) {
            if actual.name != expected.name || actual.shape_type != expected.shape_type {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "shape {} parameter schema mismatch: expected {}:{}, got {}:{}",
                        shape.name,
                        expected.name,
                        expected.shape_type,
                        actual.name,
                        actual.shape_type
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_rules(rules: &[ParsedRule], span: Span) -> syn::Result<()> {
    let mut seen = HashSet::new();
    for rule in rules {
        if !seen.insert(rule.name.clone()) {
            return Err(syn::Error::new(
                span,
                format!("duplicate evolution rule name: {}", rule.name),
            ));
        }
        validate_name(&rule.target_shape, "target shape", 32, span)?;
        for source in &rule.source_shapes {
            validate_name(source, "source shape", 32, span)?;
        }
    }
    Ok(())
}

fn build_shape_set_json(
    contract_name: &str,
    default_shape: &str,
    shapes: &[ParsedShape],
    args: &ShapeSetArgs,
) -> String {
    let schema = &shapes[0].params;
    let mut fields = vec![
        "\"version\":1".to_string(),
        format!("\"contract_name\":{}", json_string(contract_name)),
        format!("\"default_shape\":{}", json_string(default_shape)),
        format!("\"shapes\":{}", shapes_json(shapes)),
        "\"evolution_rules\":[]".to_string(),
        format!("\"parameter_schema\":{}", schema_json(schema)),
    ];
    if let Some(value) = args.evaluation_interval {
        fields.push(format!("\"evaluation_interval\":{}", value));
    }
    if let Some(value) = args.cooldown_blocks {
        fields.push(format!("\"cooldown_blocks\":{}", value));
    }
    if let Some(value) = args.max_transitions_per_day {
        fields.push(format!("\"max_transitions_per_day\":{}", value));
    }
    format!("{{{}}}", fields.join(","))
}

fn build_rules_fragment_json(rules: &[ParsedRule], args: &RuleSetArgs) -> String {
    let mut fields = vec![
        "\"version\":1".to_string(),
        format!("\"evolution_rules\":{}", rules_json(rules)),
    ];
    if let Some(value) = args.evaluation_interval {
        fields.push(format!("\"evaluation_interval\":{}", value));
    }
    if let Some(value) = args.cooldown_blocks {
        fields.push(format!("\"cooldown_blocks\":{}", value));
    }
    if let Some(value) = args.max_transitions_per_day {
        fields.push(format!("\"max_transitions_per_day\":{}", value));
    }
    format!("{{{}}}", fields.join(","))
}

fn shapes_json(shapes: &[ParsedShape]) -> String {
    let values = shapes
        .iter()
        .map(|shape| {
            let mut fields = vec![
                format!("\"name\":{}", json_string(&shape.name)),
                format!("\"parameters\":{}", params_json(&shape.params)),
            ];
            if let Some(description) = &shape.description {
                fields.push(format!("\"description\":{}", json_string(description)));
            }
            if let Some(color) = &shape.color {
                fields.push(format!("\"color\":{}", json_string(color)));
            }
            if let Some(priority) = shape.priority {
                fields.push(format!("\"priority\":{}", priority));
            }
            format!("{{{}}}", fields.join(","))
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn params_json(params: &[ShapeParam]) -> String {
    let values = params
        .iter()
        .map(|param| {
            let mut fields = vec![
                format!("\"name\":{}", json_string(&param.name)),
                format!("\"type\":{}", json_string(&param.shape_type)),
                format!("\"value\":{}", param.value_json),
            ];
            if let Some(description) = &param.description {
                fields.push(format!("\"description\":{}", json_string(description)));
            }
            format!("{{{}}}", fields.join(","))
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn schema_json(params: &[ShapeParam]) -> String {
    let values = params
        .iter()
        .map(|param| {
            let mut fields = vec![
                format!("\"name\":{}", json_string(&param.name)),
                format!("\"type\":{}", json_string(&param.shape_type)),
            ];
            if let Some(description) = &param.description {
                fields.push(format!("\"description\":{}", json_string(description)));
            }
            format!("{{{}}}", fields.join(","))
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn rules_json(rules: &[ParsedRule]) -> String {
    let values = rules
        .iter()
        .map(|rule| {
            let mut fields = vec![
                format!("\"name\":{}", json_string(&rule.name)),
                format!("\"condition\":{}", json_string(&rule.condition)),
                format!("\"target_shape\":{}", json_string(&rule.target_shape)),
                format!("\"priority\":{}", rule.priority),
                format!("\"enabled\":{}", rule.enabled),
            ];
            if let Some(description) = &rule.description {
                fields.push(format!("\"description\":{}", json_string(description)));
            }
            if !rule.source_shapes.is_empty() {
                fields.push(format!(
                    "\"source_shapes\":[{}]",
                    rule.source_shapes
                        .iter()
                        .map(|source| json_string(source))
                        .collect::<Vec<_>>()
                        .join(",")
                ));
            }
            if let Some(duration) = rule.duration_blocks {
                fields.push(format!("\"duration_blocks\":{}", duration));
            }
            format!("{{{}}}", fields.join(","))
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn build_shape_accessors(enum_ident: &Ident, params: &[ShapeParam]) -> syn::Result<TokenStream2> {
    let mut accessors = Vec::new();
    let mut seen = BTreeSet::new();
    for param in params {
        if !seen.insert(param.name.clone()) {
            continue;
        }
        let accessor = format_ident!("{}", param.name);
        let param_name = &param.name;
        let tokens = match param.shape_type.as_str() {
            "u64" => quote! {
                pub fn #accessor() -> u64 {
                    ::infrix_sdk::shapes::param_u64(#param_name)
                }
            },
            "i64" => quote! {
                pub fn #accessor() -> i64 {
                    ::infrix_sdk::shapes::param_i64(#param_name)
                }
            },
            "bool" => quote! {
                pub fn #accessor() -> bool {
                    ::infrix_sdk::shapes::param_bool(#param_name)
                }
            },
            "string" => quote! {
                pub fn #accessor() -> ::std::string::String {
                    ::infrix_sdk::shapes::param_string(#param_name)
                }
            },
            "bytes" => quote! {
                pub fn #accessor() -> ::std::vec::Vec<u8> {
                    ::infrix_sdk::shapes::param_bytes(#param_name)
                }
            },
            "u128" => quote! {
                pub fn #accessor() -> u128 {
                    ::infrix_sdk::shapes::param_u128(#param_name)
                }
            },
            other => {
                return Err(syn::Error::new(
                    enum_ident.span(),
                    format!("unsupported accessor parameter type: {}", other),
                ))
            }
        };
        accessors.push(tokens);
    }
    Ok(quote! { #(#accessors)* })
}

fn rust_type_to_shape_type(ty: &Type) -> syn::Result<String> {
    let raw = quote!(#ty).to_string().replace(' ', "");
    let shape_type = match raw.as_str() {
        "u64" => "u64",
        "i64" => "i64",
        "bool" => "bool",
        "String" | "&str" | "str" => "string",
        "Vec<u8>" => "bytes",
        "u128" => "u128",
        other => {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("unsupported shape parameter type: {}", other),
            ))
        }
    };
    Ok(shape_type.to_string())
}

fn expr_to_json_value(expr: &Expr, shape_type: &str) -> syn::Result<String> {
    match (shape_type, expr) {
        ("u64" | "i64" | "u128", Expr::Lit(lit)) => {
            if let syn::Lit::Int(value) = &lit.lit {
                Ok(value.base10_digits().to_string())
            } else {
                Err(syn::Error::new_spanned(
                    expr,
                    "numeric parameter requires an integer literal",
                ))
            }
        }
        ("bool", Expr::Lit(lit)) => {
            if let syn::Lit::Bool(value) = &lit.lit {
                Ok(value.value.to_string())
            } else {
                Err(syn::Error::new_spanned(
                    expr,
                    "bool parameter requires true or false",
                ))
            }
        }
        ("string", Expr::Lit(lit)) => {
            if let syn::Lit::Str(value) = &lit.lit {
                Ok(json_string(&value.value()))
            } else {
                Err(syn::Error::new_spanned(
                    expr,
                    "string parameter requires a string literal",
                ))
            }
        }
        ("bytes", Expr::Array(array)) => {
            let mut values = Vec::new();
            for elem in &array.elems {
                let Expr::Lit(lit) = elem else {
                    return Err(syn::Error::new_spanned(
                        elem,
                        "bytes parameter requires byte literals",
                    ));
                };
                let syn::Lit::Int(value) = &lit.lit else {
                    return Err(syn::Error::new_spanned(
                        elem,
                        "bytes parameter requires byte literals",
                    ));
                };
                let parsed: u8 = value.base10_parse()?;
                values.push(parsed.to_string());
            }
            Ok(format!("[{}]", values.join(",")))
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            format!("invalid default literal for {} parameter", shape_type),
        )),
    }
}

fn validate_name(name: &str, kind: &str, max_len: usize, span: Span) -> syn::Result<()> {
    if name.is_empty() {
        return Err(syn::Error::new(span, format!("{} name is empty", kind)));
    }
    if name.len() > max_len {
        return Err(syn::Error::new(
            span,
            format!("{} name {} exceeds {} characters", kind, name, max_len),
        ));
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(syn::Error::new(span, format!("{} name is empty", kind)));
    };
    if !(first == '_' || first.is_ascii_lowercase()) {
        return Err(syn::Error::new(
            span,
            format!(
                "invalid {} name {}: must match [a-z_][a-z0-9_]*",
                kind, name
            ),
        ));
    }
    if !chars.all(|ch| ch == '_' || ch.is_ascii_lowercase() || ch.is_ascii_digit()) {
        return Err(syn::Error::new(
            span,
            format!(
                "invalid {} name {}: must match [a-z_][a-z0-9_]*",
                kind, name
            ),
        ));
    }
    Ok(())
}

fn validate_color(color: &str, span: Span) -> syn::Result<()> {
    match color {
        "" | "green" | "blue" | "yellow" | "orange" | "red" | "purple" | "gray" => Ok(()),
        _ => Err(syn::Error::new(
            span,
            format!("invalid shape color: {}", color),
        )),
    }
}

fn is_attr_named(attr: &syn::Attribute, name: &str) -> bool {
    attr.path()
        .segments
        .last()
        .map(|segment| segment.ident == name)
        .unwrap_or(false)
}

fn to_snake_case(input: &str) -> String {
    let mut out = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_deterministic_shape_json() {
        let shapes = vec![
            ParsedShape {
                name: "conservative".into(),
                description: Some("Low risk".into()),
                color: Some("green".into()),
                priority: Some(1),
                params: vec![
                    ShapeParam {
                        name: "ltv_ratio".into(),
                        rust_type: "u64".into(),
                        shape_type: "u64".into(),
                        value_json: "5000".into(),
                        description: None,
                    },
                    ShapeParam {
                        name: "borrow_enabled".into(),
                        rust_type: "bool".into(),
                        shape_type: "bool".into(),
                        value_json: "true".into(),
                        description: None,
                    },
                ],
            },
            ParsedShape {
                name: "growth".into(),
                description: None,
                color: Some("blue".into()),
                priority: None,
                params: vec![
                    ShapeParam {
                        name: "ltv_ratio".into(),
                        rust_type: "u64".into(),
                        shape_type: "u64".into(),
                        value_json: "7500".into(),
                        description: None,
                    },
                    ShapeParam {
                        name: "borrow_enabled".into(),
                        rust_type: "bool".into(),
                        shape_type: "bool".into(),
                        value_json: "true".into(),
                        description: None,
                    },
                ],
            },
        ];
        validate_shape_set(&shapes, "conservative", Span::call_site()).unwrap();
        let json = build_shape_set_json(
            "Lending",
            "conservative",
            &shapes,
            &ShapeSetArgs {
                cooldown_blocks: Some(5),
                ..ShapeSetArgs::default()
            },
        );
        assert_eq!(
            json,
            "{\"version\":1,\"contract_name\":\"Lending\",\"default_shape\":\"conservative\",\"shapes\":[{\"name\":\"conservative\",\"parameters\":[{\"name\":\"ltv_ratio\",\"type\":\"u64\",\"value\":5000},{\"name\":\"borrow_enabled\",\"type\":\"bool\",\"value\":true}],\"description\":\"Low risk\",\"color\":\"green\",\"priority\":1},{\"name\":\"growth\",\"parameters\":[{\"name\":\"ltv_ratio\",\"type\":\"u64\",\"value\":7500},{\"name\":\"borrow_enabled\",\"type\":\"bool\",\"value\":true}],\"color\":\"blue\"}],\"evolution_rules\":[],\"parameter_schema\":[{\"name\":\"ltv_ratio\",\"type\":\"u64\"},{\"name\":\"borrow_enabled\",\"type\":\"bool\"}],\"cooldown_blocks\":5}"
        );
    }

    #[test]
    fn rejects_default_shape_that_is_not_declared() {
        let shapes = vec![ParsedShape {
            name: "alpha".into(),
            description: None,
            color: None,
            priority: None,
            params: vec![],
        }];
        let err = validate_shape_set(&shapes, "beta", Span::call_site()).unwrap_err();
        assert!(err
            .to_string()
            .contains("default shape beta is not declared"));
    }

    #[test]
    fn rejects_schema_mismatch() {
        let shapes = vec![
            ParsedShape {
                name: "alpha".into(),
                description: None,
                color: None,
                priority: None,
                params: vec![ShapeParam {
                    name: "rate".into(),
                    rust_type: "u64".into(),
                    shape_type: "u64".into(),
                    value_json: "1".into(),
                    description: None,
                }],
            },
            ParsedShape {
                name: "beta".into(),
                description: None,
                color: None,
                priority: None,
                params: vec![ShapeParam {
                    name: "enabled".into(),
                    rust_type: "bool".into(),
                    shape_type: "bool".into(),
                    value_json: "true".into(),
                    description: None,
                }],
            },
        ];
        let err = validate_shape_set(&shapes, "alpha", Span::call_site()).unwrap_err();
        assert!(err.to_string().contains("parameter schema mismatch"));
    }

    #[test]
    fn builds_rule_fragment_json() {
        let rules = vec![ParsedRule {
            name: "grow".into(),
            description: Some("Move to growth".into()),
            condition: "env::price() > 200".into(),
            target_shape: "growth".into(),
            source_shapes: vec!["conservative".into()],
            priority: 0,
            duration_blocks: Some(5),
            enabled: true,
        }];
        validate_rules(&rules, Span::call_site()).unwrap();
        let json = build_rules_fragment_json(
            &rules,
            &RuleSetArgs {
                cooldown_blocks: Some(5),
                ..RuleSetArgs::default()
            },
        );
        assert_eq!(
            json,
            "{\"version\":1,\"evolution_rules\":[{\"name\":\"grow\",\"condition\":\"env::price() > 200\",\"target_shape\":\"growth\",\"priority\":0,\"enabled\":true,\"description\":\"Move to growth\",\"source_shapes\":[\"conservative\"],\"duration_blocks\":5}],\"cooldown_blocks\":5}"
        );
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
}
