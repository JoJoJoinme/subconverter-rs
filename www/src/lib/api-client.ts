import { FileAttributes } from 'subconverter-wasm';

/**
 * Response data from the subscription converter API
 */
export interface SubResponseData {
    content: string;
    content_type: string;
    headers: Record<string, string>;
    status_code: number;
}

/**
 * Error data returned from API calls
 */
export interface ErrorData {
    error: string;
    details?: string;
}

/**
 * Parameters for subscription conversion
 */
export interface SubconverterFormParams {
    target: string;
    ver?: number;
    new_name?: boolean;
    url: string;
    group?: string;
    upload_path?: string;
    include?: string;
    exclude?: string;
    groups?: string;
    ruleset?: string;
    config?: string;
    dev_id?: string;
    insert?: boolean;
    prepend?: boolean;
    filename?: string;
    append_type?: boolean;
    emoji?: boolean;
    add_emoji?: boolean;
    remove_emoji?: boolean;
    list?: boolean;
    sort?: boolean;
    sort_script?: string;
    fdn?: boolean;
    rename?: string;
    tfo?: boolean;
    udp?: boolean;
    scv?: boolean;
    tls13?: boolean;
    rename_node?: boolean;
    interval?: number;
    strict?: boolean;
    upload?: boolean;
    token?: string;
    filter?: string;
    script?: boolean;
    classic?: boolean;
    expand?: boolean;
}

export function getWorkerUrl(): string {
    const workerUrl = process.env.NEXT_PUBLIC_WORKER_URL?.trim();
    if (!workerUrl) {
        throw new Error('NEXT_PUBLIC_WORKER_URL is required');
    }
    return workerUrl;
}

/**
 * Rules update request parameters
 */
export interface RulesUpdateRequest {
    config_path?: string;
}

/**
 * Rules update result interfaces
 */
export interface RulesUpdateResult {
    success: boolean;
    message: string;
    details: Record<string, RepoUpdateResult>;
}

export interface RepoUpdateResult {
    repo_name: string;
    files_updated: string[];
    errors: string[];
    status: string;
}

/**
 * Convert a subscription using the subconverter API
 */
export async function convertSubscription(formData: Partial<SubconverterFormParams>): Promise<SubResponseData> {
    const payload: Record<string, any> = {};

    // Create payload with only the explicitly set fields
    Object.keys(formData).forEach(key => {
        if (key in formData) {
            const value = (formData as any)[key];
            // Include the field if it exists in formData
            payload[key] = value;
        }
    });

    // Special handling for emoji flags
    if (payload.emoji === true) {
        // If combined emoji is true, remove the specific flags
        delete payload.add_emoji;
        delete payload.remove_emoji;
    }

    console.log("Sending conversion request with payload:", payload);

    const API_URL = getWorkerUrl();

    const response = await fetch(API_URL, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload),
    });

    const responseText = await response.text();

    if (!response.ok) {
        try {
            const errorObj = JSON.parse(responseText);
            throw errorObj;
        } catch (err) {
            if (typeof err === 'object' && err !== null && 'error' in err) {
                throw err;
            }
            throw {
                error: 'Error from server',
                details: responseText
            };
        }
    }

    const contentType = response.headers.get('Content-Type') || 'text/plain';

    const responseData: SubResponseData = {
        content: responseText,
        content_type: contentType,
        headers: {},
        status_code: response.status
    };

    response.headers.forEach((value, key) => {
        responseData.headers[key] = value;
    });

    return responseData;
}

/**
 * Update rules from configured GitHub repositories
 */
export async function updateRules(configPath?: string): Promise<RulesUpdateResult> {
    // Not supported in static export
    return {
        success: false,
        message: "Not supported in static export mode",
        details: {}
    };
}

/**
 * Read file content from the server
 */
export async function readFile(path: string): Promise<string> {
    // Not supported in static export
    return "";
}

/**
 * Write content to a file on the server
 */
