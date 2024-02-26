use std::{
    fs,
    io::Read,
    os::unix,
    path::{Path, PathBuf},
};

use clap::Parser;

/// Simple program to convert text file into symlink from its content.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to a file or dir
    path: PathBuf,

    /// Walk through content of the directory recursively
    #[arg(short, long)]
    recursive: bool,

    /// Do not print conversions
    #[arg(short, long)]
    silent: bool,

    /// Prompt before each conversion
    #[arg(short, long)]
    interactive: bool,

    /// Maximum file length to be considered as possible link
    #[arg(short, long, default_value = "512")]
    len: u64,

    /// Explain what is being done
    #[arg(short, long, conflicts_with = "silent")]
    verbose: bool,
}

fn print_error(path: &Path, reason: &str) {
    println!("Cannot convert '{}': {}", path.to_string_lossy(), reason)
}

fn link_target_exists(path: Option<&Path>, link: &str) -> bool {
    match path {
        Some(path) => path.join(link).exists(),
        None => Path::new(link).to_owned().exists(),
    }
}

fn ask_for_confirmation(file: &Path, link: &str) -> bool {
    println!(
        "Convert '{}' file into symlink '{}'?",
        file.to_string_lossy(),
        link
    );
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => println!("y/n only please."),
        }
    }
}

fn convert_file(file_path: &Path, args: &Args) {
    if let Ok(metadata) = fs::metadata(file_path) {
        if metadata.len() > args.len {
            if args.verbose {
                println!(
                    "File {} is too big to be considered as symlink({} > {})",
                    file_path.to_string_lossy(),
                    fs::metadata(file_path).unwrap().len(),
                    args.len
                )
            }
        }
        return;
    }

    if let Ok(link_val) = fs::read_to_string(file_path) {
        if !link_target_exists(file_path.parent(), &link_val) {
            if args.verbose {
                println!(
                    "Symlink target {} -> {} does not exists",
                    file_path.to_string_lossy(),
                    link_val
                )
            }
            return;
        }

        if !args.interactive || ask_for_confirmation(file_path, &link_val) {
            let _ = fs::remove_file(file_path);

            if let Err(error) = unix::fs::symlink(&link_val, file_path) {
                print_error(file_path, &error.to_string());
                return;
            }

            if !args.silent {
                println!(
                    "Converted to symlink: {} -> {}",
                    file_path.to_string_lossy(),
                    link_val
                )
            }
        }
    }
}

fn convert_dir(dir_path: &Path, args: &Args) {
    match fs::read_dir(dir_path) {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(entry) => {
                        let metadata = entry.metadata().unwrap();
                        if metadata.is_dir() {
                            convert_dir(&entry.path(), args)
                        } else if metadata.is_file() {
                            convert_file(&entry.path(), args)
                        } else if metadata.is_symlink() {
                            if args.verbose {
                                let path = entry.path();
                                println!(
                                    "Skipped symlink {} -> {}",
                                    path.to_string_lossy(),
                                    fs::read_link(&path).unwrap_or_default().to_string_lossy()
                                );
                            }
                        } else {
                            print_error(&entry.path(), "Not a directory or a file or a symlink")
                        }
                    }
                    Err(error) => print_error(dir_path, &error.to_string()),
                }
            }
        }
        Err(error) => print_error(dir_path, &error.to_string()),
    }
}

fn main() {
    let args = Args::parse();

    match fs::metadata(&args.path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                if args.recursive {
                    convert_dir(&args.path, &args)
                } else {
                    print_error(
                        &args.path,
                        "Is a directory. Please specify 'recursive' flag",
                    )
                }
            } else if metadata.is_file() {
                convert_file(&args.path, &args)
            } else {
                print_error(&args.path, "Not a directory or file")
            }
        }
        Err(error) => print_error(&args.path, &error.to_string()),
    }
}
