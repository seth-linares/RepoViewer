use git2::Repository;
use ignore::gitignore::Gitignore;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::app::FileItem;
use crate::app_error::AppError;

pub const MEGABYTE: usize = 1024 * 1024;

/*
    ---UI UTIL---
*/

static FILE_MAPPINGS: LazyLock<HashMap<&'static str, (&'static str, Color)>> = LazyLock::new(|| {
    let mut file_mappings = HashMap::new();
    
    macro_rules! file_types {
        // PATTERN MATCHING SECTION (what the macro expects as input)
        (
            $( // Start of repetition group - this whole thing can repeat
                $icon:expr,     // Capture an expression, call it $icon
                $color:expr     // Capture an expression, call it $color
                =>              // Literal `=>`
                [ // Literal bracket - must appear in input
                    $($ext:expr),+ // Another repetition: one or more expressions separated by commas
                ] // Literal closing bracket
            );* // End repetition group - can repeat zero or more times, separated by semicolons
            $(;)? // Optional trailing semicolon
        ) => {
            // What actually gets outputted
            $( // For each outer repetition (each icon/color group)
                $( // For each inner repetition (each extension in the brackets)
                    file_mappings.insert($ext, ($icon, $color)); // Generate this line
                )+ // End inner repetition (matches the + from input pattern)
            )* // End outer repetition (matches the * from input pattern)
        };
    }

    
    file_types! {
        // Programming languages
        "🦀", Color::Red => ["rs"];
        "🐍", Color::Green => ["py", "pyw", "pyi"];
        "📜", Color::Yellow => ["js", "mjs", "cjs"];
        "📜", Color::Yellow => ["ts", "tsx", "jsx"];
        "☕️", Color::LightRed => ["java", "class", "jar"];
        "💻", Color::Blue => ["cpp", "c++", "cxx", "cc", "hpp", "h++"];
        "💻", Color::Blue => ["c", "h"];
        "🐹", Color::Cyan => ["go", "mod"];
        "⚡️", Color::LightBlue => ["swift"];
        "💎", Color::Red => ["rb", "rbw", "gem"];
        "⚙️", Color::LightBlue => ["cs", "csx"];
        "⚙️", Color::LightBlue => ["vb", "vbs"];
        "🎯", Color::Magenta => ["kt", "kts"];
        "⚖️", Color::Red => ["scala", "sc"];
        "🎯", Color::Cyan => ["dart"];
        "📊", Color::Blue => ["r", "R"];
        "🍎", Color::LightBlue => ["m", "mm"];
        "🐪", Color::Blue => ["pl", "pm"];
        "🌙", Color::Blue => ["lua"];
        "🚀", Color::LightMagenta => ["jl"];
        "⚡️", Color::LightYellow => ["zig"];
        "👑", Color::Yellow => ["nim"];
        "💎", Color::White => ["crystal", "cr"];
        "💧", Color::Magenta => ["elixir", "ex", "exs"];
        "🧠", Color::Red => ["erl", "hrl"];
        "🟢", Color::Green => ["clj", "cljs", "cljc"];
        "🧠", Color::Magenta => ["hs", "lhs"];
        "🐫", Color::LightYellow => ["ml", "mli"];
        "⚙️", Color::LightBlue => ["fs", "fsx", "fsi"];
        "🌳", Color::Green => ["elm"];
        "🔷", Color::Blue => ["pas", "pp"];
        "💻", Color::Red => ["d"];
        "💻", Color::Blue => ["ada", "adb", "ads"];
        "🏢", Color::LightGreen => ["cob", "cbl"];
        "🔢", Color::Blue => ["f", "f90", "f95", "f03", "f08"];
        "⚙️", Color::Gray => ["asm", "s"];

        // Web technologies
        "🌐", Color::LightRed => ["html", "htm"];
        "🎨", Color::LightBlue => ["css", "scss", "sass", "less"];
        "💚", Color::Green => ["vue"];
        "🧡", Color::LightRed => ["svelte"];
        "🚀", Color::LightMagenta => ["astro"];
        "🐘", Color::Magenta => ["php"];

        // Configuration files
        "📦", Color::LightRed => ["toml"];
        "📋", Color::Yellow => ["json", "jsonc", "json5"];
        "📋", Color::Yellow => ["yaml", "yml"];
        "📋", Color::LightYellow => ["xml", "xsd", "xsl", "xslt"];
        "📝", Color::Gray => ["ini", "cfg", "conf", "config"];
        "🔧", Color::LightGreen => ["env", "environment"];
        "🐳", Color::Cyan => ["dockerfile", "Dockerfile"];
        "🐳", Color::Cyan => ["docker-compose", "compose"];
        "🔨", Color::LightGreen => ["makefile", "Makefile", "mk"];
        "🔨", Color::LightGreen => ["cmake"];
        "🐘", Color::LightBlue => ["gradle"];
        "☕️", Color::LightRed => ["maven", "pom"];
        "📦", Color::Red => ["npmrc", "yarnrc"];
        "📝", Color::Gray => ["editorconfig"];
        "🔧", Color::Magenta => ["gitignore", "gitattributes", "gitmodules"];
        "🔧", Color::LightGreen => ["eslintrc", "prettierrc"];
        "🔧", Color::Yellow => ["tsconfig"];
        "🔧", Color::Yellow => ["babel", "babelrc"];
        "📦", Color::LightBlue => ["webpack"];
        "⚡️", Color::LightMagenta => ["vite"];
        "📦", Color::Red => ["rollup"];

        // Text and documentation files
        "📝", Color::White => ["md", "markdown"];
        "📄", Color::White => ["txt"];
        "📝", Color::LightBlue => ["rst"];
        "📝", Color::LightBlue => ["adoc", "asciidoc"];
        "📚", Color::LightGreen => ["tex", "latex"];
        "📄", Color::White => ["rtf"];
        "📄", Color::LightBlue => ["doc", "docx"];
        "📄", Color::LightBlue => ["odt"];
        "📕", Color::Red => ["pdf"];
        "📚", Color::LightGreen => ["epub"];
        "📚", Color::LightGreen => ["mobi"];

        // Shell scripts
        "📜", Color::Cyan => ["sh", "bash", "zsh", "fish"];
        "⚙️", Color::Gray => ["bat", "cmd"];
        "⚙️", Color::LightBlue => ["ps1", "psm1"];

        // Database files
        "🗄️", Color::LightYellow => ["sql", "sqlite", "db"];
        "🗄️", Color::LightYellow => ["mdb", "accdb"];
        "🗄️", Color::LightYellow => ["dbf"];

        // Archives and compressed files
        "🗜️", Color::Magenta => ["zip", "tar", "gz", "gzip", "bz2", "bzip", "7z", "rar", "xz", "lz", "lzma", "cab"];
        "📦", Color::LightMagenta => ["deb", "rpm", "pkg"];
        "💿", Color::LightRed => ["dmg", "iso"];
        "📦", Color::Cyan => ["flatpak", "snap"];

        // Images
        "🖼️", Color::Green => ["jpg", "jpeg", "png", "gif", "bmp", "ico", "icon", "webp", "tiff", "tif"];
        "🎨", Color::LightGreen => ["svg"];
        "📷", Color::LightGreen => ["raw", "cr2", "nef", "arw"];
        "🎨", Color::LightBlue => ["psd", "ai", "sketch"];
        "🎨", Color::LightBlue => ["fig", "figma"];

        // Audio files
        "🎵", Color::LightMagenta => ["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus", "aiff"];

        // Video files
        "🎥", Color::LightRed => ["mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v", "3gp", "ogv"];

        // Fonts
        "🔤", Color::LightYellow => ["ttf", "otf", "woff", "woff2", "eot"];

        // Spreadsheets
        "📊", Color::Green => ["xls", "xlsx", "xlsm", "ods"];
        "📊", Color::LightGreen => ["csv", "tsv"];

        // Presentations
        "📊", Color::LightYellow => ["ppt", "pptx", "odp"];

        // Executable files
        "⚙️", Color::Red => ["exe", "msi"];
        "📱", Color::LightCyan => ["app", "AppImage"];

        // Log files
        "📜", Color::Yellow => ["log", "logs"];

        // Certificate files
        "🔐", Color::LightGreen => ["crt", "cer", "pem", "key", "p12", "pfx"];

        // Backup files
        "💾", Color::Gray => ["bak", "backup", "old"];

        // Temporary files
        "🗑️", Color::DarkGray => ["tmp", "temp", "cache"];

        // License files
        "📜", Color::LightBlue => ["license", "LICENSE", "licence", "LICENCE"];

        // README files
        "📖", Color::LightCyan => ["readme", "README"];

        // Changelog files
        "📋", Color::LightYellow => ["changelog", "CHANGELOG", "changes", "CHANGES"];

        // Other common files
        "🔒", Color::LightRed => ["lock"];
        "🔢", Color::Gray => ["pid"];
        "🔌", Color::Cyan => ["socket", "sock"];
    }

    file_mappings
});

