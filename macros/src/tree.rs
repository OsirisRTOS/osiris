use quote::quote;
use syn::{
    spanned::Spanned, Data, DeriveInput, Error, Fields, Path,
};

pub fn derive_tagged_links(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = match &input.data {
        Data::Struct(ds) => match &ds.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(Error::new(
                    ds.fields.span(),
                    "TaggedLinks only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new(
                input.span(),
                "TaggedLinks can only be derived for structs",
            ))
        }
    };

    let rbtree_impls = impl_rbtree(input, fields)?;

    Ok(quote! {
        #rbtree_impls
    })
}

fn impl_rbtree(input: &DeriveInput, fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = &input.ident;
    let generics = &input.generics;

    let mut impls = Vec::new();

    for field in fields {
        let Some(field_ident) = field.ident.clone() else { continue };

        if let (Some(tag_path), Some(idx_path)) = find_rbtree(&field.attrs)? {
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let impl_block = quote! {
                impl #impl_generics crate::mem::rbtree::Linkable<#tag_path, #idx_path> for #struct_ident #ty_generics #where_clause {
                    #[inline]
                    fn links(&self) -> &crate::mem::rbtree::Links<#tag_path, #idx_path> {
                        &self.#field_ident
                    }
                    #[inline]
                    fn links_mut(&mut self) -> &mut crate::mem::rbtree::Links<#tag_path, #idx_path> {
                        &mut self.#field_ident
                    }
                }
            };

            impls.push(impl_block);
        }
    }

    if impls.is_empty() {
        return Err(Error::new(
            input.span(),
            "No fields found with #[rbtree(tag = ..., idx = ...)] attribute",
        ));
    }

    Ok(quote! { #(#impls)* })
}

fn find_rbtree(attrs: &[syn::Attribute]) -> syn::Result<(Option<Path>, Option<Path>)> {
    for attr in attrs {
        if !attr.path().is_ident("rbtree") {
            continue;
        }

        let mut tag: Option<Path> = None;
        let mut idx: Option<Path> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value = meta.value()?; // expects '='
                let p: Path = value.parse()?;
                tag = Some(p);
                Ok(())
            } else if meta.path.is_ident("idx") {
                let value = meta.value()?; // expects '='
                let p: Path = value.parse()?;
                idx = Some(p);
                Ok(())
            } else {
                Err(meta.error("expected `tag = SomePath` or `idx = SomePath`"))
            }
        })?;

        return Ok((tag, idx));
    }
    Ok((None, None))
}
