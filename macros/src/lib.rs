use syn::parse_macro_input;

mod tree;
mod syscall;
mod logging;

#[proc_macro_derive(TaggedLinks, attributes(rbtree, list))]
pub fn derive_tagged_links(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match tree::derive_tagged_links(&input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error(),
    }.into()
}

#[proc_macro_attribute]
pub fn fmt(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match logging::derive_fmt(&input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error(),
    }.into()
}

#[proc_macro_attribute]
pub fn app_main(input: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemFn);
    let block = &item.block;

    let expanded = quote::quote! {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        extern "C" fn main() {
            osiris::hal::asm::startup_trampoline!();
        }

        #[cfg(freestanding)]
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            osiris::panic(info);
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn app_main() -> () {
            #block
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn syscall_handler(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut num = 0;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("num") {
            num = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    });

    parse_macro_input!(attr with parser);

    let item = syn::parse_macro_input!(item as syn::ItemFn);
    syscall::syscall_handler_fn(&item).into()
}