/// HERE IS WHERE YOU CAN MODIFY FILE TYPE COLORS
pub fn get_file_display_info(item: &FileItem) -> (&str, Style) {
    let (icon, color) = if item.is_dir {
        ("📁", Color::Yellow)
    } else if item.is_symlink {
        ("🔗", Color::Magenta)
    } else {
        FILE_MAPPINGS
            .get(item.name.split('.').last().unwrap_or(""))
            .copied()
            .unwrap_or(("📄", Color::Gray))
    };

    let mut style = Style::default().fg(color);
    
    if item.is_dir {
        style = style.add_modifier(Modifier::BOLD);
    }
    if item.is_hidden {
        style = style.add_modifier(Modifier::DIM);
    }

    (icon, style)
}


/*
    ---------
*/

/*
    ---MAIN UTIL---
*/

/// Parse and validate the target directory
pub fn parse_target_dir(path_arg: Option<String>) -> Result<PathBuf, AppError> {
    match path_arg {
        Some(raw_path) => {
            let cleaned_path = raw_path.trim_matches('"').trim();
            let path = PathBuf::from(cleaned_path);

            if !path.exists() {
                return Err(AppError::DirectoryNotFound(raw_path));
            }
            if !path.is_dir() {
                return Err(AppError::NotADirectory(raw_path));
            }

            path.canonicalize()
                .map_err(|e| AppError::InvalidPath(format!("Cannot canonicalize path: {}", e)))
        },
        
        None => std::env::current_dir().map_err(|e| AppError::Io(e)),

    }
}

