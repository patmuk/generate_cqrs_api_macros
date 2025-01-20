use proc_macro2::{Span, TokenStream};
use quote::format_ident;
use syn::{Ident, Path, PathArguments, Result, Type};

/// extracts the path from a type
pub(crate) fn get_path(tipe: &Type) -> Result<Path> {
    match tipe {
        syn::Type::Path(type_path) => Ok(type_path.path.to_owned()),
        _ => Err(syn::Error::new(Span::call_site(), "Not a struct type.")),
    }
}

/// converts a type to an ident, snake-casing it
/// e.g. Foo -> foo
/// e.g. Foo<Bar> -> foo_bar
/// e.g. Vec<(Foo, Bar)> -> vec_foo_bar
/// this is needed, as "Foo<Bar>" would be an invalid ident"
pub(crate) fn get_type_as_snake_case_ident(tipe: &Type) -> Result<Ident> {
    let type_string = get_type_as_string(tipe)?;
    let cleaned_type_string = type_string.replace(['>', ')', '[', ']', ' '], "");
    let replaced_type_string = cleaned_type_string.replace(['<', '(', ','], "_");
    Ok(format_ident!(
        "{}",
        stringcase::snake_case(&replaced_type_string)
    ))
}

/// converts a type to an ident, Capital snake-casing it
/// e.g. "Foo" -> Foo
/// e.g. "Foo<Bar>" -> FooBar
/// e.g. "Vec<(Foo, Bar)>" -> VecFooBar
/// this is needed, as "Foo<Bar>" would be an invalid ident"
pub(crate) fn get_type_as_capital_ident(tipe: &Type) -> Result<Ident> {
    let type_string = get_type_as_string(tipe)?;
    // replace all illegal characters
    let cleaned_type_string = type_string.replace(['<', '(', ',', '>', ')', '[', ']', ' '], "");
    Ok(format_ident!("{}", cleaned_type_string))
}

pub(crate) fn get_type_as_tokens(tipe: &Type) -> Result<TokenStream> {
    let type_string = get_type_as_string(tipe)?;
    Ok(type_string.parse().unwrap())
}

// todo fix this to have the proper output
/// converts a type into a string, e.g.
/// Foo<Bar> -> "Foo<Bar>"
pub(crate) fn get_type_as_string(tipe: &Type) -> Result<String> {
    match tipe {
        Type::Array(type_array) => Ok(format!("[{}]", get_type_as_string(&type_array.elem)?)),
        Type::Slice(type_slice) => Ok(format!("[{}]", get_type_as_string(&type_slice.elem)?)),
        Type::Group(type_group) => get_type_as_string(&type_group.elem),
        Type::Paren(type_paren) => Ok(format!("({})", get_type_as_string(&type_paren.elem)?)),
        Type::Ptr(type_ptr) => get_type_as_string(&type_ptr.elem),
        // Type::Reference(type_reference) => Ok(get_type_as_string(&type_reference.elem)? + "_"),
        Type::Reference(_type_reference) => Err(syn::Error::new(
            Span::call_site(),
            "References are not supported by FlutterRustBridge.",
        )),
        Type::Tuple(type_tuple) => {
            let elements: Result<Vec<String>> =
                type_tuple.elems.iter().map(get_type_as_string).collect();
            Ok(format!("({})", elements?.join(", ")))
        }
        Type::Path(type_path) => {
            let last_segment = type_path
                .path
                .segments
                .last()
                .expect("should be a type, but there was no last path segment!");

            match &last_segment.arguments {
                PathArguments::None => Ok(last_segment.ident.to_string()),
                PathArguments::AngleBracketed(angled_args) => {
                    let args: Result<Vec<String>> = angled_args
                        .args
                        .iter()
                        .map(|e| match e {
                            syn::GenericArgument::Type(t) => get_type_as_string(t),
                            _ => Err(syn::Error::new(
                                Span::call_site(),
                                "Not a supported type in this angle_bracketed_argument.",
                            )),
                        })
                        .collect();
                    Ok(format!("{}<{}>", last_segment.ident, args?.join(", ")))
                }
                PathArguments::Parenthesized(_) => Err(syn::Error::new(
                    Span::call_site(),
                    "Parenthesized types are not supported.",
                )),
            }
        }
        _ => Err(syn::Error::new(Span::call_site(), "Not a supported type.")),
    }
}

#[cfg(test)]
mod tests {
    use syn::{Ident, Result, Type};

    use super::*;

    struct TypeTestBuilder {
        input: Type,
    }
    fn given(input: &str) -> TypeTestBuilder {
        TypeTestBuilder {
            input: syn::parse_str::<Type>(input).expect("test input should be parsable"),
        }
    }
    impl TypeTestBuilder {
        fn when_i_call<T, F>(self, f: F) -> TypeTestAssertion<T>
        where
            F: Fn(&Type) -> Result<T>,
        {
            let result = f(&self.input).expect("function call should succeed");
            TypeTestAssertion { result }
        }
    }
    struct TypeTestAssertion<T> {
        result: T,
    }
    impl TypeTestAssertion<String> {
        fn then(self, expected: &str) {
            assert_eq!(self.result, expected);
        }
    }
    impl TypeTestAssertion<Ident> {
        fn then(self, expected: &str) {
            assert_eq!(self.result.to_string(), expected);
        }
    }

