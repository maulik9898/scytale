use axum::{
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response, Html},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../app/dist"]
struct Asset;

pub struct StaticFile<T>(pub T);

pub struct SpaController;

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => Asset::get("index.html")
                .map(|content| {
                    let body = boxed(Full::from(content.data));
                    let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                    Response::builder()
                        .header(header::CONTENT_TYPE, mime.as_ref())
                        .body(body)
                        .unwrap()
                })
                .unwrap()
        }
    }
}

impl SpaController {
    pub async fn index_handler() -> impl IntoResponse {
        SpaController::static_handler("/index.html".parse::<Uri>().unwrap()).await
    }

    pub async fn static_handler(uri: Uri) -> impl IntoResponse {
        let mut path = uri.path().trim_start_matches('/').to_string();

        if path.starts_with("dist/") {
            path = path.replace("dist/", "");
        }

        StaticFile(path)
    }

    pub async fn not_found() -> Html<&'static str> {
        Html("<h1>404</h1><p>Not Found</p>")
    }
}
