
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
    patterns: Vec::<String>
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
    let mut files = list_files(&args);

    let buffer_filename = std::env::temp_dir().join(".brn_buffer");
    write_filenames_to_buffer(&buffer_filename, &files);
    invoke_editor(&buffer_filename);
    read_filenames_from_buffer(&buffer_filename, &mut files);

    execute_rename(&files);
    println!("{} files renamed.", files.len());
}

fn parse_arguments() -> Arguments
{
    Arguments
    {
        patterns: env::args().skip(1).collect()
    }
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
        2 => die!(
            "Unable to create glob from arguments #{} and #{}.",
            invalid_indices[0],
            invalid_indices[1]
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

fn invoke_editor(buffer_filename: &Path)
{
    let status = Command::new("vim")
        .args(buffer_filename.to_str())
        .status()
        .unwrap_or_else(
            |_| die!("Failed to spawn editor.")
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
