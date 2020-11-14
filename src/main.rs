
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Write, LineWriter, BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use die::die;
use glob;

struct Arguments
{
    patterns: Vec::<String>,
    editor_executable: Option<String>,
    usage: bool
}

struct FileToRename
{
    full_path: PathBuf,
    filename_before: OsString,
    filename_after: OsString
}

fn main()
{
    let args = parse_arguments();
    if args.usage == true
    {
        print_usage();
        exit(0);
    }
    let mut files = list_files(&args);
    handle_degenerate_cases(&args, &files);

    let buffer_filename = std::env::temp_dir().join(".brn_buffer");
    write_filenames_to_buffer(&buffer_filename, &files);
    invoke_editor(&args, &buffer_filename);
    read_filenames_from_buffer(&buffer_filename, &mut files);

    execute_rename(&files);
    println!("{} files renamed.", files.len());
}

fn print_usage()
{
    println!("brn");
    println!("");
    println!("    brn [globs]");
    println!("    brn -- [globs]");
    println!("");
}

fn parse_arguments() -> Arguments
{
    let mut result = Arguments
    {
        patterns: Vec::new(),
        editor_executable: None,
        usage: false
    };
    let mut force_patterns = false;
    let mut next_is_editor_executable = false;

    for arg in env::args().skip(1)
    {
        if next_is_editor_executable == true
        {
            result.editor_executable = Some(arg);
            next_is_editor_executable = false;
        }
        else if arg.starts_with("--") == true && force_patterns == false
        {
            match arg.as_str()
            {
                "--usage" => result.usage = true,
                "--help" => result.usage = true,
                "--editor" => next_is_editor_executable = true,
                "--" => force_patterns = true,
                _ => die!("Don't understand argument {}.", arg)
            }
        }
        else
        {
            result.patterns.push(arg);
        }
    }

    if next_is_editor_executable == true
    {
        result.usage = true;
    }

    if result.patterns.len() == 0
    {
        result.usage = true;
    }

    result
}

fn list_files(args: &Arguments) -> Vec<FileToRename>
{
    let mut filenames = Vec::<FileToRename>::new();
    let mut invalid_indices = Vec::<usize>::new();
    let patterns = &args.patterns;

    for (index, pattern) in patterns.into_iter().enumerate()
    {
        let glob_result = glob::glob(&pattern);
        let paths = match glob_result
        {
            Ok(g) => g,
            Err(_) =>
            {
                invalid_indices.push(index);
                continue;
            }
        };
        
        for path in paths
        {
            let path = match path
            {
                Ok(path) => path,
                Err(_) =>
                {
                    invalid_indices.push(index);
                    continue;
                }
            };

            filenames.push(FileToRename
            {
                full_path: path.to_owned(),
                filename_before: path.file_name().unwrap_or_else(
                    || die!("Unable to get file name out of path.")
                ).to_owned(),
                filename_after: OsString::new()
            });
        }
    }

    match invalid_indices.len()
    {
        0 => filenames,
        1 => die!(
            "Unable to create glob from argument #{}.", invalid_indices[0]
        ),
        _ => {
            let string_indices: Vec<String> =
                invalid_indices.iter().map(|n| format!("#{}", n)).collect();
            let (last, rest) = string_indices.split_last().unwrap();
            die!(
                "Unable to create glob from arguments {} and {}.",
                rest.join(", "),
                last
            )
        }
    }
}

fn handle_degenerate_cases(args: &Arguments, files: &Vec<FileToRename>)
{
    if files.len() == 0
    {
        if args.patterns.len() == 1
        {
            println!("No files matched glob.");
        }
        else
        {
            println!("No files matched any of those globs.");
        }
        exit(0);
    }
}

fn write_filenames_to_buffer(buffer_filename: &Path, files: &Vec<FileToRename>)
{
    let buffer_file = match File::create(&buffer_filename)
    {
        Ok(file) => file,
        Err(_) => die!("Unable to open buffer file for writing.")
    };
    let mut writer = LineWriter::new(buffer_file);

    for file in files
    {
        write!(&mut writer, "{}\n", file.filename_before.to_str().unwrap()).unwrap();
    }
}

fn invoke_editor(args: &Arguments, buffer_filename: &Path)
{
    let editor = match &args.editor_executable
    {
        Some(e) => e,
        None => "vim"
    };

    let status = Command::new(editor)
        .args(buffer_filename.to_str())
        .status()
        .unwrap_or_else(
            |_| die!("Failed to start editor ({}).", editor)
        );

    if status.success() == false
    {
        die!("Editor returned non-zero exit code.");
    }
}

fn read_filenames_from_buffer(buffer_filename: &Path, files: &mut Vec<FileToRename>)
{
    let buffer_file = File::open(buffer_filename).unwrap_or_else(
        |_| die!("Unable to open buffer file for reading.")
    );
    let reader = BufReader::new(buffer_file);
    let mut filenames_coming_in = Vec::<OsString>::new();

    for line in reader.lines()
    {
        let line = match line
        {
            Ok(line) => line,
            Err(_) => {
                println!("Unable to read buffer file.");
                exit(1);
            }
        };
        
        let trimmed = line.trim().to_owned();
        if trimmed.len() > 0
        {
            filenames_coming_in.push(OsString::from(trimmed));
        }
    }

    if filenames_coming_in.len() < files.len()
    {
        die!(
            "Not enough filenames in text file after edit ({} instead of {}).",
            filenames_coming_in.len(),
            files.len()
        );
    }
    else if filenames_coming_in.len() > files.len()
    {
        die!(
            "Too many filenames in text file after edit ({} instead of {}).",
            filenames_coming_in.len(),
            files.len()
        );
    }

    for n in 0..files.len()
    {
        files[n].filename_after = filenames_coming_in[n].to_owned();
    }
}

fn execute_rename(files: &Vec<FileToRename>)
{
    for file in files
    {
        let path_afterwards = file.full_path.parent().unwrap().join(
            &file.filename_after
        );
        fs::rename(&file.full_path, path_afterwards).unwrap();
    }
}
