use std::path::Path;

use tenui_core::Color;

/// Icon rendering mode determining fallback strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// Crisp Nerd Font glyph (Private Use Area unicode symbols).
    #[default]
    NerdFont,
    /// Clean standard Unicode symbol fallback (e.g. `✔`, `⚠`, `✖`, `ℹ`, `📁`).
    Unicode,
    /// Plain ASCII fallback (e.g. `[OK]`, `[!]`, `[X]`, `[i]`, `[DIR]`).
    Ascii,
}

/// Standardized icon symbols for files, development tools, VCS, and UI status badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    // Filesystem
    Folder,
    FolderOpen,
    File,
    FileCode,
    FileText,
    FileArchive,
    FileBinary,
    Symlink,

    // Programming Languages & Formats
    Rust,
    Go,
    Python,
    Javascript,
    Typescript,
    C,
    Cpp,
    Html,
    Css,
    Markdown,
    Toml,
    Yaml,
    Json,
    Shell,
    Sql,
    Docker,

    // VCS / Git
    GitBranch,
    GitCommit,
    GitMerge,
    GitDiff,
    GitRepo,

    // Diagnostics & UI Status
    Success,
    Warning,
    Error,
    Info,
    Question,
    Check,
    Cross,
    Dot,

    // System & Hardware
    Terminal,
    Cpu,
    Memory,
    Disk,
    Network,
    Cloud,
    Clock,
    Lock,
    Unlock,
    Key,
    User,
    Gear,
}

