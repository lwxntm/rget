use std::collections::HashMap;

use actix_files::Files;
use actix_http::{body::Body, Response};
use actix_web::dev::ServiceResponse;
use actix_web::get;
use actix_web::http::StatusCode;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{error, middleware, web, App, Error, HttpResponse, HttpServer, Responder, Result};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs::read_dir;

//templates
use tera::Tera;

//download
use download_rs::async_download::Download;
use std::fs;

const FS_PATH: &str = "static";

// add below
#[derive(Serialize, Deserialize, Debug)]
struct FileList {
    file_name: String,
    mod_time: String,
    size: String,
}

fn size_trans(mut size: u64) -> String {
    let mut step = 0;
    while size > 1024 {
        size /= 1024;
        step += 1;
    }
    match step {
        0 => format!("{}  B", size),
        1 => format!("{} KB", size),
        2 => format!("{} MB", size),
        3 => format!("{} GB", size),
        4 => format!("{} TB", size),
        _ => "WTF".to_string(),
    }
}

#[get("delete/{file_name}")]
async fn del_file(info: web::Path<String>) -> impl Responder {
    println!("{}", (FS_PATH.to_owned() + "/" + info.as_str()));
    if let Ok(e) = fs::remove_file(FS_PATH.to_owned() + "/" + info.as_str()) {
        "Ok".to_string()
    } else {
        "Err".to_string()
    }
}

async fn list(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    if let Some(url) = query.get("link") {
        let filename =
            FS_PATH.to_owned() + "/" + url.split('/').last().unwrap().split('%').last().unwrap();
        let download = Download::new(url, Some(filename.as_ref()), None);
        match download.download_async().await {
            Err(e) => println!("error: {}", e.to_string()),
            Ok(()) => println!("ok"),
        }
    }
    let s = {
        let mut v1 = vec![];
        let dir = read_dir(FS_PATH)?;
        for d in dir {
            if let Ok(r) = d {
                let metadata = r.metadata().unwrap();
                let mtime = filetime::FileTime::from_last_modification_time(&metadata);
                extern crate chrono;
                use chrono::{DateTime, Utc};
                let naive = NaiveDateTime::from_timestamp(mtime.seconds(), 0);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                let datetime = datetime.format("%Y-%m-%d %H:%M:%S");
                let file_entry = FileList {
                    file_name: r.file_name().into_string().unwrap(),
                    mod_time: datetime.to_string(),
                    size: size_trans(r.metadata().unwrap().len()),
                };
                v1.push(file_entry)
            }
        }
        let mut ctx = tera::Context::new();
        ctx.insert("file_names", &v1);
        tmpl.render("list.html", &ctx)
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        // let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();
        let tera = Tera::new("templates/**/*").unwrap();

        App::new()
            .data(tera)
            .wrap(middleware::Logger::default()) // enable logger
            .service(web::resource("/").route(web::get().to(list)))
            .service(Files::new("/dl", FS_PATH).show_files_listing())
            .service(del_file)
            .service(web::scope("").wrap(error_handlers()))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

// Custom error handlers, to return HTML responses when an error occurs.
fn error_handlers() -> ErrorHandlers<Body> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

// Error handler for a 404 Page not found error.
fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> Response<Body> {
    let request = res.request();

    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |e: &str| {
        Response::build(res.status())
            .content_type("text/plain")
            .body(e.to_string())
    };

    let tera = request.app_data::<web::Data<Tera>>().map(|t| t.get_ref());
    match tera {
        Some(tera) => {
            let mut context = tera::Context::new();
            context.insert("error", error);
            context.insert("status_code", res.status().as_str());
            let body = tera.render("error.html", &context);

            match body {
                Ok(body) => Response::build(res.status())
                    .content_type("text/html")
                    .body(body),
                Err(_) => fallback(error),
            }
        }
        None => fallback(error),
    }
}
