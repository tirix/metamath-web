mod statement;
#[cfg(feature = "sts")]
mod sts;
#[cfg(feature = "sts")]
mod sts_parser;

use crate::statement::Renderer;
use clap::crate_version;
use clap::App as ClapApp;
use clap::Arg;
use clap::ArgMatches;
use metamath_knife::database::DbOptions;
use metamath_knife::Database;
use metamath_knife::diag::DiagnosticClass;
use std::convert::Infallible;
use std::path::Path;
use std::str::FromStr;
use warp::reject::Rejection;
use warp::Filter;

#[cfg(feature = "sts")]
use sts_parser::parse_sts;

fn positive_integer(val: &str) -> Result<(), String> {
    u32::from_str(val)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

fn command_args<'a>() -> ArgMatches {
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
            Arg::new("host")
                .help("Hostname to serve")
                .long("host")
                .short('h'),
        )
        .arg(
            Arg::new("port")
                .help("Port to listen to")
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
    let diag = db.diag_notations(&[DiagnosticClass::Parse], |diag| { format!("{:?}", diag) });
    if !diag.is_empty() { return Err(format!("{:?}", diag)); }
    #[cfg(feature = "sts")]
    db.grammar_pass();
    println!("Ready.");
    Ok(db)
}

fn with_renderer(
    renderer: Renderer,
) -> impl Filter<Extract = (Renderer,), Error = Infallible> + Clone {
    warp::any().map(move || renderer.clone())
}

pub async fn get_theorem(explorer: String, label: String, renderer: Renderer) -> Result<impl warp::Reply, Rejection> {
    let label = label.replace(".html", "");
    match renderer.render_statement(explorer, label) {
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
    match build_renderer(args) {
        Ok(renderer) => {
            let theorems = warp::path::param()
                .and(warp::path::param())
                .and(with_renderer(renderer))
                .and_then(get_theorem)
                .or(warp::fs::dir(path));
            warp::serve(theorems).run(([127, 0, 0, 1], 3030)).await;
        },
        Err(message) => {
            println!("Error: {}", message);
        },
    }
}

fn build_renderer(args: ArgMatches) -> Result<Renderer, String> {
    let db = build_db(&args)?;
    #[cfg(feature = "sts")]
    let sts = parse_sts(db.clone(), &args, "mathml")?;
    let bib_file = args.value_of("bib_file");
    Ok(Renderer::new(db, bib_file.map(str::to_string),
        #[cfg(feature = "sts")]
        sts,
    ))
}
