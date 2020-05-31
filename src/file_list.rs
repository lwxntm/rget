use crate::ip;
use actix_web::{error, web, Error, HttpResponse, Responder};
use actix_web::{get, HttpRequest};
use chrono::NaiveDateTime;
use download_rs::async_download::Download;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::read_dir;

pub(crate) const FS_PATH: &str = "static";

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
pub(crate) async fn del_file(info: web::Path<String>) -> impl Responder {
    println!("{}", (FS_PATH.to_owned() + "/" + info.as_str()));
    if let Err(e) = fs::remove_file(FS_PATH.to_owned() + "/" + info.as_str()) {
        e.to_string()
    } else {
        "Success".to_string()
    }
}

pub(crate) async fn list(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    req: HttpRequest,
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

    let ci_host = ip::handle_ip(req).await;
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
        ctx.insert("ip", &ci_host);
        tmpl.render("list.html", &ctx)
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}
