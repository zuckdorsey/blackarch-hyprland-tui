use crate::models::{BlackArchTool, PackageSearchResult};

#[derive(Debug)]
#[allow(dead_code)]
pub enum WorkerCommand {
    LoadInitialData,
    SyncBasicCache,
    LoadToolDetail {
        package_name: String,
        request_id: u64,
    },
    RefreshToolDetail {
        package_name: String,
        request_id: u64,
    },
    SearchPackages {
        query: String,
        request_id: u64,
    },
    ClearPackageCache {
        package_name: String,
        request_id: u64,
    },
    RunTool {
        package_name: String,
        executable: String,
        request_id: u64,
    },
    InstallPackage {
        package_name: String,
        request_id: u64,
    },
    InstallPackages {
        package_names: Vec<String>,
        request_id: u64,
    },
    RemovePackage {
        package_name: String,
        request_id: u64,
    },
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum WorkerEvent {
    InitialDataLoaded {
        tools: Vec<BlackArchTool>,
        categories: Vec<String>,
    },
    BasicCacheSynced {
        tools: Vec<BlackArchTool>,
        categories: Vec<String>,
    },
    ToolDetailLoaded {
        package_name: String,
        request_id: u64,
        tool: BlackArchTool,
    },
    ToolDetailFailed {
        package_name: String,
        request_id: u64,
        error: String,
    },
    SearchCompleted {
        query: String,
        request_id: u64,
        results: Vec<PackageSearchResult>,
    },
    CacheCleared {
        package_name: String,
        request_id: u64,
    },
    ToolRunStarted {
        package_name: String,
        executable: String,
        request_id: u64,
    },
    ToolRunFailed {
        package_name: String,
        error: String,
        request_id: u64,
    },
    PackageInstallStarted {
        package_name: String,
        request_id: u64,
    },
    PackagesInstallStarted {
        package_names: Vec<String>,
        request_id: u64,
    },
    PackageInstallFinished {
        package_name: String,
        request_id: u64,
        refreshed_tool: Option<BlackArchTool>,
    },
    PackagesInstallFinished {
        package_names: Vec<String>,
        request_id: u64,
        refreshed_tools: Vec<BlackArchTool>,
    },
    PackageInstallFailed {
        package_name: String,
        request_id: u64,
        error: String,
    },
    PackagesInstallFailed {
        package_names: Vec<String>,
        request_id: u64,
        error: String,
    },
    PackageRemoveStarted {
        package_name: String,
        request_id: u64,
    },
    PackageRemoveFinished {
        package_name: String,
        request_id: u64,
        refreshed_tool: BlackArchTool,
        refreshed: bool,
    },
    PackageRemoveFailed {
        package_name: String,
        request_id: u64,
        error: String,
    },
    TaskFailed {
        label: String,
        error: String,
    },
}
