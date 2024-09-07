use std::{fmt::Display, fs::read_dir, io};
use url_escape::{encode, NON_ALPHANUMERIC};

use super::request::{HttpRequest, Version};

#[derive(Debug)]
pub struct HttpResponse {
    version: Version,
    status: ResponseStatus,
    content_length: usize,
    accept_ranges: AcceptRanges,
    pub response_body: Vec<u8>,
    pub current_path: String,
}

impl HttpResponse {
    pub fn new(request: &HttpRequest) -> io::Result<HttpResponse> {
        let version = Version::V1_1;
        let mut status = ResponseStatus::NotFound;
        let mut content_length = 0;
        let mut accept_ranges = AcceptRanges::None;
        let current_path = request.resource.path.clone();
        let mut response_body = Vec::new();
        let server_root_path = std::env::current_dir()?;
        let resource = request.resource.path.clone();
        let mut new_path = server_root_path.join(resource);
        let server_root_len = server_root_path.canonicalize()?.components().count();
        let new_path_len = new_path.canonicalize()?.components().count();

        // check condition to prevent backtracking
        if !(server_root_len <= new_path_len) {
            new_path = server_root_path;
        }

        let display_path = format!("/{}", request.resource.path.clone());

        let begin_html = r#"
        <!DOCTYPE html> 
        <html> 
        <head> 
            <meta charset="utf-8"> 
        </head> 
        <body>
        "#
        .to_string();

        let current_dir_header = format!(
            "<h1>Currently in: {}</h1><br><hr><a href=\"../\">Go up</a>",
            display_path
        );

        if new_path.exists() {
            if new_path.is_file() {
                let content = std::fs::read(&new_path)?; // Read as bytes
                content_length = content.len();
                status = ResponseStatus::OK;
                accept_ranges = AcceptRanges::Bytes;

                // Infer the file type to set the appropriate Content-Type header
                let content_type = if let Some(info) = infer::get(&content) {
                    format!("Content-Type: {}", info.mime_type())
                } else {
                    "Content-Type: text/plain".to_string()
                };

                let header = format!(
                    "{} {}\n{}\ncontent-length: {}\n{}\r\n\r\n",
                    version, status, accept_ranges, content_length, content_type
                );
                response_body.extend_from_slice(&header.as_bytes());
                response_body.extend_from_slice(&content);
            } else if new_path.is_dir() {
                let mut directory_listing = String::new();
                directory_listing.push_str(&begin_html);
                directory_listing.push_str(&current_dir_header);
                directory_listing.push_str("<ul>");

                let files = read_dir(new_path).expect("failed to read file");

                for file in files {
                    let path = file.unwrap().path();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let display_name = if path.is_dir() {
                        format!("{}/", file_name)
                    } else {
                        file_name.to_string()
                    };

                    // Encode the file path to make it safe for URLs
                    let path_to_encode = format!("{current_path}/{file_name}");
                    let encoded_path = encode(&path_to_encode, NON_ALPHANUMERIC).to_string();
                    println!("Encoded path: {}", encoded_path);
                    directory_listing.push_str(&format!(
                        "<li><a href=\"{encoded_path}\">{display_name}</a></li>"
                    ));
                }

                directory_listing.push_str("</ul></body></html>");
                content_length = directory_listing.len();
                status = ResponseStatus::OK;

                let content = format!(
                    "{} {}\n{}\ncontent-length: {}\r\n\r\n{}",
                    version, status, accept_ranges, content_length, directory_listing
                );
                response_body.extend_from_slice(&content.as_bytes());
            } else {
                let not_found_page = r#" 
                                <!DOCTYPE html> 
                                <html> 
                                    <head> 
                                        <meta charset="utf-8"> 
                                    </head> 
                                    <body>
                                        <h1>404 NOT FOUND</h1>
                                    </body>
                                </html"#;
                content_length = not_found_page.len();
                let content = format!(
                    "{} {}\n{}\ncontent-length: {}\r\n\r\n{}",
                    version, status, accept_ranges, content_length, not_found_page
                );
                response_body.extend_from_slice(&content.as_bytes());
            }
        }

        Ok(HttpResponse {
            version,
            status,
            content_length,
            accept_ranges,
            response_body,
            current_path,
        })
    }
}

#[derive(Debug)]
enum ResponseStatus {
    OK = 200,
    NotFound = 404,
}

impl Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ResponseStatus::OK => "200 OK",
            ResponseStatus::NotFound => "404 NOT FOUND",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug)]
enum AcceptRanges {
    Bytes,
    None,
}

impl Display for AcceptRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            AcceptRanges::Bytes => "accept-ranges: bytes",
            AcceptRanges::None => "accept-ranges: none",
        };
        write!(f, "{}", msg)
    }
}
