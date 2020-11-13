
use std::env;
use std::path::PathBuf;
use std::process::exit;
use glob;

fn main()
{
    let mut filenames = Vec::<PathBuf>::new();
    let mut valid = true;

    for (index, argument) in env::args().skip(1).enumerate()
    {
        let glob_result = glob::glob(&argument);
        let paths = match glob_result
        {
            Ok(g) => g,
            Err(_) =>
            {
                println!("Unable to interpret argument #{} as glob.", index);
                valid = false;
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
                    valid = false;
                    break;
                }
            };

            filenames.push(path);
        }
    }

    if valid == false
    {
        exit(1);
    }

    for p in filenames
    {
        println!("{:?}", p);
    }
}
