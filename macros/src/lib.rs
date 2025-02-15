extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::format_ident;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    // This macro should be used to annotate a service struct.
    let item = syn::parse_macro_input!(item as syn::ItemStruct);

    let service_name = item.ident.clone();

    let mut mem_size: usize = 0;
    let mut stack_size: usize = 0;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("mem_size") {
            mem_size = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("stack_size") {
            stack_size = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    });

    parse_macro_input!(attr with parser);

    let mem_size_ident = format_ident!("TASK_{}_MEM_SIZE", service_name.to_string().to_uppercase());
    let stack_size_ident = format_ident!(
        "TASK_{}_STACK_SIZE",
        service_name.to_string().to_uppercase()
    );

    let expanded = quote::quote! {
        const #mem_size_ident: usize = #mem_size;
        const #stack_size_ident: usize = #stack_size;
        #item

        impl #service_name {
            pub fn task_desc() -> crate::sched::task::TaskDesc {
                crate::sched::task::TaskDesc {
                    mem_size: #mem_size_ident,
                    stack_size: #stack_size_ident,
                }
            }
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn syscall_handler(attr: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);

    let name = item.sig.ident.to_string().to_uppercase();

    let args_name = format_ident!("{}_ARGS", name);
    let num_name = format_ident!("{}_NUM", name);

    let mut args = 0;
    let mut num = 0;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("args") {
            args = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("num") {
            num = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    });

    parse_macro_input!(attr with parser);

    let expanded = quote::quote! {
        pub const #args_name: i32 = #args;
        pub const #num_name: u8 = #num as u8;
        #item
    };

    expanded.into()
}
