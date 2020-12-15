mod action_history;

use colored::*;
use confy;
use glob;
use action_history::{ActionHistory, ChangeList};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

#[derive(Serialize, Deserialize)]
struct Config {
    editor_executable: String
}
impl Default for Config {
    fn default() -> Config {
        Config {
            editor_executable: "vim".to_owned()
        }
    }
}

struct Arguments {
    patterns:              Vec<String>,
    editor_executable:     Option<String>,
    set_editor_executable: Option<String>,
    include_extensions:    bool,
    dry_run:               bool,
    usage:                 bool
}

#[derive(PartialEq)]
enum FileOutcome {
    Renamed,
    RenameWasNoop,
    Unchanged
}

struct FileToRename {
    full_path_before: PathBuf,
    full_path_after:  PathBuf,
    filename_before:  OsString,
    filename_after:   OsString,
    outcome:          FileOutcome
}

#[derive(PartialEq)]
enum ActionWhenStuck {
    Retry,
    Skip,
    Abort,
    Rollback
}

#[derive(PartialEq)]
enum ActionWhenStuckRollingBack {
    Retry,
    Skip,
    AbortRollback
}

macro_rules! die
{
    ($($arg:expr),+) => {{
        print!("{}", "ERROR. ".red());
        println!($($arg), +);
        exit(1);
    }}
}

fn main() {
    let mut config = confy::load::<Config>("brnt").unwrap_or(Config::default());
    let args = parse_arguments();
    if args.usage == true {
        print_usage();
        exit(0);
    }
    if let Some(x) = &args.set_editor_executable {
        config.editor_executable = x.to_owned();
        confy::store("brnt", &config)
            .unwrap_or_else(|_| die!("Unable to save config file."));
        println!("Editor set to '{}'.", config.editor_executable);
        exit(0);
    }

    let mut files = list_files(&args);
    handle_degenerate_cases(&args, &files);

    let buffer_filename = std::env::temp_dir().join(".brnt_buffer");
    write_filenames_to_buffer(&buffer_filename, &files);
    invoke_editor(&config, &args, &buffer_filename);
    read_filenames_from_buffer(&buffer_filename, &mut files, &args);

    execute_rename(&args, &mut files);
    print_state(&files);
}

fn print_usage() {
    let version = env!("CARGO_PKG_VERSION");
    println!("");
    println!("brnt {}", version);
    println!("Rename files in bulk using your text editor of choice.");
    println!("");
    println!("    brnt");
    println!("        [-e|--editor EDITOR-PATHNAME]");
    println!("        [-x|--include-extensions]");
    println!("        [--dry-run]");
    println!("        SEARCH-PATTERN...");
    println!("");
    println!("    brnt --set-editor EDITOR-PATHNAME");
    println!("");
    println!("brnt will collect all the files which match the search patterns provided");
    println!("into a list, then display that list in your text editor of your choosing.");
    println!("Edit the filenames at your leisure, then close the editor and brnt");
    println!("will rename the files correspondingly.");
    println!("");
}

fn parse_arguments() -> Arguments {
    let mut result = Arguments {
        patterns:              Vec::new(),
        editor_executable:     None,
        set_editor_executable: None,
        include_extensions:    false,
        dry_run:               false,
        usage:                 false
    };
    let mut force_patterns = false;
    let mut next_is_editor_executable = false;
    let mut next_is_set_editor_executable = false;

    for arg in env::args().skip(1) {
        if next_is_editor_executable == true {
            result.editor_executable = Some(arg);
            next_is_editor_executable = false;
        } else if next_is_set_editor_executable == true {
            result.set_editor_executable = Some(arg.to_owned());
            result.editor_executable = Some(arg.to_owned());
            next_is_set_editor_executable = false;
        } else if arg.starts_with("--") == true && force_patterns == false {
            match arg.as_str() {
                "--usage" => result.usage = true,
                "--help" => result.usage = true,
                "--editor" => next_is_editor_executable = true,
                "--set-editor" => next_is_set_editor_executable = true,
                "--include-extensions" => result.include_extensions = true,
                "--dry-run" => result.dry_run = true,
                "--" => force_patterns = true,
                _ => die!("Don't understand option {}.", arg)
            }
        } else if arg.starts_with("-") == true && force_patterns == false {
            match arg.as_str() {
                "-e" => next_is_editor_executable = true,
                "-x" => result.include_extensions = true,
                _ => die!("Don't understand option {}.", arg)
            }
        } else {
            result.patterns.push(arg);
        }
    }

    if next_is_editor_executable == true || next_is_set_editor_executable == true {
        result.usage = true;
    }

    if result.set_editor_executable != None {
        // require that no globs were provided
        if result.patterns.len() != 0 {
            result.usage = true;
        }
    } else {
        // require that some globs were provided
        if result.patterns.len() == 0 {
            result.usage = true;
        }
    }

    result
}

