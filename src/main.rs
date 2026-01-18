use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::Result;
use clap::Subcommand;
use std::path::Path;
use std::io::IsTerminal;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[arg(hide = true, env = "PWD", default_value = ".")]
    cwd: PathBuf,
    #[arg(hide = true, long)]
    complete: Option<String>,
    /// Favourite or Path
    #[arg(value_name = "TARGET")]
    target: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List Favourites
    List,
    /// Edit file
    Edit {
        #[arg(value_name = "PATH")]
        path: String,
    },
    /// Add path Favourite
    Add {
        #[arg(value_name = "PATH")]
        path: String,
    },
    /// Remove Favourite
    Remove {
        #[arg(value_name = "FAVOURITE")]
        favourite: String,
    },
    /// Display a directory as a tree
    #[command(short_flag = 't')]
    Tree {
        /// Directory Path
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Tree Depth
        #[arg(long, default_value_t = 3)]
        depth: usize,
        /// Max entries to show visual only (not in pipe)
        #[arg(short = 'e', long, default_value_t = 10)]
        entries: usize,
        /// Show hidden Entries
        #[arg(short = 'a')]
        hidden: bool,
    },
    /// Jump back to previous Dir
    #[command(short_flag = 'b')]
    Back {
        ///Jump n times back
        #[arg(short = 'n', default_value_t = 1)]
        number: usize,
        ///List last 10 states
        #[arg(short = 'l', conflicts_with = "number")]
        list: bool,
    },
}

fn main() -> Result<()> {

    let cli = Cli::parse();

    if let Some(partial) = cli.complete {
        handle_completion(partial);
        return Ok(());
    }

    match cli.command {
        Some(Commands::List) => {
            if stdout_is_tty() {
                println!("Listing favourites...");
                for line in get_fav_list().clone().into_iter().filter(|l| !l.to_string_lossy().trim().is_empty()) {
                    println!("  {}", line.display());
                }
            } else {
                for line in get_fav_list().clone().into_iter().filter(|l| !l.to_string_lossy().trim().is_empty()) {
                    println!("{}", line.display());
                }
            }
        }
        Some(Commands::Edit { path }) => {
            let target = Path::new(&path);
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
            if target.exists() && target.is_file() {
                Command::new(editor)
                    .arg(target)
                    .status()?; // waits for editor to finish
                return Ok(());
            };
            if let Some(fav_path) = check_fav(path.clone()) {
                if fav_path.exists() && fav_path.is_file() {
                    Command::new(editor)
                        .arg(fav_path)
                        .status()?; // waits for editor to finish
                    return Ok(());
                }
            };
            return Ok(());

        }
        Some(Commands::Add { path }) => {
            let path_conv = PathBuf::from(path);
            if !path_conv.exists() || get_fav_list().contains(&path_conv){
                return Ok(());
            }
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(get_conf_dir())?;
            let _ = writeln!(file, "{}", path_conv.to_string_lossy());
            //get_fav_list().push(path_conv);
        }
        Some(Commands::Remove { favourite }) => {
            let f_upper = favourite.to_ascii_uppercase();
            if let Some(matched_path) = get_fav_list().iter().find(|p| {
                p.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.to_ascii_uppercase() == f_upper)
                    .unwrap_or(false)})
            {
                let contents = fs::read_to_string(get_conf_dir().clone())?;
                let filtered: String = contents
                    .lines()
                    .filter(|line| line != &matched_path.display().to_string())
                    .map(|line| format!("{line}\n"))
                    .collect();

                let mut file = fs::File::create(get_conf_dir())?;
                file.write_all(filtered.as_bytes())?;
            }
        }
        Some(Commands::Tree { path, depth, entries, hidden }) => {
            let root = path.unwrap_or(get_current_dir());

            if stdout_is_tty() {
                create_tree(&root, String::new(), true, depth, entries, hidden);
            } else {
                let mut paths = Vec::new();
                collect_tree_paths(&root, depth, &mut paths, hidden);

                for p in paths {
                    println!("{}", p.display());
                }
            }
        }
        Some(Commands::Back { number, list }) => {
            if list {
                let states = get_states();
                if stdout_is_tty() {
                    let start = states.len().saturating_sub(10);
                    let last_ten = &states[start..];
                    for (i, line) in last_ten.iter().rev().enumerate() {
                        println!("{:>2}  {}", i + 1, line.display());
                    }
                } else {
                    for line in states.clone().into_iter() {
                        println!("{}", line.to_string_lossy());
                    }
                }
                return Ok(());
            }
            if let Some(prev) = pop_stack(number) {
                println!("{}", prev.to_string_lossy());
            } else {
                eprintln!("no previous directory");
            }
        }
        None => {
            //Default for CD
            if let Some(target) = cli.target {
                if let Some(fav_path) = check_fav(target.clone()) {
                    if fav_path.exists() && fav_path != cli.cwd {
                        write_state(&cli.cwd);
                        println!("{}", fav_path.display());
                        return Ok(());
                    }
                }

                let path = Path::new(&target);
                if path.exists() && path != cli.cwd {
                    write_state(&cli.cwd);
                    println!("{}", path.display());
                    return Ok(());
                }
                eprintln!("Wrong path/favourite")
            } else {
                eprintln!("No target specified")
            }
        }
    }
    Ok(())
}