/// Get git root and gitignore if they exist
pub fn find_repo(path: &Path) -> Result<(Option<PathBuf>, Option<Gitignore>), AppError> {
    match Repository::discover(path) {
        Ok(repo) => {
            let root = repo.workdir()
                .or_else(|| repo.path().parent())
                .map(|p| p.to_path_buf())
                .ok_or(AppError::GitRepoNoParent)?;
            
            // Build gitignore, but don't fail if the file doesn't exist
            let gitignore_path = root.join(".gitignore");
            let gitignore = if gitignore_path.exists() {
                let (ignore, err) = Gitignore::new(&gitignore_path);
                if let Some(err) = err {
                    eprintln!("Warning: Error parsing .gitignore: {}", err);
                }
                Some(ignore)
            } else {
                None
            };
            
            Ok((Some(root), gitignore))
        },
        Err(_) => Ok((None, None)),
    }
}

/*
    ---FILE READING UTIL---
*/



static TEXT_FILE_MAPPINGS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    
    // NOTE: All keys in this map should be lowercase. The lookup logic will lowercase
    // the input filename/extension before checking the map.

    macro_rules! text_types {
        (
            $(
                $lang:expr => [$($ext:expr),+]
            );*
            $(;)?
        ) => {
            $(
                $(
                    map.insert($ext, $lang);
                )+
            )*
        };
    }
    
    text_types! {
        // --- Filetype by Extension ---

        // Programming Languages
        "rust" => ["rs"];
        "python" => ["py", "pyw", "pyi"];
        "javascript" => ["js", "mjs", "cjs"];
        "typescript" => ["ts", "tsx"];
        "jsx" => ["jsx"];
        "java" => ["java"];
        "cpp" => ["cpp", "c++", "cxx", "cc", "hpp", "h++", "hxx", "h"];
        "c" => ["c"];
        "csharp" => ["cs", "csx"];
        "go" => ["go"];
        "swift" => ["swift"];
        "kotlin" => ["kt", "kts"];
        "scala" => ["scala", "sc"];
        "ruby" => ["rb", "rbw", "rake", "gemspec"];
        "php" => ["php", "php3", "php4", "php5", "phtml"];
        "perl" => ["pl", "pm", "pod"];
        "lua" => ["lua"];
        "r" => ["r", "rmd"];
        "julia" => ["jl"];
        "dart" => ["dart"];
        "haskell" => ["hs", "lhs"];
        "clojure" => ["clj", "cljs", "cljc", "edn"];
        "elixir" => ["ex", "exs"];
        "erlang" => ["erl", "hrl"];
        "ocaml" => ["ml", "mli"];
        "fsharp" => ["fs", "fsx", "fsi"];
        "nim" => ["nim", "nims"];
        "zig" => ["zig"];
        "crystal" => ["cr"];
        "v" => ["v"];
        "solidity" => ["sol"];
        
        // Web Technologies
        "html" => ["html", "htm", "xhtml"];
        "css" => ["css"];
        "scss" => ["scss", "sass"];
        "less" => ["less"];
        "vue" => ["vue"];
        "svelte" => ["svelte"];
        "astro" => ["astro"];
        
        // Shell & Scripts
        "bash" => ["sh", "bash", "zsh", "fish", "ksh", "csh"];
        "powershell" => ["ps1", "psm1", "psd1"];
        "batch" => ["bat", "cmd"];
        
        // Config/Data formats
        "json" => ["json", "jsonc", "json5"];
        "yaml" => ["yaml", "yml"];
        "toml" => ["toml"];
        "xml" => ["xml", "xsd", "xsl", "xslt", "svg"];
        "ini" => ["ini", "cfg", "conf", "config"];
        "kdl" => ["kdl"];
        "properties" => ["properties", "props"];
        "graphql" => ["graphql", "gql"];
        "protobuf" => ["proto"];
        
        // Markup and Documentation
        "markdown" => ["md", "markdown", "mdown", "mdx"];
        "restructuredtext" => ["rst", "rest"];
        "asciidoc" => ["adoc", "asciidoc", "asc"];
        "latex" => ["tex", "latex", "ltx"];
        "org" => ["org"];
        
        // Build & Infra
        "gradle" => ["gradle", "gradle.kts"];
        "maven" => ["pom"];
        "terraform" => ["tf", "tfvars"];
        "hcl" => ["hcl"];
        
        // Database
        "sql" => ["sql", "psql", "mysql"];
        
        // Other
        "diff" => ["diff", "patch"];
        "plaintext" => ["txt", "text", "log", "logs", "out", "csv", "tsv", "lock"];


        // --- Filetype by Full Filename (case-insensitive) ---

        // Build/Project Files
        "makefile" => ["makefile", "mk", "mak"]; // "Makefile" will be lowercased to "makefile"
        "cmake" => ["cmakelists.txt", "cmake"];
        "dockerfile" => ["dockerfile", "containerfile"];
        "go" => ["go.mod", "go.sum"];
        
        // Common Documentation (all lowercase)
        "plaintext" => [
            "license", "licence", "readme", "changelog", "authors", 
            "contributors", "todo", "notes"
        ];
        
        // Common Config Files (including dotfiles, which are full filenames)
        "plaintext" => [
            // Git
            ".gitignore", ".gitattributes", ".gitmodules", ".gitkeep",
            // Docker, NPM, ESLint
            ".dockerignore", ".npmignore", ".eslintignore",
            // Environment variables
            ".env", ".env.example", ".env.sample",
            // Editor/Tooling Config
            ".editorconfig", ".prettierrc", ".eslintrc", ".babelrc",
            // Version managers
            ".nvmrc", ".rvmrc", "ruby-version", "node-version"
        ];
    }
    
    map
});


