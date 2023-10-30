use crate::sts::{StsDefinition, StsScheme};
use clap::ArgMatches;
use metamath_knife::formula::TypeCode;
use metamath_knife::{grammar::FormulaToken, statement::as_str};
use metamath_knife::{Database, Span};
use nom::{
    branch::alt, bytes::complete::tag, bytes::complete::take_until, bytes::complete::take_while1,
    character::complete::multispace1, combinator::map, combinator::value, multi::many0,
    multi::separated_list1, sequence::delimited, sequence::terminated, sequence::tuple, IResult,
};
use regex::Regex;
use std::fs::read_to_string;

impl StsScheme {
    fn parse(
        database: Database,
        is_identifier: bool,
        math: Vec<&str>,
        subst: &str,
    ) -> Result<Self, String> {
        let grammar = database.grammar_result().clone();
        let nset = database.name_result();
        let mut symbols = vec![];
        for t in math {
            symbols.push(
                nset.lookup_symbol(t.as_bytes())
                    .ok_or(format!("Unknown symbol {}", t))?
                    .atom,
            );
        }
        let typecode = symbols[0];
        let all_typecodes = grammar.typecodes();
        let this_typecode = &[typecode];
        let typecodes: &[TypeCode] = if all_typecodes.contains(&typecode) {
            this_typecode
        } else {
            &all_typecodes
        };
        let formula = grammar
            .parse_formula(
                &mut symbols.into_iter().skip(1).map(|t| {
                    Ok(FormulaToken {
                        symbol: t,
                        span: Span::NULL,
                    })
                }),
                typecodes,
                true,
                nset,
            )
            .map_err(|diag| {
                format!(
                    "Could not parse formula: {:?} ({}) {}",
                    diag,
                    subst,
                    as_str(nset.atom_name(typecode))
                )
            })?;
        Ok(Self::new(is_identifier, typecode, formula, subst))
    }
}

#[derive(Clone, Debug)]
enum Directive<'a> {
    Comment,
    Scheme((bool, Vec<&'a str>, &'a str)),
    Command(&'a str),
    Display(&'a str),
    Inline(&'a str),
    Header(&'a str),
}

fn is_mm_token(chr: char) -> bool {
    ('\x21'..='\x7E').contains(&chr) && chr != '$'
}
fn comment(input: &str) -> IResult<&str, Directive> {
    value(
        Directive::Comment,
        alt((
            delimited(tag("$("), take_until("$)"), tag("$)")),
            // Also considering whitespace between other directives as comment
            multispace1,
        )),
    )(input)
}
fn mathstring(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list1(multispace1, take_while1(is_mm_token))(input)
}
fn scheme(input: &str) -> IResult<&str, Directive> {
    map(
        tuple((
            map(alt((tag("$s"), tag("$i"))), |tag| tag == "$i"),
            terminated(delimited(multispace1, mathstring, multispace1), tag("$:")),
            terminated(take_until("$."), tag("$.")),
        )),
        |(is_identifier, math, subst): (bool, Vec<&str>, &str)| {
            Directive::Scheme((is_identifier, math, subst))
        },
    )(input)
}
fn typecodes(input: &str) -> IResult<&str, Directive> {
    map(
        delimited(tag("$u"), take_until("$."), tag("$.")),
        |_: &str| Directive::Comment,
    )(input)
}
fn command(input: &str) -> IResult<&str, Directive> {
    map(
        delimited(tag("$c"), take_until("$."), tag("$.")),
        |command: &str| Directive::Command(command),
    )(input)
}
fn display(input: &str) -> IResult<&str, Directive> {
    map(
        delimited(tag("$d"), take_until("$."), tag("$.")),
        |display: &str| Directive::Display(display),
    )(input)
}
fn inline(input: &str) -> IResult<&str, Directive> {
    map(
        delimited(tag("$t"), take_until("$."), tag("$.")),
        |inline: &str| Directive::Inline(inline),
    )(input)
}
fn header(input: &str) -> IResult<&str, Directive> {
    map(
        delimited(tag("$h"), take_until("$."), tag("$.")),
        |header: &str| Directive::Header(header),
    )(input)
}
fn file(input: &str) -> IResult<&str, Vec<Directive>> {
    many0(alt((
        comment, scheme, typecodes, command, display, inline, header,
    )))(input)
}

impl StsDefinition {
    fn parse(db: Database, input: String) -> Result<Self, String> {
        let mut schemes = vec![];
        let mut header = "".to_string();
        let mut display = "".to_string();
        let mut inline = "".to_string();
        let mut command = "".to_string();
        let (remaining, directives) = file(&input).map_err(|e| format!("Parse Error: {}", e))?;
        remaining
            .is_empty()
            .then_some(())
            .ok_or("File could not be parsed completely!")?;
        for directive in directives {
            match directive {
                Directive::Comment => {}
                Directive::Scheme((i, m, s)) => match StsScheme::parse(db.clone(), i, m, s) {
                    Ok(scheme) => schemes.push(scheme),
                    Err(error) => eprintln!("{}", error),
                },
                Directive::Command(c) => {
                    command = c.to_string();
                }
                Directive::Display(d) => {
                    display = d.to_string();
                }
                Directive::Inline(i) => {
                    inline = i.to_string();
                }
                Directive::Header(h) => {
                    header = h.to_string();
                }
            }
        }
        StsDefinition::new(db, schemes, header, display, inline, command)
    }
}

pub fn parse_sts(db: Database, args: &ArgMatches, format: &str) -> Result<StsDefinition, String> {
    let dbpath = args.value_of("database").unwrap();
    // Match an optional path ending in /, the database name, and the .mm extention
    let dbname_matches = Regex::new(r"^(.+/)?([^/]+)\.mm$")
        .unwrap()
        .captures(dbpath)
        .ok_or("Could not parse database file name")?;
    let path = dbname_matches.get(1).map_or("", |m| m.as_str());
    let name = dbname_matches.get(2).unwrap().as_str();
    let filename = format!("{}{}-{}.mmts", path, name, format);
    let contents =
        read_to_string(filename).expect("Something went wrong reading the STS definition file");
    let definition = StsDefinition::parse(db, contents)?;
    if args.is_present("check_sts") {
        definition.check();
    }
    Ok(definition)
}
