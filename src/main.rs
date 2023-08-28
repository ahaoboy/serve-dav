#![feature(absolute_path)]

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use dav_server::actix::*;
use std::io;
use tokio;

use dav_server::{fakels::FakeLs, localfs::LocalFs, DavConfig, DavHandler};

pub async fn dav_handler(req: DavRequest, davhandler: web::Data<DavHandler>) -> DavResponse {
    if let Some(prefix) = req.prefix() {
        let config = DavConfig::new().strip_prefix(prefix);
        davhandler.handle_with(config, req.request).await.into()
    } else {
        davhandler.handle(req.request).await.into()
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Cli {
    /// user name
    #[arg(short, long, default_value_t = String::from("root"))]
    user: String,

    /// password
    #[arg(short, long, default_value_t = String::from("root"))]
    pwd: String,

    /// port
    #[arg(long, default_value_t = 9423)]
    port: u32,

    file_or_dir: String,
}

pub(crate) fn get_server(cli: Cli) -> io::Result<Server> {
    println!("input args: {:?}", cli);
    let Cli {
        file_or_dir,
        pwd,
        port,
        user,
    } = cli;

    let path = std::path::Path::new(&file_or_dir);
    let path = std::path::absolute(path).unwrap();

    let fs = if path.is_dir() {
        LocalFs::new(path, false, false, false)
    } else {
        let parent = path.parent().unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        LocalFs::new_with_includes(parent, false, false, false, vec![name])

    };

    let dav_server = DavHandler::builder()
        .filesystem(fs)
        .locksystem(FakeLs::new())
        .build_handler();

    let ip = "0.0.0.0";
    let addr = format!("{}:{}", ip, port);

    println!("serve-dav: http://{} , user:{} , pwd: {}", addr, user, pwd);

    return Ok(HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dav_server.clone()))
            .service(web::resource("/{tail:.*}").to(dav_handler))
    })
    .bind(addr)?
    .run());
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let cli = Cli::parse();
    let server = get_server(cli)?;
    server.await
}
