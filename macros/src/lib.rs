//! Derive macros for `bidi-protocol`.
//!
//! Mirrors the `browser-protocol` macro crate: the generator emits plain
//! structs, and these derives add the builder, read-only getters and the
//! command/event glue.
//!
//! * [`BidiBuilder`] - `builder(required...)` + chained setters + getters.
//! * [`BidiCommand`] - `METHOD` + the `BidiCommand` trait impl for a command's
//!   params struct, driven by `#[bidi(method = "...", response = "...")]`.
//! * [`BidiEvent`] - `METHOD` + the `BidiEvent` trait impl, driven by
//!   `#[bidi(method = "...")]`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, LitStr,
    PathArguments, Type,
};

/// Generates a builder and read-only getters for a WebDriver BiDi type.
///
/// Non-`Option` fields become required arguments to `builder(...)`; `Option`
/// fields become chainable setters. Getter return types borrow (`&str`, `&T`,
/// `&[T]`, `Option<&T>`, ...).
#[proc_macro_derive(BidiBuilder)]
pub fn derive_bidi_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_builder(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_builder(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_ident = input.ident.clone();
    let builder_ident = format_ident!("{}Builder", struct_ident);
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let builder_where = &generics.where_clause;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named.named.iter().collect::<Vec<_>>(),
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "BidiBuilder can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "BidiBuilder can only be derived for structs",
            ));
        }
    };

    let mut getters: Vec<TokenStream2> = Vec::new();
    let mut builder_fields: Vec<TokenStream2> = Vec::new();
    let mut builder_inits: Vec<TokenStream2> = Vec::new();
    let mut build_fields: Vec<TokenStream2> = Vec::new();
    let mut required_args: Vec<TokenStream2> = Vec::new();
    let mut required_names: Vec<String> = Vec::new();
    let mut setters: Vec<TokenStream2> = Vec::new();

    for field in fields {
        let name = field
            .ident
            .clone()
            .expect("named field is guaranteed to have an identifier");
        let field_ty = field.ty.clone();
        let docs: Vec<&Attribute> = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .collect();

        let (optional, core_ty) = match option_inner(&field_ty) {
            Some(inner) => (true, inner),
            None => (false, field_ty.clone()),
        };

        builder_fields.push(quote! { #name: #field_ty });
        build_fields.push(quote! { #name: self.#name });

        if optional {
            builder_inits.push(quote! { #name: None });
            setters.push(quote! {
                #(#docs)*
                pub fn #name(mut self, #name: impl Into<#core_ty>) -> Self {
                    self.#name = Some(#name.into());
                    self
                }
            });
        } else {
            builder_inits.push(quote! { #name: #name.into() });
            required_args.push(quote! { #name: impl Into<#core_ty> });
            required_names.push(name.to_string());
        }

        getters.push(make_getter(&name, &core_ty, optional, &docs));
    }

    let builder_init = if builder_inits.is_empty() {
        quote! { #builder_ident {} }
    } else {
        quote! { #builder_ident { #(#builder_inits),* } }
    };

    let builder_doc = if required_names.is_empty() {
        "Creates a builder for this type.".to_string()
    } else {
        format!(
            "Creates a builder for this type. Required parameters: {}.",
            required_names
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let default_derive = if required_names.is_empty() {
        quote! { #[derive(Default)] }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #[doc = #builder_doc]
            pub fn builder(#(#required_args),*) -> #builder_ident #ty_generics {
                #builder_init
            }

            #(#getters)*
        }

        #default_derive
        pub struct #builder_ident #generics #builder_where {
            #(#builder_fields),*
        }

        impl #impl_generics #builder_ident #ty_generics #where_clause {
            #(#setters)*

            pub fn build(self) -> #struct_ident #ty_generics {
                #struct_ident {
                    #(#build_fields),*
                }
            }
        }
    })
}

fn make_getter(name: &Ident, core_ty: &Type, optional: bool, docs: &[&Attribute]) -> TokenStream2 {
    let doc = quote! { #(#docs)* };

    match classify(core_ty) {
        FieldKind::Str => {
            if optional {
                quote! {
                    #doc
                    pub fn #name(&self) -> Option<&str> { self.#name.as_deref() }
                }
            } else {
                quote! {
                    #doc
                    pub fn #name(&self) -> &str { self.#name.as_ref() }
                }
            }
        }
        FieldKind::Vec(inner) => {
            if optional {
                quote! {
                    #doc
                    pub fn #name(&self) -> Option<&[#inner]> { self.#name.as_deref() }
                }
            } else {
                quote! {
                    #doc
                    pub fn #name(&self) -> &[#inner] { self.#name.as_slice() }
                }
            }
        }
        FieldKind::Boxed(inner) => {
            if optional {
                quote! {
                    #doc
                    pub fn #name(&self) -> Option<&#inner> { self.#name.as_deref() }
                }
            } else {
                quote! {
                    #doc
                    pub fn #name(&self) -> &#inner { &self.#name }
                }
            }
        }
        FieldKind::Primitive => {
            if optional {
                quote! {
                    #doc
                    pub fn #name(&self) -> Option<#core_ty> { self.#name }
                }
            } else {
                quote! {
                    #doc
                    pub fn #name(&self) -> #core_ty { self.#name }
                }
            }
        }
        FieldKind::Other => {
            if optional {
                quote! {
                    #doc
                    pub fn #name(&self) -> Option<&#core_ty> { self.#name.as_ref() }
                }
            } else {
                quote! {
                    #doc
                    pub fn #name(&self) -> &#core_ty { &self.#name }
                }
            }
        }
    }
}

enum FieldKind {
    Str,
    Vec(Type),
    Boxed(Type),
    Primitive,
    Other,
}

fn classify(ty: &Type) -> FieldKind {
    let ident = match path_ident(ty) {
        Some(ident) => ident,
        None => return FieldKind::Other,
    };

    if ident == "Cow" || ident == "String" {
        FieldKind::Str
    } else if ident == "Vec" {
        FieldKind::Vec(type_arg(ty).unwrap_or_else(|| syn::parse_quote!(JsonValue)))
    } else if ident == "Box" {
        FieldKind::Boxed(type_arg(ty).unwrap_or_else(|| syn::parse_quote!(JsonValue)))
    } else if is_primitive(ident) {
        FieldKind::Primitive
    } else {
        FieldKind::Other
    }
}

fn option_inner(ty: &Type) -> Option<Type> {
    match path_ident(ty) {
        Some(ident) if ident == "Option" => type_arg(ty),
        _ => None,
    }
}

/// Last path segment ident, e.g. `Option` in `std::option::Option<T>`.
fn path_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            path.path.segments.last().map(|segment| &segment.ident)
        }
        _ => None,
    }
}

/// First type argument, skipping lifetimes, e.g. `T` in `Cow<'a, T>`.
fn type_arg(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    })
}

fn is_primitive(ident: &Ident) -> bool {
    matches!(
        ident.to_string().as_str(),
        "bool"
            | "char"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
    )
}

/// Generates the `METHOD` constant and the `BidiCommand` impl for a command's
/// parameter struct, driven by `#[bidi(method = "...", response = "...")]`.
#[proc_macro_derive(BidiCommand, attributes(bidi))]
pub fn derive_bidi_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_command(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates the `METHOD` constant and the `BidiEvent` impl for an event struct,
/// driven by `#[bidi(method = "...")]`.
#[proc_macro_derive(BidiEvent, attributes(bidi))]
pub fn derive_bidi_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_event(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn bidi_meta(attrs: &[Attribute]) -> syn::Result<(Option<String>, Option<String>)> {
    let mut method = None;
    let mut response = None;
    for attr in attrs {
        if !attr.path().is_ident("bidi") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("method") {
                method = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("response") {
                response = Some(meta.value()?.parse::<LitStr>()?.value());
            } else {
                return Err(
                    meta.error("unsupported `bidi` attribute (expected `method` or `response`)")
                );
            }
            Ok(())
        })?;
    }
    Ok((method, response))
}

fn expand_command(input: DeriveInput) -> syn::Result<TokenStream2> {
    let ident = input.ident.clone();
    let (method, response) = bidi_meta(&input.attrs)?;
    let method = method.ok_or_else(|| {
        syn::Error::new_spanned(&ident, "missing `#[bidi(method = \"module.command\")]`")
    })?;

    let response_ty: Type = match response {
        Some(text) => syn::parse_str(&text)?,
        None => syn::parse_quote!(crate::EmptyResult),
    };

    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const METHOD: &'static str = #method;
        }

        impl #impl_generics crate::BidiCommand for #ident #ty_generics #where_clause {
            const METHOD: &'static str = #method;
            type Response = #response_ty;
        }
    })
}

fn expand_event(input: DeriveInput) -> syn::Result<TokenStream2> {
    let ident = input.ident.clone();
    let (method, _) = bidi_meta(&input.attrs)?;
    let method = method.ok_or_else(|| {
        syn::Error::new_spanned(&ident, "missing `#[bidi(method = \"module.event\")]`")
    })?;

    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const METHOD: &'static str = #method;
        }

        impl #impl_generics crate::BidiEvent for #ident #ty_generics #where_clause {
            const METHOD: &'static str = #method;
        }
    })
}
