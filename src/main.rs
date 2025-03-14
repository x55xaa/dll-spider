//! Main module.

#![warn(missing_docs)]


use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use log::debug;
use tabled::builder::Builder;
use tabled::settings::{Alignment, Modify, Style, object::Segment};


mod logging;
mod winapi;


#[derive(Debug)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Set the verbosity level. option is additive, and can be used up to 5 times (trace/debug/info/warn/error). (default: info)
    #[arg(short, long, action = clap::ArgAction::Count, value_parser = clap::value_parser!(u8).range(0..=5))]
    #[clap(global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug)]
#[derive(Args)]
#[group(required = true, multiple = false)]
struct Process {
    /// Name of the target process.
    #[arg(short, long = "by-name")]
    name: Option<String>,

    /// PID of the target process.
    #[arg(short, long = "by-pid")]
    pid: Option<u32>,
}

#[derive(Debug)]
#[derive(Subcommand)]
enum Commands {
    /// Load a DLL inside a target process.
    Load {
        #[command(flatten)]
        process: Process,

        /// Path to the DLL to load.
        #[arg(value_parser = |path: &str| dunce::canonicalize(path))]
        module: PathBuf,
    },

    /// Enumearate target processes.
    Enum {},
}


/// Main function.
fn main() {
    let args: Cli = Cli::parse();
    
    logging::init(
        if args.verbose != 0 { Some(logging::u8_to_level(args.verbose - 1)) } else { None }
    );

    match &args.command {
        Commands::Load { process , module} => {
            debug!("{}", format!("action=load, process={:#?}, module={:#?}", process, module));

            let dll_path: &str = module.to_str().unwrap();

            let pid: u32 = if let Some(process_name) = &process.name {
                winapi::find_process_by_name(&process_name, Some(true)).unwrap()
            } else {
                process.pid.expect("must provide either the PID or the name of the target process")
            };

            let _ = winapi::load_dll(pid, dll_path);
        },
        Commands::Enum {  } => {
            debug!("action=enum");

            let mut builder = Builder::default();

            for (key, value) in &winapi::get_process_name_pid_mapping().unwrap() {
                builder.push_record([&value.to_string(), key]);
            }

            let mut table = builder.build();
            table
                .with(
                    Modify::new(Segment::all())
                        .with(Alignment::left())
                        .with(Alignment::top()))
                .with(Style::blank());
            
            println!("{}", table);
        }
    }
}
