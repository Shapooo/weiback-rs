export enum TaskType {
    BackupUser = "BackupUser",
    BackupFavorites = "BackupFavorites",
    UnfavoritePosts = "UnfavoritePosts",
    Export = "Export",
    CleanupPictures = "CleanupPictures",
    CleanupAvatars = "CleanupAvatars",
}

export enum ResolutionPolicy {
    Highest = "Highest",
    Lowest = "Lowest",
}

export interface CleanupPicturesOptions {
    policy: ResolutionPolicy;
}

export enum TaskStatus {
    InProgress = "InProgress",
    Completed = "Completed",
    Failed = "Failed",
}

export interface Task {
    id: number;
    task_type: TaskType;
    description: string;
    status: TaskStatus;
    progress: number;
    total: number;
    error: string | null;
}

export enum SubTaskErrorType {
    DownloadMedia = "DownloadMedia",
}

export interface SubTaskError {
    error_type: { [key in SubTaskErrorType]?: string }; // e.g., { "DownloadMedia": "http://..." }
    message: string;
}

// --- From OnlineBackup ---
export enum BackupType {
    Normal = "Normal",
    Original = "Original",
    Picture = "Picture",
    Video = "Video",
    Article = "Article",
}