    #[test]
    fn test_type_as_string_type() {
        given("Foo").when_i_call(get_type_as_string).then("Foo");
    }
    #[test]
    fn test_type_as_capital_ident_type() {
        given("Foo")
            .when_i_call(get_type_as_capital_ident)
            .then("Foo");
    }
    #[test]
    fn test_type_as_snake_case_ident_type() {
        given("Foo")
            .when_i_call(get_type_as_snake_case_ident)
            .then("foo");
    }

    #[test]
    fn test_type_as_string_type_with_parameter() {
        given("Foo<Bar>")
            .when_i_call(get_type_as_string)
            .then("Foo<Bar>");
    }
    #[test]
    fn test_type_as_capital_ident_type_with_parameter() {
        given("Foo<Bar>")
            .when_i_call(get_type_as_capital_ident)
            .then("FooBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_type_with_parameter() {
        given("Foo<Bar>")
            .when_i_call(get_type_as_snake_case_ident)
            .then("foo_bar");
    }

    #[test]
    fn test_type_as_string_type_vec_type_with_type_with_parameter() {
        given("Vec<Foo<Bar>>")
            .when_i_call(get_type_as_string)
            .then("Vec<Foo<Bar>>");
    }
    #[test]
    fn test_type_as_capital_ident_type_vec_type_with_type_with_parameter() {
        given("Vec<Foo<Bar>>")
            .when_i_call(get_type_as_capital_ident)
            .then("VecFooBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_type_vec_type_with_type_with_parameter() {
        given("Vec<Foo<Bar>>")
            .when_i_call(get_type_as_snake_case_ident)
            .then("vec_foo_bar");
    }

    #[test]
    fn test_type_as_string_two_tuple_type() {
        given("(Foo, Bar)")
            .when_i_call(get_type_as_string)
            .then("(Foo, Bar)");
    }
    #[test]
    fn test_type_as_capital_ident_two_tuple_type() {
        given("(Foo, Bar)")
            .when_i_call(get_type_as_capital_ident)
            .then("FooBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_two_tuple_type() {
        given("(Foo, Bar)")
            .when_i_call(get_type_as_snake_case_ident)
            .then("foo_bar");
    }

    #[test]
    fn test_type_as_string_slice_type() {
        given("[Foo]").when_i_call(get_type_as_string).then("[Foo]");
    }
    #[test]
    fn test_type_as_capital_ident_slice_type() {
        given("[Foo]")
            .when_i_call(get_type_as_capital_ident)
            .then("Foo");
    }
    #[test]
    fn test_type_as_snake_case_ident_slice_type() {
        given("[Foo]")
            .when_i_call(get_type_as_snake_case_ident)
            .then("foo");
    }

    #[test]
    fn test_type_as_string_two_tuple_vec_type() {
        given("(Vec<Foo>, Bar)")
            .when_i_call(get_type_as_string)
            .then("(Vec<Foo>, Bar)");
    }
    #[test]
    fn test_type_as_capital_ident_two_tuple_vec_type() {
        given("(Vec<Foo>, Bar)")
            .when_i_call(get_type_as_capital_ident)
            .then("VecFooBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_two_tuple_vec_type() {
        given("(Vec<Foo>, Bar)")
            .when_i_call(get_type_as_snake_case_ident)
            .then("vec_foo_bar");
    }

    #[test]
    fn test_type_as_string_two_tuple_two_vec_type() {
        given("(Vec<Foo>, Vec<Bar>)")
            .when_i_call(get_type_as_string)
            .then("(Vec<Foo>, Vec<Bar>)");
    }
    #[test]
    fn test_type_as_capital_ident_two_tuple_two_vec_type() {
        given("(Vec<Foo>, Vec<Bar>)")
            .when_i_call(get_type_as_capital_ident)
            .then("VecFooVecBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_two_tuple_two_vec_type() {
        given("(Vec<Foo>, Vec<Bar>)")
            .when_i_call(get_type_as_snake_case_ident)
            .then("vec_foo_vec_bar");
    }

    #[test]
    fn test_type_as_string_tuple_in_vec_type() {
        given("Vec<(Foo, Bar)>")
            .when_i_call(get_type_as_string)
            .then("Vec<(Foo, Bar)>");
    }
    #[test]
    fn test_type_as_capital_ident_tuple_in_vec_type() {
        given("Vec<(Foo, Bar)>")
            .when_i_call(get_type_as_capital_ident)
            .then("VecFooBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_tuple_in_vec_type() {
        given("Vec<(Foo, Bar)>")
            .when_i_call(get_type_as_snake_case_ident)
            .then("vec_foo_bar");
    }

    #[test]
    fn test_type_as_string_two_tuple_vec_type_with_type() {
        given("(Vec<Foo<Bar>>, Vec<Bar>)")
            .when_i_call(get_type_as_string)
            .then("(Vec<Foo<Bar>>, Vec<Bar>)");
    }
    #[test]
    fn test_type_as_capital_ident_two_tuple_vec_type_with_type() {
        given("(Vec<Foo<Bar>>, Vec<Bar>)")
            .when_i_call(get_type_as_capital_ident)
            .then("VecFooBarVecBar");
    }
    #[test]
    fn test_type_as_snake_case_ident_two_tuple_vec_type_with_type() {
        given("(Vec<Foo<Bar>>, Vec<Bar>)")
            .when_i_call(get_type_as_snake_case_ident)
            .then("vec_foo_bar_vec_bar");
    }
}
