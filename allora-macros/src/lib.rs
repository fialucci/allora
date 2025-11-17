use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, ImplItem, ImplItemFn, ItemImpl};
use syn::{punctuated::Punctuated, Ident, LitStr, Token};

#[derive(Debug)]
enum ServiceArgKind {
    Name(String),
}

#[derive(Debug, Default)]
struct ServiceArgs {
    name: Option<String>,
}

impl Parse for ServiceArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = ServiceArgs::default();
        let punct: Punctuated<ArgItem, Token![,]> =
            input.parse_terminated(ArgItem::parse, Token![,])?;
        for item in punct {
            match item.kind {
                ServiceArgKind::Name(v) => {
                    if args.name.is_some() {
                        return Err(syn::Error::new(item.span, "duplicate name"));
                    }
                    args.name = Some(v);
                }
            }
        }
        Ok(args)
    }
}

struct ArgItem {
    kind: ServiceArgKind,
    span: proc_macro2::Span,
}
impl Parse for ArgItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let span = ident.span();
        if ident == "name" {
            let _eq: Token![=] = input.parse()?;
            let lit: LitStr = input.parse()?;
            return Ok(ArgItem {
                kind: ServiceArgKind::Name(lit.value()),
                span,
            });
        }
        Err(syn::Error::new(
            span,
            "unsupported #[service] argument; expected name=...",
        ))
    }
}

/// Attribute macro for registering a Service implementation with the runtime inventory.
///
/// Placement: apply to an inherent impl block that defines a constructor and (separately) implements
/// the `Service` trait. Example:
///
/// ```ignore
/// use allora::{Service, Exchange, error::Result};
/// struct MyService;
/// impl MyService { pub fn new() -> Self { Self } }
/// #[service(name="my_service")]
/// impl MyService { /* may also hold helper fns */ }
/// impl Service for MyService { /* provide process/process_sync */ }
/// ```
///
/// Supported forms:
/// * `#[service(name = "custom")]` – explicit implementation identifier to match YAML `service_activator.ref-name`.
/// * Combination: `#[service(name="x")]`
///
/// Constraints / validation:
/// * Must be on an inherent impl (trait impls rejected).
/// * Generics unsupported (emit compile error) – keeps inventory constructor stable.
/// * A zero-arg `new()` method must appear in THIS impl block.
///
/// Reusability / Testability improvements over the prior version:
/// * Structured parsing (no brittle string slicing).
/// * Helpful, span-anchored compile errors.
/// * Optional per-call instantiation for stateful / non-thread-safe services.
/// * Clear default naming semantics.
///
/// NOTE: The macro intentionally does not assert that the Service trait is implemented; the user
/// will receive a normal Rust method resolution error if `process` / `process_sync` is missing.
#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_args = if attr.is_empty() {
        ServiceArgs::default()
    } else {
        parse_macro_input!(attr as ServiceArgs)
    };
    let name_block = parse_macro_input!(item as ItemImpl);

    if name_block.trait_.is_some() {
        return syn::Error::new_spanned(
            &name_block.self_ty,
            "#[service] must be applied to an inherent impl (e.g. impl Type { ... })",
        )
        .to_compile_error()
        .into();
    }
    if !name_block.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &name_block.generics,
            "#[service] does not currently support generic impl blocks",
        )
        .to_compile_error()
        .into();
    }

    let self_ty = name_block.self_ty.clone();

    let mut has_new = false;
    // Always require zero-arg new now (no alternate ctor support).
    for item in &name_block.items {
        if let ImplItem::Fn(ImplItemFn { sig, .. }) = item {
            if sig.ident == "new" && sig.inputs.is_empty() {
                has_new = true;
                break;
            }
        }
    }
    if !has_new {
        return syn::Error::new_spanned(
            &name_block.self_ty,
            "No zero-arg `new()` found in this impl block; define one for #[service]",
        )
        .to_compile_error()
        .into();
    }

    let name_tokens = if let Some(n) = &parsed_args.name {
        quote!(#n)
    } else {
        let type_name = self_ty.to_token_stream().to_string().replace(' ', "");
        quote!(#type_name)
    };
    let processor_body = quote! {{ let svc = std::sync::Arc::new(#self_ty::new()); allora::ClosureProcessor::new(move |exchange: &mut allora::Exchange| { svc.process_sync(exchange)?; Ok(()) }) }};

    let gen = quote! { #name_block inventory::submit! { allora::ServiceDescriptor { name: #name_tokens, constructor: || { use std::sync::Arc; let proc = #processor_body; Arc::new(proc) as Arc<dyn allora::SyncProcessor> } } } };
    gen.into()
}
