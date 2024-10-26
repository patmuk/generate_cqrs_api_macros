use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_str, File};

pub(crate) fn generate_use_statement(base_path: &str, tipe: &str) -> TokenStream {
    if base_path.is_empty() {
        panic!("base path can't be empty!")
    }
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

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::utils::generate_use_statement::generate_use_statement;

    #[test]
    #[should_panic = "base path can't be empty!"]
    fn generate_use_statement_test_zero_level() {
        let expected = quote! {
            use crate::MyStruct;
        };
        let result = generate_use_statement("", "MyStruct");

        assert_eq!(expected.to_string(), result.to_string())
    }
    #[test]
    fn generate_use_statement_test_one_level() {
        let expected = quote! {
            use crate::module::MyStruct;
        };
        let result = generate_use_statement("crate::module", "MyStruct");

        assert_eq!(expected.to_string(), result.to_string())
    }
    #[test]
    fn generate_use_statement_test_two_level() {
        let expected = quote! {
            use crate::module::domain::MyStruct;
        };
        let result = generate_use_statement("crate::module::domain", "MyStruct");

        assert_eq!(expected.to_string(), result.to_string())
    }
    #[test]
    fn generate_use_statement_test_three_level() {
        let expected = quote! {
            use crate::module::domain::my::MyStruct;
        };
        let result = generate_use_statement("crate::module::domain::my", "MyStruct");

        assert_eq!(expected.to_string(), result.to_string())
    }
}