impl Icon {
    /// Returns the Nerd Font glyph string for this icon.
    pub const fn nerd_glyph(&self) -> &'static str {
        match self {
            Self::Folder => "",
            Self::FolderOpen => "",
            Self::File => "",
            Self::FileCode => "",
            Self::FileText => "",
            Self::FileArchive => "",
            Self::FileBinary => "",
            Self::Symlink => "",

            Self::Rust => "",
            Self::Go => "",
            Self::Python => "",
            Self::Javascript => "",
            Self::Typescript => "",
            Self::C => "",
            Self::Cpp => "",
            Self::Html => "",
            Self::Css => "",
            Self::Markdown => "",
            Self::Toml => "",
            Self::Yaml => "",
            Self::Json => "",
            Self::Shell => "",
            Self::Sql => "",
            Self::Docker => "",

            Self::GitBranch => "",
            Self::GitCommit => "",
            Self::GitMerge => "",
            Self::GitDiff => "",
            Self::GitRepo => "",

            Self::Success => "",
            Self::Warning => "",
            Self::Error => "",
            Self::Info => "",
            Self::Question => "",
            Self::Check => "",
            Self::Cross => "",
            Self::Dot => "",

            Self::Terminal => "",
            Self::Cpu => "",
            Self::Memory => "",
            Self::Disk => "",
            Self::Network => "󰛳",
            Self::Cloud => "",
            Self::Clock => "",
            Self::Lock => "",
            Self::Unlock => "",
            Self::Key => "",
            Self::User => "",
            Self::Gear => "",
        }
    }

    /// Returns the standard Unicode fallback symbol for this icon.
    pub const fn unicode_fallback(&self) -> &'static str {
        match self {
            Self::Folder => "📁",
            Self::FolderOpen => "📂",
            Self::File => "📄",
            Self::FileCode => "📜",
            Self::FileText => "📝",
            Self::FileArchive => "📦",
            Self::FileBinary => "⚙",
            Self::Symlink => "🔗",

            Self::Rust => "🦀",
            Self::Go => "🐹",
            Self::Python => "🐍",
            Self::Javascript => "🟨",
            Self::Typescript => "🔷",
            Self::C => "🅲",
            Self::Cpp => "🅲",
            Self::Html => "🌐",
            Self::Css => "🎨",
            Self::Markdown => "Ⓜ",
            Self::Toml | Self::Yaml | Self::Json => "⚙",
            Self::Shell => "🐚",
            Self::Sql => "🗄",
            Self::Docker => "🐳",

            Self::GitBranch => "⌥",
            Self::GitCommit => "●",
            Self::GitMerge => "⑂",
            Self::GitDiff => "±",
            Self::GitRepo => "📚",

            Self::Success => "✔",
            Self::Warning => "⚠",
            Self::Error => "✖",
            Self::Info => "ℹ",
            Self::Question => "?",
            Self::Check => "✓",
            Self::Cross => "✗",
            Self::Dot => "•",

            Self::Terminal => "▶",
            Self::Cpu => "■",
            Self::Memory => "▤",
            Self::Disk => "💾",
            Self::Network => "⚡",
            Self::Cloud => "☁",
            Self::Clock => "⏱",
            Self::Lock => "🔒",
            Self::Unlock => "🔓",
            Self::Key => "🔑",
            Self::User => "👤",
            Self::Gear => "⚙",
        }
    }

    /// Returns the plain ASCII fallback string for this icon.
    pub const fn ascii_fallback(&self) -> &'static str {
        match self {
            Self::Folder | Self::FolderOpen => "[DIR]",
            Self::File => "[FILE]",
            Self::FileCode => "[CODE]",
            Self::FileText => "[TXT]",
            Self::FileArchive => "[ZIP]",
            Self::FileBinary => "[BIN]",
            Self::Symlink => "->",

            Self::Rust => "[RS]",
            Self::Go => "[GO]",
            Self::Python => "[PY]",
            Self::Javascript => "[JS]",
            Self::Typescript => "[TS]",
            Self::C => "[C]",
            Self::Cpp => "[C++]",
            Self::Html => "[HTML]",
            Self::Css => "[CSS]",
            Self::Markdown => "[MD]",
            Self::Toml | Self::Yaml | Self::Json => "[CFG]",
            Self::Shell => "[SH]",
            Self::Sql => "[SQL]",
            Self::Docker => "[DOCKER]",

            Self::GitBranch => "[branch]",
            Self::GitCommit => "[commit]",
            Self::GitMerge => "[merge]",
            Self::GitDiff => "[diff]",
            Self::GitRepo => "[repo]",

            Self::Success | Self::Check => "[OK]",
            Self::Warning => "[!]",
            Self::Error | Self::Cross => "[X]",
            Self::Info => "[i]",
            Self::Question => "[?]",
            Self::Dot => "*",

            Self::Terminal => ">_",
            Self::Cpu => "[CPU]",
            Self::Memory => "[MEM]",
            Self::Disk => "[DISK]",
            Self::Network => "[NET]",
            Self::Cloud => "[CLOUD]",
            Self::Clock => "[TIME]",
            Self::Lock => "[LOCK]",
            Self::Unlock => "[OPEN]",
            Self::Key => "[KEY]",
            Self::User => "[USER]",
            Self::Gear => "[SET]",
        }
    }

    /// Resolves the icon string for the specified rendering mode.
    pub fn as_str(&self, mode: IconMode) -> &'static str {
        match mode {
            IconMode::NerdFont => self.nerd_glyph(),
            IconMode::Unicode => self.unicode_fallback(),
            IconMode::Ascii => self.ascii_fallback(),
        }
    }

    /// Returns the canonical semantic color for this icon.
    pub const fn color(&self) -> Color {
        match self {
            Self::Folder | Self::FolderOpen => Color::BLUE,
            Self::File | Self::FileText => Color::GRAY,
            Self::FileArchive => Color::Red,
            Self::FileBinary => Color::DarkGray,
            Self::Symlink => Color::Cyan,

            Self::Rust => Color::Rgb(249, 115, 22), // Rust Orange
            Self::Go => Color::Rgb(0, 173, 216),    // Go Cyan
            Self::Python => Color::YELLOW,
            Self::Javascript | Self::Typescript => Color::YELLOW,
            Self::C | Self::Cpp => Color::Rgb(56, 189, 248),
            Self::Html => Color::Rgb(227, 79, 38), // HTML Orange
            Self::Css => Color::Rgb(38, 77, 228),  // CSS Blue
            Self::Markdown => Color::WHITE,
            Self::Toml | Self::Yaml | Self::Json => Color::LightMagenta,
            Self::Shell => Color::LightGreen,
            Self::Sql => Color::Rgb(227, 140, 45),
            Self::Docker => Color::Rgb(13, 183, 237),

            Self::GitBranch | Self::GitCommit | Self::GitMerge | Self::GitDiff | Self::GitRepo => {
                Color::Rgb(240, 80, 50) // Git Red/Orange
            }

            Self::Success | Self::Check => Color::GREEN,
            Self::Warning => Color::YELLOW,
            Self::Error | Self::Cross => Color::RED,
            Self::Info => Color::BLUE,
            Self::Question => Color::Cyan,
            Self::Dot => Color::WHITE,

            Self::Terminal => Color::LightGreen,
            Self::Cpu | Self::Memory | Self::Disk | Self::Network | Self::Cloud => Color::Cyan,
            Self::Clock => Color::LightYellow,
            Self::Lock => Color::LightRed,
            Self::Unlock => Color::LightGreen,
            Self::Key => Color::YELLOW,
            Self::User => Color::LightBlue,
            Self::Gear => Color::GRAY,
            _ => Color::Reset,
        }
    }

    /// Convenient helper resolving `(icon_str, color)` based on a boolean `use_nerd_fonts` flag.
    pub fn resolve(&self, use_nerd_fonts: bool) -> (&'static str, Color) {
        let mode = if use_nerd_fonts {
            IconMode::NerdFont
        } else {
            IconMode::Ascii
        };
        (self.as_str(mode), self.color())
    }

    /// Infers an icon from a file extension (e.g. `"rs"`, `"py"`, `"toml"`).
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "go" => Self::Go,
            "py" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" | "jsx" => Self::Javascript,
            "ts" | "tsx" => Self::Typescript,
            "c" | "h" => Self::C,
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Self::Cpp,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "md" | "markdown" => Self::Markdown,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "sh" | "bash" | "zsh" | "fish" => Self::Shell,
            "sql" => Self::Sql,
            "dockerfile" => Self::Docker,
            "zip" | "tar" | "gz" | "xz" | "7z" | "bz2" => Self::FileArchive,
            "exe" | "bin" | "so" | "dll" | "dylib" | "o" | "a" => Self::FileBinary,
            "txt" | "log" | "csv" | "tsv" => Self::FileText,
            _ => Self::File,
        }
    }

    /// Infers an icon for a filesystem entry path and directory flag.
    pub fn for_path(path: &str, is_dir: bool) -> Self {
        if is_dir {
            return Self::Folder;
        }
        let ext = Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("");
        Self::from_extension(ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_resolution() {
        assert_eq!(Icon::Rust.nerd_glyph(), "");
        assert_eq!(Icon::Rust.ascii_fallback(), "[RS]");
        assert_eq!(Icon::Folder.nerd_glyph(), "");
        assert_eq!(Icon::Folder.ascii_fallback(), "[DIR]");

        let (glyph, col) = Icon::Go.resolve(true);
        assert_eq!(glyph, "");
        assert_eq!(col, Color::Rgb(0, 173, 216));

        let (ascii, _) = Icon::Go.resolve(false);
        assert_eq!(ascii, "[GO]");
    }

    #[test]
    fn test_icon_from_extension() {
        assert_eq!(Icon::from_extension("rs"), Icon::Rust);
        assert_eq!(Icon::from_extension("go"), Icon::Go);
        assert_eq!(Icon::from_extension("py"), Icon::Python);
        assert_eq!(Icon::from_extension("toml"), Icon::Toml);
        assert_eq!(Icon::from_extension("unknown_xyz"), Icon::File);
    }
}