fn list_files(args: &Arguments) -> Vec<FileToRename> {
    let mut filenames = Vec::<FileToRename>::new();
    let mut invalid_indices = Vec::<usize>::new();
    let patterns = &args.patterns;

    for (index, pattern) in patterns.into_iter().enumerate() {
        let glob_result = glob::glob(&pattern);
        let paths = match glob_result {
            Ok(g) => g,
            Err(_) => {
                invalid_indices.push(index);
                continue;
            }
        };

        for path in paths {
            let path = match path {
                Ok(path) => path,
                Err(_) => {
                    invalid_indices.push(index);
                    continue;
                }
            };

            let relevant_part_of_file_name = if args.include_extensions {
                path.file_name()
            } else {
                path.file_stem()
            };

            let relevant_part_of_file_name = relevant_part_of_file_name
                .unwrap_or_else(|| die!("Unable to get file name out of path."));

            filenames.push(FileToRename {
                full_path_before: path.to_owned(),
                full_path_after:  PathBuf::new(),
                filename_before:  relevant_part_of_file_name.to_owned(),
                filename_after:   OsString::new(),
                outcome:          FileOutcome::Unchanged
            });
        }
    }

    match invalid_indices.len() {
        0 => filenames,
        1 => die!(
            "Unable to create search pattern from argument #{}.",
            invalid_indices[0]
        ),
        _ => {
            let string_indices: Vec<String> =
                invalid_indices.iter().map(|n| format!("#{}", n)).collect();
            let (last, rest) = string_indices.split_last().unwrap();
            die!(
                "Unable to create search pattern from arguments {} and {}.",
                rest.join(", "),
                last
            )
        }
    }
}

fn handle_degenerate_cases(args: &Arguments, files: &Vec<FileToRename>) {
    if files.len() == 0 {
        if args.patterns.len() == 1 {
            println!("No files matched pattern.");
        } else {
            println!("No files matched any patterns.");
        }
        exit(0);
    }
}

fn write_filenames_to_buffer(buffer_filename: &Path, files: &Vec<FileToRename>) {
    let buffer_file = match File::create(&buffer_filename) {
        Ok(file) => file,
        Err(_) => die!("Unable to open buffer file for writing.")
    };
    let mut writer = LineWriter::new(buffer_file);

    for n in 0..files.len() {
        let file = &files[n];
        let filename_before = file
            .filename_before
            .to_str()
            .unwrap_or_else(|| die!("Unable to get string for filename."));
        // let newline = if n < files.len() - 1 { "\n" } else { "" };
        let newline = "\n";
        write!(&mut writer, "{}{}", filename_before, newline)
            .unwrap_or_else(|_| die!("Unable to write filenames to buffer file."));
    }
}

fn invoke_editor(config: &Config, args: &Arguments, buffer_filename: &Path) {
    let editor: &str = match &args.editor_executable {
        Some(e) => &e,
        None => &config.editor_executable
    };

    let status = Command::new(editor)
        .args(buffer_filename.to_str())
        .status()
        .unwrap_or_else(|_| die!("Failed to start editor ({}).", editor));

    if status.success() == false {
        die!("Editor returned non-zero exit code.");
    }
}

