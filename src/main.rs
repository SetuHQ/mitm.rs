extern crate clap;

// Compiler config
#[allow(unused_variables)]
#[allow(dead_code)]
mod util;

use clap::{load_yaml, App};

use crate::util::args::parse_args;

fn main() {
    let yaml = load_yaml!("../config/cmd_args.yml");
    let app: App = App::from(yaml);

    parse_args(app);
}
