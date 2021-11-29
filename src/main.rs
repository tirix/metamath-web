mod statement;

use crate::statement::Renderer;
use clap::crate_version;
use clap::App as ClapApp;
use clap::Arg;
use clap::ArgMatches;
use metamath_knife::database::DbOptions;
use metamath_knife::Database;
use std::convert::Infallible;
use std::path::Path;
use std::str::FromStr;
use warp::reject::Rejection;
use warp::Filter;

fn positive_integer(val: String) -> Result<(), String> {
    u32::from_str(&val)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

fn command_args<'a>() -> ArgMatches<'a> {
    ClapApp::new("metamath-web")
        .version(crate_version!())
        .about("A web server providing Metamath pages")
        .arg(
            Arg::with_name("database")
                .help("Database file to load")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .help("Hostname to serve")
                .long("host")
                .short("h"),
        )
        .arg(
            Arg::with_name("port")
                .help("Port to listen to")
                .long("port")
                .short("p"),
        )
        .arg(
            Arg::with_name("jobs")
                .help("Number of threads to use for startup parsing")
                .long("jobs")
                .short("j")
                .takes_value(true)
                .validator(positive_integer),
        )
        .arg(
            Arg::with_name("bib_file")
                .help("Index file, which includes the bibliography")
                .long("bib")
                .short("b")
                .takes_value(true),
        )
        .get_matches()
}

fn build_db(args: &ArgMatches<'_>) -> Database {
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
    println!("Ready.");
    db
}

fn with_renderer(
    renderer: Renderer,
) -> impl Filter<Extract = (Renderer,), Error = Infallible> + Clone {
    warp::any().map(move || renderer.clone())
}

pub async fn get_theorem(label: String, renderer: Renderer) -> Result<impl warp::Reply, Rejection> {
    let label = label.replace(".html", "");
    match renderer.render_statement(label) {
        Some(html) => Ok(warp::reply::html(html)),
        None => Err(warp::reject::not_found()),
    }
}

#[tokio::main]
async fn main() {
    let args = command_args();
    let db = build_db(&args);
    let path = Path::new(args.value_of("database").unwrap())
        .parent()
        .unwrap_or(Path::new("."))
        .to_string_lossy()
        .to_string();
    let bib_file = args.value_of("bib_file");
    let renderer = Renderer::new(db, bib_file.map(str::to_string));
    let theorems = warp::path::param()
        .and(with_renderer(renderer))
        .and_then(get_theorem)
        .or(warp::fs::dir(path));
    warp::serve(theorems).run(([127, 0, 0, 1], 3030)).await;
}
