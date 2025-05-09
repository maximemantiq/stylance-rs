#![cfg_attr(feature = "nightly", feature(proc_macro_span))]

use std::{env, path::Path};

use anyhow::Context as _;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, LitStr};

fn try_import_style_classes_with_path(
    manifest_path: &Path,
    file_path: &Path,
    identifier_span: Span,
) -> anyhow::Result<TokenStream> {
    let config = stylance_core::load_config(manifest_path)?;
    let (_, classes) = stylance_core::get_classes(manifest_path, file_path, &config)?;

    let binding = file_path.canonicalize().unwrap();
    let full_path = binding.to_string_lossy();

    let identifiers = classes
        .iter()
        .map(|class| Ident::new(&class.original_name.replace('-', "_"), identifier_span))
        .collect::<Vec<_>>();

    let output_fields = classes.iter().zip(identifiers).map(|(class, class_ident)| {
        let class_str = &class.hashed_name;
        quote_spanned!(identifier_span =>
            #[allow(non_upper_case_globals)]
            pub const #class_ident: &str = #class_str;
        )
    });

    Ok(quote! {
        const _ : &[u8] = include_bytes!(#full_path);
        #(#output_fields )*
    }
    .into())
}

fn try_import_style_classes(input: &LitStr) -> anyhow::Result<TokenStream> {
    let manifest_dir_env =
        env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR env var not found")?;
    let manifest_path = Path::new(&manifest_dir_env);
    let file_path = manifest_path.join(Path::new(&input.value()));

    try_import_style_classes_with_path(manifest_path, &file_path, input.span())
}

#[proc_macro]
pub fn import_style_classes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);

    match try_import_style_classes(&input) {
        Ok(ts) => ts,
        Err(err) => syn::Error::new_spanned(&input, err.to_string())
            .to_compile_error()
            .into(),
    }
}

#[cfg(feature = "nightly")]
fn try_import_style_classes_rel(input: &LitStr) -> anyhow::Result<TokenStream> {
    let manifest_dir_env =
        env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR env var not found")?;
    let manifest_path = Path::new(&manifest_dir_env);

    let Some(source_path) = proc_macro::Span::call_site().source().local_file() else {
        // It would make sense to error here but currently rust analyzer is returning None when
        // the normal build would return the path.
        // For this reason we bail silently creating no code.
        return Ok(TokenStream::new());
    };

    let css_path = source_path
        .parent()
        .expect("Macro source path should have a parent dir")
        .join(input.value());

    try_import_style_classes_with_path(manifest_path, &css_path, input.span())
}

#[cfg(feature = "nightly")]
#[proc_macro]
pub fn import_style_classes_rel(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);

    match try_import_style_classes_rel(&input) {
        Ok(ts) => ts,
        Err(err) => syn::Error::new_spanned(&input, err.to_string())
            .to_compile_error()
            .into(),
    }
}
