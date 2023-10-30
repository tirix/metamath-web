mod statement;
#[cfg(feature = "sts")]
mod sts;
#[cfg(feature = "sts")]
mod sts_parser;
mod toc;
mod uni;

use crate::statement::Renderer;
use clap::crate_version;
use clap::App as ClapApp;
use clap::Arg;
use clap::ArgMatches;
use metamath_knife::database::DbOptions;
use metamath_knife::Database;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use warp::reject::Rejection;
use warp::Filter;

#[cfg(feature = "sts")]
use sts_parser::parse_sts;

fn positive_integer(val: &str) -> Result<(), String> {
    u32::from_str(val).map(|_| ()).map_err(|e| format!("{}", e))
}

fn command_args() -> ArgMatches {
    ClapApp::new("metamath-web")
        .version(crate_version!())
        .about("A web server providing Metamath pages")
        .arg(
            Arg::new("database")
                .help("Database file to load")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("address")
                .help("Address to serve")
                .default_value("0.0.0.0")
                .long("address")
                .short('a'),
        )
        .arg(
            Arg::new("port")
                .help("Port to listen to")
                .default_value("3030")
                .long("port")
                .short('p'),
        )
        .arg(
            Arg::new("jobs")
                .help("Number of threads to use for startup parsing")
                .long("jobs")
                .short('j')
                .takes_value(true)
                .validator(positive_integer),
        )
        .arg(
            Arg::new("bib_file")
                .help("Index file, which includes the bibliography")
                .long("bib")
                .short('b')
                .takes_value(true),
        )
        .arg(
            Arg::new("check_sts")
                .help(
                    "Check that all constructs defined in the database are covered by the STS file",
                )
                .long("check-sts")
                .short('S'),
        )
        .get_matches()
}

fn build_db(args: &ArgMatches) -> Result<Database, String> {
    let job_count =
        usize::from_str(args.value_of("jobs").unwrap_or("8")).expect("validator should check this");
    let options = DbOptions {
        autosplit: false,
        incremental: true,
        jobs: job_count,
        ..Default::default()
    };
    let mut db = Database::new(options);
    let data: Vec<(String, Vec<u8>)> = Vec::new();
    let start = args
        .value_of("database")
        .map(|x| x.to_owned())
        .unwrap_or_else(|| data[0].0.clone());
    println!("Starting up...");
    db.parse(start, data);
    db.scope_pass();
    let diag = db.diag_notations();
    if !diag.is_empty() {
        return Err(format!("{:?}", diag));
    }
    db.typesetting_pass();
    db.grammar_pass();
    db.stmt_parse_pass();
    db.outline_pass();
    println!("Ready.");
    Ok(db)
}

fn with_renderer(
    renderer: Renderer,
) -> impl Filter<Extract = (Renderer,), Error = Infallible> + Clone {
    warp::any().map(move || renderer.clone())
}

pub async fn get_theorem(
    explorer: String,
    label: String,
    renderer: Renderer,
) -> Result<impl warp::Reply, Rejection> {
    let label = label.replace(".html", "");
    match renderer.render_statement(explorer, label) {
        Some(html) => Ok(warp::reply::html(html)),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn get_toc(
    explorer: String,
    query: HashMap<String, String>,
    renderer: Renderer,
) -> Result<impl warp::Reply, Rejection> {
    let chapter_ref: usize = query.get("ref").map_or(Ok(0), |c| c.parse()).unwrap_or(0);
    match renderer.render_toc(explorer, chapter_ref) {
        Some(html) => Ok(warp::reply::html(html)),
        None => Err(warp::reject::not_found()),
    }
}

#[tokio::main]
async fn main() {
    let args = command_args();
    let path = Path::new(args.value_of("database").unwrap())
        .parent()
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .to_string();
    let addr: IpAddr = args.value_of("address").unwrap().parse().unwrap();
    let port: u16 = args.value_of("port").unwrap().parse().unwrap();
    match build_renderer(args) {
        Ok(renderer) => {
            let toc_renderer = renderer.clone();
            let theorems = warp::path::param()
                .and(warp::path::param())
                .and(with_renderer(renderer))
                .and_then(get_theorem);
            let toc = warp::path::param()
                .and(warp::path("toc"))
                .and(warp::query::<HashMap<String, String>>())
                .and(with_renderer(toc_renderer))
                .and_then(get_toc);
            let res =
                warp::path("static")
                    .and(warp::fs::dir("static"))
                    .map(|res: warp::fs::File| {
                        warp::reply::with_header(res, "cache-control", "public, max-age=31536000")
                    });
            let statics = warp::fs::dir(path);
            let routes = theorems.or(toc).or(res).or(statics);
            warp::serve(routes).run((addr, port)).await;
        }
        Err(message) => {
            println!("Error: {}", message);
        }
    }
}

fn build_renderer(args: ArgMatches) -> Result<Renderer, String> {
    let db = build_db(&args)?;
    #[cfg(feature = "sts")]
    let sts = parse_sts(db.clone(), &args, "mathml")?;
    let bib_file = args.value_of("bib_file");
    Ok(Renderer::new(
        db,
        bib_file.map(str::to_string),
        #[cfg(feature = "sts")]
        sts,
    ))
}
