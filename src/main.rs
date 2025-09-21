use std::env::args;
use std::process::{exit, ExitCode};

mod pack;
mod core;
mod unpack;

fn help(name: &str) {
    println!("Usage:");
    println!("  {} help", name);
    println!("  {} pack <file name> <directory>", name);
    println!("  {} unpack <file name> <directory>", name);
}

fn not_enough(name: &str) {
    println!("Not enough arguments");
    println!();
    help(name);
    exit(1);
}

fn main() -> ExitCode {
    let args: Vec<_> = args().collect();

    if args.len() < 2 {
        println!("No command provided");
        println!();
        help(args[0].as_str());
        return ExitCode::FAILURE;
    }

    match args[1].as_str() {
        "pack" => {
            if args.len() < 4 {
                not_enough(args[0].as_str());
            }

            let directory = args[3].as_str();
            let file = args[2].as_str();

            println!("Packing {} into {}...", directory, file);

            if let Err(error) = pack::pack(file, directory) {
                println!("Error occurred: {}", error);
                return ExitCode::FAILURE;
            } else {
                println!("Finished.");
            }
        },
        "unpack" => {
            if args.len() < 4 {
                not_enough(args[0].as_str());
            }

            let directory = args[3].as_str();
            let file = args[2].as_str();

            println!("Unpacking {} into {}...", file, directory);

            if let Err(error) = unpack::unpack(file, directory) {
                println!("Error occurred: {}", error);
                return ExitCode::FAILURE;
            } else {
                println!("Finished.");
            }
        },
        "help" => {
            help(args[0].as_str());
        },
        invalid_command => {
            println!("Unknown command: {}", invalid_command);
            println!();
            help(args[0].as_str());
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
