use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use dav_server::actix::*;
use dav_server::{localfs::LocalFs, DavConfig, DavHandler};
use fast_qr::{QRBuilder, ECL};
use std::io;
use std::path::PathBuf;
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
    #[arg(long, default_value_t = 9421)]
    port: u16,

    /// host
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    host: String,

    #[clap(default_value_t = String::from("."))]
    file_or_dir: String,
}

const TMP: &str = "__dav_tmp__";

fn get_tmp_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let temp_dir = exe.parent().unwrap();
    temp_dir.join(TMP)
}
fn init_dir(tmp_dir: &PathBuf, p: &PathBuf) {
    if tmp_dir.exists() {
        std::fs::remove_dir_all(tmp_dir).unwrap();
    }
    std::fs::create_dir(tmp_dir).unwrap();
    let link_path = tmp_dir.join(p.file_name().unwrap());
    std::os::windows::fs::symlink_file(p, link_path).unwrap();
}

pub(crate) fn get_server(cli: Cli) -> io::Result<Server> {
    let Cli {
        file_or_dir,
        pwd: _,
        port,
        user: _,
        host,
    } = cli;

    let path = std::path::Path::new(&file_or_dir);
    let port = find_port::find_port("127.0.0.1", port).expect("");

    if !path.exists() {
        panic!("file_or_dir not found: {}", path.to_string_lossy());
    }

    let path = path.canonicalize().unwrap();

    let fs = if path.is_dir() {
        LocalFs::new(path, false, false, false)
    } else {
        let tmp_path = get_tmp_dir();
        init_dir(&tmp_path, &path);
        LocalFs::new(tmp_path, false, false, false)
    };

    let dav_server = DavHandler::builder()
        .hide_symlinks(false)
        .filesystem(fs)
        .build_handler();

    let addr = format!("{}:{}", host, port);

    let local_ip = local_ip_address::local_ip().unwrap();

    let s1 = format!("http://{}:{}/", "localhost", port);
    let s2 = format!("http://{}:{}/", local_ip, port);
    let s3 = format!("http://{}:{}/", host, port);

    println!("serve-dav:\n{}\n{}\n{}", s1, s2, s3);
    let qrcode = QRBuilder::new(s2).ecl(ECL::H).build().unwrap();
    qrcode.print();

    Ok(HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dav_server.clone()))
            .service(web::resource("/{tail:.*}").to(dav_handler))
    })
    .bind(addr)?
    .run())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let cli = Cli::parse();
    let server = get_server(cli)?;
    server.await
}
