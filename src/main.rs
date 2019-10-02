#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate docopt;
extern crate regex;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod conversation;
mod emoji;
mod message;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::process;

use docopt::Docopt;

use conversation::{Conversation, Frequency, Timeline, TimelineType};
use emoji::Emojis;
use message::Message;

#[derive(Debug)]
enum AppErrorKind {
    FileNotFound,
    InvalidFile,
    InvalidHistory,
}

#[derive(Debug)]
struct AppError(AppErrorKind);

// TODO: use `error::Error`
impl Error for AppError {
    fn description(&self) -> &str {
        match self.0 {
            AppErrorKind::FileNotFound => "file not found",
            AppErrorKind::InvalidFile => "invalid file contents",
            AppErrorKind::InvalidHistory => "invalid chat history",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.description().fmt(f)
    }
}

#[derive(Debug, Serialize)]
struct Insights<'is> {
    first: Option<&'is Message>,
    last: Option<&'is Message>,
    duration: String,
    frequency: Frequency,
    frequency_per_participant: HashMap<String, Frequency>,
    total_messages: usize,
    total_words: usize,
    total_letters: usize,
    avg_words_per_message: f32,
    avg_letters_per_message: f32,
    participants: &'is Vec<String>,
    timeline: Timeline,
    emojis: Emojis,
}

impl Insights<'_> {
    fn new<'is>(cnv: &'is Conversation, tl_type: TimelineType) -> Insights<'is> {
        let (avg_words_per_message, avg_letters_per_message) = cnv.average();
        let mut frequency_per_participant = HashMap::new();
        let participants = cnv.participants();

        for p in participants {
            frequency_per_participant
                .insert(p.to_string(), cnv.by_author(p.to_string()).frequency());
        }

        Insights {
            first: cnv.first(),
            last: cnv.last(),
            duration: cnv.duration().unwrap().to_string(),
            frequency: cnv.frequency(),
            total_messages: cnv.count(),
            total_words: cnv.words(),
            total_letters: cnv.letters(),
            avg_words_per_message,
            avg_letters_per_message,
            participants,
            frequency_per_participant,
            timeline: cnv.timeline(tl_type),
            emojis: cnv.emojis(),
        }
    }
}

fn execute(filename: String, timeline_type: TimelineType) -> Result<(), AppError> {
    let contents = match fs::read_to_string(filename) {
        Ok(contents) => contents,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => return Err(AppError(AppErrorKind::FileNotFound)),
            _ => return Err(AppError(AppErrorKind::InvalidFile)),
        },
    };

    let conversation = match Conversation::from_str(&contents) {
        Ok(conversation) => conversation,
        Err(_) => return Err(AppError(AppErrorKind::InvalidHistory)),
    };

    let insights = Insights::new(&conversation, timeline_type);
    println!(
        "{}",
        serde_json::to_string(&insights).expect("unable to parse json")
    );

    Ok(())
}

const USAGE: &'static str = "
Insights - A minimalistic whatsapp chat analyser.

Usage:
    insights <file> [--pretty] [--timeline=<duration>]
    insights (-h | --help)
    insights --version

Options:
    -h --help                   shows this usage
    --version                   shows the version of application
    --pretty                    prints the analysis in pretty format 
    --timeline=<duration>       sets the duration of the timeline [default: monthly]
                                options:
                                    - daily     
                                    - weekly
                                    - monthly
                                    - yearly
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_file: String,
    flag_pretty: bool,
    flag_timeline: String,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let timeline_type = match args.flag_timeline.as_str() {
        "daily" => TimelineType::Daily,
        "weekly" => TimelineType::Weekly,
        "monthly" => TimelineType::Monthly,
        "yearly" => TimelineType::Yearly,
        _ => {
            println!("Invalid Arguments");
            println!("{}", USAGE);
            process::exit(1);
        }
    };

    process::exit(match execute(args.arg_file, timeline_type) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    })
}
