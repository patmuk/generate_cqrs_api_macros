use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_str, File};

pub(crate) fn generate_use_statement(base_path: &str, tipe: &str) -> TokenStream {
    let use_statement_string = format!("use {}::{};", base_path, tipe);
    // parse with syn
    let use_statement_parsed =
        parse_str::<File>(&use_statement_string).expect("error parsing use statement");
    let use_statement = use_statement_parsed
        .items
        .first()
        .expect("first item was not the use statement");
    quote! {
        #use_statement
    }
}
