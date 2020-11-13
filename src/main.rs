
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Write, LineWriter};
use std::path::PathBuf;
use std::process::{Command, exit};
use glob;

struct Arguments
{
    patterns: Vec::<String>
}

#[derive(Debug)]
struct FailError
{
    details: String
}

fn main()
{
    let args = parse_arguments();
    let filenames = match collect_filenames(&args)
    {
        Ok(f) => f,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

    let buffer_filename = std::env::temp_dir().join(".brn_buffer");
    {
        let buffer_file = File::create(&buffer_filename).expect(
            "Unable to open buffer file for writing."
        );
        let mut writer = LineWriter::new(buffer_file);

        for filename in filenames
        {
            write!(&mut writer, "{}\n", filename.display()).unwrap();
        }
    }

    let status = Command::new("C:\\Program Files\\Sublime Text 3\\sublime_text.exe")
        .args(buffer_filename.to_str())
        .status()
        .expect("Failed to spawn editor.");
    if status.success() == false
    {
        println!("Editor returned non-zero exit code.");
        exit(1);
    }

    {

    }
}

fn parse_arguments() -> Arguments
{
    Arguments
    {
        patterns: env::args().skip(1).collect()
    }
}

fn collect_filenames(args: &Arguments) -> Result<Vec<PathBuf>, FailError>
{
    let mut filenames = Vec::<PathBuf>::new();
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
                println!("Unable to interpret argument #{} as glob.", index);
                invalid_indices.push(index);
                break;
            }
        };
        
        for path in paths
        {
            let path = match path
            {
                Ok(path) => path,
                Err(_) =>
                {
                    println!("Unable to interpret argument #{} as glob.", index);
                    invalid_indices.push(index);
                    break;
                }
            };

            filenames.push(path);
        }
    }

    match invalid_indices.len()
    {
        0 => Ok(filenames),
        1 => Err(FailError { details: format!(
            "Unable to interpret argument #{} as glob.", invalid_indices[0]
        )}),
        2 => Err(FailError { details: format!(
            "Unable to interpret arguments #{} and #{} as glob.", invalid_indices[0], invalid_indices[1]
        )}),
        _ => Err(FailError { details:
            {
                let string_indices: Vec<String> =
                    invalid_indices.iter().map(|n| format!("#{}", n)).collect();
                let (last, rest) = string_indices.split_last().unwrap();
                format!(
                    "Unable to interpret arguments {} and {} as glob.",
                    rest.join(", "),
                    last
                )
            }
        })
    }
}



impl std::fmt::Display for FailError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for FailError {
    fn description(&self) -> &str {
        &self.details
    }
}