fn read_filenames_from_buffer(
    buffer_filename: &Path,
    files: &mut Vec<FileToRename>,
    args: &Arguments
) {
    let buffer_file = File::open(buffer_filename)
        .unwrap_or_else(|_| die!("Unable to open buffer file for reading."));
    let reader = BufReader::new(buffer_file);
    let mut filenames_coming_in = Vec::<OsString>::new();

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                println!("Unable to read buffer file.");
                exit(1);
            }
        };

        let trimmed = line.trim().to_owned();
        if trimmed.len() > 0 {
            filenames_coming_in.push(OsString::from(trimmed));
        }
    }

    if filenames_coming_in.len() < files.len() {
        die!(
            "Not enough filenames in text file after edit ({} instead of {}).",
            filenames_coming_in.len(),
            files.len()
        );
    } else if filenames_coming_in.len() > files.len() {
        die!(
            "Too many filenames in text file after edit ({} instead of {}).",
            filenames_coming_in.len(),
            files.len()
        );
    }

    let new_filename_for_file = |file: &FileToRename| -> OsString {
        if args.include_extensions == true {
            file.filename_after.to_owned()
        } else {
            let extension = file.full_path_before.extension();
            let mut new_name = file.filename_after.to_owned();
            if let Some(e) = extension {
                new_name.push(".");
                new_name.push(e);
            }
            new_name
        }
    };
    let new_path_for_file = |file: &FileToRename| -> PathBuf {
        file.full_path_before
            .with_file_name(new_filename_for_file(file))
    };

    for n in 0..files.len() {
        files[n].filename_after = filenames_coming_in[n].to_owned();
        files[n].full_path_after = new_path_for_file(&files[n]);
    }
}

fn ask_what_to_do_when_stuck(stuck_at_file: &FileToRename) -> ActionWhenStuck {
    let friendly_name_before = &stuck_at_file
        .full_path_before
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let friendly_name_after = &stuck_at_file
        .full_path_after
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    println!(
        "{} Can't rename '{}' -> '{}'.",
        "HALT. ".yellow(),
        friendly_name_before.yellow(),
        friendly_name_after.yellow()
    );
    println!("       {}{}", "r".bright_cyan(), ": Retry".cyan());
    println!("       {}{}", "s".bright_cyan(), ": Skip this file".cyan());
    println!("       {}{}", "a".bright_cyan(), ": Abort here".cyan());
    println!("       {}{}", "u".bright_cyan(), ": Undo all".cyan());
    print!("{}", "       [r/s/u/a]: ".blue());
    std::io::stdout().flush().unwrap();
    let mut key: char = '_';
    let mut action = None::<ActionWhenStuck>;
    while action == None {
        let getch_result = getch::Getch::new().getch();
        if let Ok(k) = getch_result {
            key = k as char
        };
        action = match getch_result {
            Ok(b'r') | Ok(b'R') => Some(ActionWhenStuck::Retry),
            Ok(b's') | Ok(b'S') => Some(ActionWhenStuck::Skip),
            Ok(b'a') | Ok(b'A') => Some(ActionWhenStuck::Abort),
            Ok(b'u') | Ok(b'U') => Some(ActionWhenStuck::Rollback),
            _ => None
        };
    }
    println!("{}", key);
    action.unwrap()
}

