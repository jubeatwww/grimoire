//! The library and the asset cache on disk. The only code that moves or deletes anything under
//! the library root.

mod fs_asset_store;
mod fs_library_files;
mod fs_library_folders;
mod fs_library_scanner;
#[cfg(test)]
mod temp_dir;

pub(crate) use fs_asset_store::FsAssetStore;
pub(crate) use fs_library_files::FsLibraryFiles;
pub(crate) use fs_library_folders::FsLibraryFolders;
pub(crate) use fs_library_scanner::FsLibraryScanner;