fn stdout_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

fn get_home_dir() -> PathBuf {
    match dirs_next::home_dir() {
        Some(dir) => dir,
        None => PathBuf::from(format!("/home/{}", whoami::username()))
    }
}

fn get_conf_dir() -> PathBuf {
    get_home_dir().join(".config/crabwalker/fav.txt")
}

fn get_state_dir() -> PathBuf {
    get_home_dir().join(".config/crabwalker/state.txt")
}

fn get_current_dir() -> PathBuf {
    //Get current Dir
    match env::current_dir() {
        Ok(path) => path,
        Err(_) => get_home_dir()
    }
}

fn get_fav_list() -> Vec<PathBuf> {
    //Create favourites list from config file
    match fs::read_to_string(get_conf_dir()) {
        Ok(text) => {
            let paths: Vec<PathBuf> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            paths
        },
        Err(_err) => {Vec::new()}
    }
}

fn get_states() -> Vec<PathBuf> {
    match fs::read_to_string(get_state_dir()) {
        Ok(text) => {
            let paths: Vec<PathBuf> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(PathBuf::from)
                .collect();
            paths
        },
        Err(_err) => {Vec::new()}
    }
}

fn get_fav_names() -> Vec<String> {
    let fav_list = get_fav_list();
    let fav_names: Vec<String> = fav_list
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|name| name.to_str())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    fav_names
}

fn check_fav (target: String) -> Option<PathBuf> {
    let fav_upper = target.to_ascii_uppercase();
    let fav_list = get_fav_list();
    let path = fav_list
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_uppercase() == fav_upper)
                .unwrap_or(false)
        })
        .cloned();
    path
}

fn collect_tree_paths(path: &Path, max_depth: usize, out: &mut Vec<PathBuf>, hidden: bool) {
    if max_depth == 0 {
        return;
    }

    out.push(path.to_path_buf());

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if !hidden {
                    if let Some(name) = entry_path.file_name() {
                        if name.to_string_lossy().starts_with('.') {
                            continue;
                        }
                    }
                }
                collect_tree_paths(&entry.path(), max_depth - 1, out, hidden);
            }
        }
    }
}

fn create_tree(path: &Path, prefix: String, last: bool, max_depth: usize, max_entries: usize, hidden: bool) {
    if max_depth == 0 {return;}

    //let max_entries = 10;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let icon = if path.is_dir() { "📁" } else { "📄" };
    let connector = if last { "└─ " } else { "├─ " };
    println!("{}{}{} {}", prefix, connector, icon, name);

    if path.is_dir() {
        let all_entries = entries_in_dir(path, hidden);
        let entries = &all_entries[..std::cmp::min(max_entries, all_entries.len())];
        let new_prefix = if last { format!("{}   ", prefix) } else { format!("{}│  ", prefix) };
        for (i, entry) in entries.iter().enumerate() {
            let is_last = i == entries.len() - 1 && all_entries.len() <= max_entries;
            create_tree(&entry.path(), new_prefix.clone(), is_last, max_depth - 1, max_entries, hidden);
        }
        if all_entries.len() > entries.len() && max_depth > 1 {
            let dots_prefix = new_prefix.clone();
            let connector = "└─ ";
            println!("{}{}…", dots_prefix, connector);
        }
    }
}

fn entries_in_dir(path: &std::path::Path, hidden: bool) -> Vec<std::fs::DirEntry> {
    let entries = fs::read_dir(path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            if hidden {
                true
            } else {
                e.file_name()
                    .to_str()
                    .map(|s| !s.starts_with('.'))
                    .unwrap_or(true)
            }
        })
        .collect::<Vec<_>>();

    entries
}

fn write_state(path: &Path) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(get_state_dir()).unwrap();
    let _ = writeln!(file, "{}", path.display());

}

fn pop_stack(mut number: usize) -> Option<PathBuf> {
    let path = get_state_dir();
    let content = std::fs::read_to_string(&path).ok()?;
    let mut lines: Vec<_> = content.lines().map(|s| s.to_string()).collect();

    if lines.is_empty() {
        return None;
    }
    if number > lines.len() {
        number = lines.len();
    }

    let last_entry = lines[lines.len() - number].clone();
    let new_len = lines.len()- number;
    lines.truncate(new_len);

    if let Err(e) = std::fs::write(&path, lines.join("\n")) {
        eprintln!("Failed to update state file: {}", e);
    }
    Some(PathBuf::from(last_entry))
}

fn handle_completion(partial: String) {
    let partial_upper = partial.to_uppercase();
    for fav in get_fav_names() {
        if fav.to_uppercase().starts_with(&partial_upper) {
            println!("{fav}");
        }
    }
}