export async function writeFile(path: string, content: string): Promise<void> {
    // Not supported in static export
    return;
}

/**
 * Delete a file or directory on the server
 */
export async function deleteFile(path: string): Promise<void> {
    // Not supported in static export
    return;
}

/**
 * Check if a file exists on the server
 */
export async function checkFileExists(path: string): Promise<boolean> {
    // Not supported in static export
    return false;
}

/**
 * Get file attributes from the server
 */
export async function getFileAttributes(path: string): Promise<FileAttributes> {
    // Not supported in static export
    throw new Error("Not supported in static export");
}

/**
 * Create a directory on the server
 */
export async function createDirectory(path: string): Promise<void> {
    // Not supported in static export
    return;
}

/**
 * List files in a directory
 */
export async function listDirectory(path: string = ''): Promise<any> {
    // Not supported in static export
    return { files: [] };
}

/**
 * Load files from a GitHub repository
 */
export async function loadGitHubDirectory(
    path: string,
    shallow: boolean = true,
    recursive: boolean = true
): Promise<any> {
    // Not supported in static export
    return { result: {} };
}

/**
 * Format a file size number to a human-readable string
 */
export function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/**
 * Format a timestamp (seconds since epoch) to a localized date string
 */
export function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
}

/**
 * Short URL data structure
 */
export interface ShortUrlData {
    id: string;
    target_url: string;
    short_url: string;
    created_at: number;
    last_used?: number;
    use_count: number;
    custom_id: boolean;
    description?: string;
}

/**
 * Short URL creation request
 */
export interface CreateShortUrlRequest {
    target_url: string;
    custom_id?: string;
    description?: string;
}

/**
 * Create a new short URL
 */
export async function createShortUrl(request: CreateShortUrlRequest): Promise<ShortUrlData> {
    // Not supported in static export yet
    throw new Error("Not supported in static export");
}

/**
 * Get list of all short URLs
 */
export async function listShortUrls(): Promise<ShortUrlData[]> {
    return [];
}

/**
 * Delete a short URL
 */
export async function deleteShortUrl(id: string): Promise<void> {
    return;
}

/**
 * Update a short URL
 */
export async function updateShortUrl(id: string, updates: { target_url?: string; description?: string | null; custom_id?: string }): Promise<ShortUrlData> {
    throw new Error("Not supported in static export");
}

/**
 * Move a short URL to a new ID/alias
 */
export async function moveShortUrl(id: string, newId: string): Promise<ShortUrlData> {
    throw new Error("Not supported in static export");
}

/**
 * Interface for application download information
 */
export interface AppDownloadInfo {
    name: string;
    version: string;
    platform: string;
    size: number;
    download_url: string;
    release_date: string;
    description: string;
}

/**
 * Interface for platform configuration
 */
export interface PlatformConfig {
    repo: string;
    asset_pattern: string;
    fallback_url: string;
}

/**
 * Interface for app download configuration
 */
export interface AppDownloadConfig {
    name: string;
    description: string;
    platforms: Record<string, PlatformConfig>;
}

/**
 * Get available application downloads
 */
export async function getAvailableDownloads(): Promise<AppDownloadInfo[]> {
    // Mock data for static export
    return [
        {
            name: "Subconverter",
            version: "0.9.0",
            platform: "windows",
            size: 1024 * 1024 * 10,
            download_url: "https://github.com/tindy2013/subconverter/releases/latest",
            release_date: new Date().toISOString(),
            description: "Windows version"
        },
        {
            name: "Subconverter",
            version: "0.9.0",
            platform: "linux",
            size: 1024 * 1024 * 10,
            download_url: "https://github.com/tindy2013/subconverter/releases/latest",
            release_date: new Date().toISOString(),
            description: "Linux version"
        },
        {
            name: "Subconverter",
            version: "0.9.0",
            platform: "macos",
            size: 1024 * 1024 * 10,
            download_url: "https://github.com/tindy2013/subconverter/releases/latest",
            release_date: new Date().toISOString(),
            description: "macOS version"
        }
    ];
}

