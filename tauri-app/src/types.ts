// This file defines TypeScript types that mirror the Rust structs from the backend.

export enum TaskType {
    BackupUser = "BackupUser",
    BackupFavorites = "BackupFavorites",
    UnfavoritePosts = "UnfavoritePosts",
    Export = "Export",
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


// --- Types from PostDisplay ---

export interface UrlStructItem {
    long_url: string | null;
    short_url: string;
    url_title: string;
}

export interface User {
    id: number;
    screen_name: string;
}

export interface Post {
    id: number;
    text: string;
    favorited: boolean;
    created_at: string;
    user: User | null;
    retweeted_status?: Post | null;
    url_struct: UrlStructItem[] | null;
}

export interface PostInfo {
    post: Post;
    avatar_id: string | null;
    emoji_map: Record<string, string>;
    standalone_ids: string[];
    inline_map: Record<string, string>,
}
