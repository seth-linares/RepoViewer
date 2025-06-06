use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::app::FileItem;
use crate::app_error::AppError;

/*
    ---UI UTIL---
*/

static FILE_MAPPINGS: LazyLock<HashMap<&'static str, (&'static str, Color)>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    
    macro_rules! file_types {
        // PATTERN MATCHING SECTION (what the macro expects as input)
        (
            $( // Start of repetition group - this whole thing can repeat
                $icon:expr,     // Capture an expression, call it $icon
                $color:expr     // Capture an expression, call it $color
                => 
                [ // Literal bracket - must appear in input
                    $($ext:expr),+ // Another repetition: one or more expressions separated by commas
                ] // Literal closing bracket
            );* // End repetition group - can repeat zero or more times, separated by semicolons
            $(;)? // Optional trailing semicolon
        ) => {
            // CODE GENERATION SECTION (what gets output)
            $( // For each outer repetition (each icon/color group)
                $( // For each inner repetition (each extension in the brackets)
                    map.insert($ext, ($icon, $color)); // Generate this line
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

    map
});


pub fn get_file_display_info(item: &FileItem) -> (&str, Style) {
    let (icon, color) = if item.is_dir {
        ("📁", Color::Blue)
    } else if item.is_symlink {
        ("🔗", Color::Magenta)
    } else {
        FILE_MAPPINGS
            .get(item.name.split('.').last().unwrap_or(""))
            .copied()
            .unwrap_or(("📄", Color::Gray))
    };

    let mut style = if item.is_dir {
        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
    } else if item.is_symlink {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(color)
    };

    if item.is_hidden {
        style = style.add_modifier(Modifier::DIM);
    }

    (icon, style)
}

// fn get_file_color(name: &str) -> Style {
//     let color = FILE_MAPPINGS
//         .get(name.split('.').last().unwrap_or(""))
//         .map(|(_, color)| *color)
//         .unwrap_or(Color::Gray);


//     Style::default().fg(color)
// }

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





