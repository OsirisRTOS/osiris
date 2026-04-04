use syn::DeriveInput;

pub fn derive_fmt(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Check if the env variable "OSIRIS_DEBUG_DEFMT" is set. If it is, generate a defmt::Format implementation. Otherwise, generate a Debug implementation.
    if std::env::var("OSIRIS_DEBUG_DEFMT").is_ok() {
        Ok(derive_fmt_defmt(input))
    } else {
        Ok(derive_fmt_debug(input))
    }
}

fn derive_fmt_defmt(input: &DeriveInput) -> proc_macro2::TokenStream {
    quote::quote! {
        #[derive(defmt::Format)]
        #input
    }
}

fn derive_fmt_debug(input: &DeriveInput) -> proc_macro2::TokenStream {
    quote::quote! {
        #[derive(core::fmt::Debug)]
        #input
    }
}