/**
 * Download application
 * Returns a URL to initiate the download
 */
export function getDownloadUrl(appId: string, platform: string): string {
    return `https://github.com/tindy2013/subconverter/releases/latest`;
}

/**
 * Get the download configs from the admin API
 * This is only available to admin users
 */
export async function getDownloadConfigs(): Promise<AppDownloadConfig[]> {
    return [];
}

/**
 * Update the download configs via the admin API
 * This is only available to admin users
 */
export async function updateDownloadConfigs(downloads: AppDownloadConfig[]): Promise<boolean> {
    return false;
}

/**
 * Detect the user's operating system
 */
export function detectUserOS(): string {
    if (typeof navigator === 'undefined') return 'unknown';
    const platform = navigator.platform.toLowerCase();

    if (platform.includes('win')) {
        return 'windows';
    } else if (platform.includes('mac')) {
        return 'macos';
    } else if (platform.includes('linux')) {
        return 'linux';
    } else if (/android/i.test(navigator.userAgent)) {
        return 'android';
    } else if (/iphone|ipad|ipod/i.test(navigator.userAgent)) {
        return 'ios';
    }

    return 'unknown';
}

/**
 * Settings management interfaces and functions
 */
export interface ServerSettings {
    general: {
        listen_address: string;
        listen_port: number;
        api_mode: boolean;
        max_pending_conns: number;
        max_concur_threads: number;
        update_interval: number;
        max_allowed_download_size: number;
        log_level: number;
    };
    subscription: {
        default_urls: string[];
        insert_urls: string[];
        prepend_insert: boolean;
        skip_failed_links: boolean;
        enable_insert: boolean;
        enable_sort: boolean;
        filter_script: string;
        sort_script: string;
    };
    rules: {
        enable_rule_gen: boolean;
        update_ruleset_on_request: boolean;
        overwrite_original_rules: boolean;
        async_fetch_ruleset: boolean;
        max_allowed_rulesets: number;
        max_allowed_rules: number;
    };
    cache: {
        cache_subscription: number;
        cache_config: number;
        cache_ruleset: number;
        serve_cache_on_fetch_fail: boolean;
    };
    custom: {
        emojis: Record<string, string>;
        renames: Record<string, string>;
        aliases: Record<string, string>;
    };
}

/**
 * Get current server settings
 */
export async function getServerSettings(): Promise<ServerSettings> {
    throw new Error("Not supported in static export");
}

/**
 * Update server settings
 */
export async function updateServerSettings(settings: Partial<ServerSettings>): Promise<ServerSettings> {
    throw new Error("Not supported in static export");
}

/**
 * Export settings to file
 */
export async function exportSettings(format: 'yaml' | 'toml' | 'ini' = 'yaml'): Promise<Blob> {
    throw new Error("Not supported in static export");
}

/**
 * Import settings from file
 */
export async function importSettings(file: File): Promise<ServerSettings> {
    throw new Error("Not supported in static export");
}

/**
 * Settings file operations
 */

/**
 * Read the pref.yml file content
 * If the file doesn't exist, it will create it from the example file
 */
export async function readSettingsFile(): Promise<string> {
    return "";
}

/**
 * Write content to the pref.yml file
 */
export async function writeSettingsFile(content: string): Promise<void> {
    return;
}

/**
 * Initialize settings with a specific preference path
 * Uses the WASM initialization function directly
 */
export async function initSettings(prefPath: string = ''): Promise<boolean> {
    return true;
}

/**
 * Initializes the Subconverter Webapp VFS.
 * Calls the /api/init endpoint.
 * Returns true if the GitHub load was triggered (likely first run), false otherwise.
 */
export async function initializeWebApp(): Promise<{ success: boolean; githubLoadTriggered: boolean; message: string }> {
    // Mock initialization for static deployment
    return { success: true, githubLoadTriggered: false, message: "Initialized (Mock)" };
}
