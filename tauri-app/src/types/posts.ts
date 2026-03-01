import { User } from './user';

// --- From PostDisplay ---
export interface UrlStructItem {
    long_url: string | null;
    short_url: string;
    url_title: string;
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

// --- From LocalExport ---
export interface PaginatedPostInfo {
    posts: PostInfo[];
    total_items: number;
}

export interface PostQuery {
    user_id?: number;
    start_date?: number; // Unix timestamp
    end_date?: number;   // Unix timestamp
    search_term?: string;
    is_favorited: boolean;
    reverse_order: boolean;
    page: number;
    posts_per_page: number;
}

export interface ExportOutputConfig {
    task_name: string;
    export_dir: string;
}

export interface ExportJobOptions {
    query: PostQuery;
    output: ExportOutputConfig;
}
