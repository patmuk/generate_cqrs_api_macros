use log::{debug, info, trace};
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::Result;

use crate::{
    generate_api_macro_impl::{ParsedFiles, SourceCode},
    parsing::file_location_2_base_path::file_location_2_base_path,
};

/// extracts file locations from a TokenStream
pub(crate) fn tokens_2_file_locations(file_paths: TokenStream) -> Result<Vec<String>> {
    let file_paths = file_paths
        .into_iter()
        .filter_map(|token| match token {
            TokenTree::Literal(literal) => Some(literal),
            _ => None,
        })
        .map(|literal| {
            // if to_string() breaks, parse it with https://github.com/LukasKalbertodt/litrs/
            let cleaned = literal.to_string();
            cleaned[1..cleaned.len() - 1].to_string()
        })
        .collect::<Vec<String>>();
    info!("Parsing content of: {:#?}", file_paths);
    Ok(file_paths)
}

/// reads multiple rust files, generates use statements for them and returns their content in one concatenated String
pub(crate) fn read_rust_file_content(
    file_paths: Vec<String>,
) -> Result<Vec<ParsedFiles>> {
    file_paths.iter().map(|file_path| { 
        // Attempt to read each file's content as a string.
        std::fs::read_to_string(file_path)           
            .map_err(|io_error| {
                let current_dir = std::env::current_dir();
                match current_dir {
                    Ok(cwd) => syn::Error::new(
                        Span::call_site(),
                        format!(
                            "Error loading the given file: {io_error}\nLooked in: {cwd:?} / \"{file_path}\"\nFile paths need to start from the project root."
                        ),
                    ),
                    Err(cwd_io_error) => syn::Error::new(
                        Span::call_site(),
                        format!(
                            "Error reading current directory: {cwd_io_error}\nWhile loading the file: {io_error}\nFile paths need to start from the project root."
                        ),
                    ),
                }
            }).map(|source|{            
                trace!("File content:\n{}", source);
                let base_path = file_location_2_base_path(file_path);  // Assuming this function exists
                debug!("Base path is: {:#?}", base_path);
                ParsedFiles{ base_path, source_code: SourceCode(source)}
            })
        }).collect()

}

#[cfg(test)]
mod tests {
    use crate::parsing::read_rust_files::tokens_2_file_locations;
    use quote::quote;

    #[test]
    fn parse_one_filepath() {
        let input = quote! {"tests/good_source_file/mod.rs"};
        assert_eq!(
            vec!["tests/good_source_file/mod.rs"],
            tokens_2_file_locations(input).unwrap()
        );
    }
    #[test]
    fn parse_two_filepaths() {
        let input = quote! {"tests/good_source_file/mod.rs", "tests/second_model_file/mod.rs"};
        assert_eq!(
            vec![
                "tests/good_source_file/mod.rs",
                "tests/second_model_file/mod.rs"
            ],
            tokens_2_file_locations(input).unwrap()
        );
    }
    #[test]
    fn parse_three_filepaths() {
        let input = quote! {"tests/good_source_file/mod.rs", "tests/second_model_file/mod.rs", "tests/third_model_file/mod.rs"};
        assert_eq!(
            vec![
                "tests/good_source_file/mod.rs",
                "tests/second_model_file/mod.rs",
                "tests/third_model_file/mod.rs"
            ],
            tokens_2_file_locations(input).unwrap()
        );
    }
}
