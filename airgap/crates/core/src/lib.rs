pub mod config;
pub mod exporter;
pub mod importer;
pub mod io;
pub mod path_safety;
pub mod search;
pub mod trash;
pub mod vault;
pub mod watcher;

pub use config::{load_config, save_config, AppConfig};
pub use exporter::{export_vault_zip, generate_html_document, ExportError};
pub use importer::{import_zip, ConflictPolicy, ImportError, ImportSummary};
pub use io::{
    apply_line_ending, detect_line_ending, read_note, strip_bom, write_atomic, IoError, LineEnding,
};
pub use path_safety::{sanitize_path, validate_filename, PathSafetyError};
pub use search::{global_search, quick_open_search, QuickOpenMatch, SearchResult};
pub use trash::{delete_to_trash, TrashError};
pub use vault::{TreeNode, Vault, VaultError, VaultItemType};
pub use watcher::{VaultWatcher, WatcherError};
