use aria_core::core::start as start_aria;
use aria_core::core::stop as stop_aria;
use clap::Parser;

/// CLI usage for Aria.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

/// CLI subcommands for Aria.
#[derive(Parser, Debug)]
enum Command {
    /// Start Aria.
    Start,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Start => start_aria(),
    }

    // Wait for user input to exit, due to the keylogger, only Enter, LeftControl, and C can be used.
    let mut input = String::new();

    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line.");

    stop_aria();
}