pub fn get_file_type(path: &Path) -> Option<&'static str> {

    // Check if the filename is in our whitelist and return the corresponding md lang code
    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        if let Some(lang) = TEXT_FILE_MAPPINGS.get(filename.to_lowercase().as_str()) {
            return Some(lang);
        }
    }
    
    // Do the same for extensions
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        if let Some(lang) = TEXT_FILE_MAPPINGS.get(extension.to_lowercase().as_str()) {
            return Some(lang);
        }
    }

    
    
    
    // defaults to `None` if we can't find matches
    None
}

/// Weed out non-text files and safely read them
pub fn read_file_safely(path: &Path, max_size: usize) -> Result<String, AppError> {
    // Check if file type is whitelisted
    if get_file_type(path).is_none() {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());
        return Err(AppError::UnrecognizedFileType { extension });
    }
    
    // Get metadata and check size
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size as u64 {
        return Err(AppError::FileTooLarge { 
            size: metadata.len(), 
            max: max_size 
        });
    }
    
    // Read the file
    let contents = fs::read(path)?;
    
    // Check for binary content (null bytes are a strong indicator)
    if contents.contains(&0) {
        return Err(AppError::BinaryFile);
    }
    
    // Check for excessive control characters
    // Text files might have some control chars (tabs, newlines)
    // but too many suggests binary content
    let control_char_count = contents.iter()
        .filter(|&&b| {
            // Count control chars except common text formatting ones
            (b < 0x20 || b == 0x7F) && !matches!(b, b'\t' | b'\n' | b'\r')
        })
        .count();
    
    // If more than 5% of the file is control characters, it's probably binary
    if control_char_count > contents.len() / 20 {
        return Err(AppError::BinaryFile);
    }
    
    // Try to convert to UTF-8
    match String::from_utf8(contents) {
        Ok(string) => Ok(string),
        Err(e) => {
            // Let's be forgiving with encoding issues
            // Some text files might have a few bad bytes
            let lossy = String::from_utf8_lossy(e.as_bytes());
            
            // Count how many replacement characters we'd need
            let replacement_count = lossy.chars()
                .filter(|&c| c == '\u{FFFD}') // Unicode replacement character
                .count();
            
            // If less than 0.1% of characters need replacement, accept it
            if replacement_count < lossy.len() / 1000 {
                Ok(lossy.into_owned())
            } else {
                Err(AppError::EncodingError)
            }
        }
    }
}