use std::env;
use std::fs;

pub mod ast;
pub mod dmmf;
use ast::parser;
pub mod dml;
use dml::validator::{BaseValidator, EmptyAttachmentValidator, Validator};

mod postgres;

// Pest grammar generation on compile time.
extern crate pest;
#[macro_use]
extern crate pest_derive;

extern crate clap;
use clap::{App, Arg, SubCommand};

fn main() {
    let formats = ["sorenbs", "matthewmueller"];

    let matches = App::new("Prisma Datamodel Playgroung")
        .version("0.1")
        .author("Emanuel Jöbstl <emanuel.joebstl@gmail.com>")
        .about("Alpha implementation of different datamodel definition grammars.")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input datamodel file to use")
                .required(true)
                .index(1),
        )
        .get_matches();

    let file_name = matches.value_of("INPUT").unwrap();
    let file = fs::read_to_string(&file_name).expect(&format!("Unable to open file {}", file_name));

    let ast = parser::parse(&file);

    // Builtin Tooling
    // let validator = BaseValidator::<dml::BuiltinTypePack, EmptyAttachmentValidator>::new();

    // Postgres-Specific Tooling
    let validator = BaseValidator::<postgres::PostgresTypePack, postgres::PostgresAttachmentValidator>::new();

    let dml = validator.validate(&ast);

    let json = dmmf::render_to_dmmf(&dml);

    println!("{}", json);
}