fn ask_what_to_do_when_stuck_rolling_back(
    stuck_at_file: &FileToRename
) -> ActionWhenStuckRollingBack {
    let friendly_name_before = &stuck_at_file
        .full_path_before
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let friendly_name_after = &stuck_at_file
        .full_path_after
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    println!(
        "{} Can't undo rename '{}' back to '{}'.",
        "HALT. ".yellow(),
        friendly_name_after.yellow(),
        friendly_name_before.yellow()
    );
    println!("       {}{}", "r".bright_cyan(), ": Retry".cyan());
    println!("       {}{}", "s".bright_cyan(), ": Skip this file".cyan());
    println!("       {}{}", "a".bright_cyan(), ": Abort here".cyan());
    print!("{}", "       [r/s/u]: ".blue());
    std::io::stdout().flush().unwrap();
    let mut key: char = '_';
    let mut action = None::<ActionWhenStuckRollingBack>;
    while action == None {
        let getch_result = getch::Getch::new().getch();
        if let Ok(k) = getch_result {
            key = k as char
        };
        action = match getch_result {
            Ok(b'r') | Ok(b'R') => Some(ActionWhenStuckRollingBack::Retry),
            Ok(b's') | Ok(b'S') => Some(ActionWhenStuckRollingBack::Skip),
            Ok(b'a') | Ok(b'A') => Some(ActionWhenStuckRollingBack::AbortRollback),
            _ => None
        };
    }
    println!("{}", key);
    action.unwrap()
}

fn execute_rename(args: &Arguments, files: &mut Vec<FileToRename>) {
    fn rename_file_if_safe(p: &Path, q: &Path) -> Result<(), ()> {
        if q.exists() {
            return Err(());
        };
        match fs::rename(p, q) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    if args.dry_run == true {
        for file in files {
            println!(
                "{} -> {}",
                file.full_path_before.display(),
                file.full_path_after.display()
            );
        }
        exit(0);
    }

    let mut index = 0;
    let mut rollback = false;
    let mut change_list = ChangeList::new();
    while index < files.len() {
        let mut file = &mut files[index];

        if file.full_path_after == file.full_path_before {
            file.outcome = FileOutcome::RenameWasNoop;
            index += 1;
            continue;
        }

        match rename_file_if_safe(&file.full_path_before, &file.full_path_after) {
            Ok(_) => {
                file.outcome = FileOutcome::Renamed;
                index += 1;
                change_list.push(&file.full_path_before, &file.full_path_after);
            }
            Err(_) => match ask_what_to_do_when_stuck(&file) {
                ActionWhenStuck::Retry => continue,
                ActionWhenStuck::Abort => break,
                ActionWhenStuck::Skip => {
                    index += 1;
                    continue;
                }
                ActionWhenStuck::Rollback => {
                    rollback = true;
                    break;
                }
            }
        }
    }

    if rollback == true {
        println!("Undoing renames...");

        index = 0;
        while index < files.len() {
            let mut file = &mut files[index];
            if file.outcome != FileOutcome::Renamed {
                index += 1;
                continue;
            }

            match rename_file_if_safe(&file.full_path_after, &file.full_path_before) {
                Ok(_) => {
                    file.outcome = FileOutcome::Unchanged;
                    index += 1;
                    continue;
                }
                Err(_) => match ask_what_to_do_when_stuck_rolling_back(&file) {
                    ActionWhenStuckRollingBack::Retry => continue,
                    ActionWhenStuckRollingBack::AbortRollback => break,
                    ActionWhenStuckRollingBack::Skip => {
                        index += 1;
                        continue;
                    }
                }
            }
        }
    } else {
        ActionHistory::write(change_list, ".history.json");
    }
}

fn print_state(files: &Vec<FileToRename>) {
    let mut renamed = 0;
    let mut noop = 0;
    let mut unchanged = 0;

    for f in files {
        match f.outcome {
            FileOutcome::Renamed => renamed += 1,
            FileOutcome::RenameWasNoop => noop += 1,
            FileOutcome::Unchanged => unchanged += 1
        }
    }

    if unchanged == 0 {
        println!("{}  renamed             ... {}", "DONE.".green(), renamed);
    } else {
        println!("{}  renamed             ... {}", "DONE.".yellow(), renamed);
    }
    if noop > 0 {
        println!("       skipped (no change) ... {}", noop);
    }
    if unchanged > 0 {
        println!("       skipped (problem)   ... {}", unchanged);
    }
}
