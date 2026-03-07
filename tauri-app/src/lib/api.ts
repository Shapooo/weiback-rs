import { invoke } from '@tauri-apps/api/core';
import {
    User,
    Task,
    TaskError,
    PaginatedPostInfo,
    PostQuery,
    ExportJobOptions,
    BackupType,
    ResolutionPolicy,
    CleanupInvalidPostsOptions,
} from '../types';
import { Config } from '../types/config';

// Backend
export type BackendStatus =
    | { status: 'Uninitialized' }
    | { status: 'Running' }
    | { status: 'Error', message: string };

export const getBackendStatus = () => invoke<BackendStatus>('get_backend_status');
export const initBackend = () => invoke<BackendStatus>('init_backend');

// Auth
export const loginState = () => invoke<User | null>('login_state');
export const getSmsCode = (phoneNumber: string) => invoke('get_sms_code', { phoneNumber });
export const login = (smsCode: string) => invoke<User>('login', { smsCode });

// Tasks
export const getCurrentTaskStatus = () => invoke<Task | null>('get_current_task_status');
export const getAndClearTaskErrors = () => invoke<TaskError[]>('get_and_clear_task_errors');

// Backup
export const backupUser = (uid: string, numPages: number, backupType: BackupType) =>
    invoke('backup_user', { uid, numPages, backupType });
export const backupFavorites = (numPages: number) => invoke('backup_favorites', { numPages });
export const unfavoritePosts = () => invoke('unfavorite_posts');
export const rebackupPosts = (query: PostQuery) => invoke('rebackup_posts', { query });

// Posts
export const queryLocalPosts = (query: PostQuery) =>
    invoke<PaginatedPostInfo>('query_local_posts', { query });
export const deletePost = (id: string) => invoke('delete_post', { id });
export const rebackupPost = (id: string) => invoke('rebackup_post', { id });

// Users
export const getUsernameById = (uid: string) => invoke<string | null>('get_username_by_id', { uid });
export const searchIdByUsernamePrefix = (prefix: string) =>
    invoke<User[]>('search_id_by_username_prefix', { prefix });

// Export
export const exportPosts = (options: ExportJobOptions) => invoke('export_posts', { options });

// Pictures
export const getPictureBlob = (id: string) => invoke<ArrayBuffer>('get_picture_blob', { id });
export const cleanupPictures = (policy: ResolutionPolicy) => invoke('cleanup_pictures', { options: { policy } });
export const cleanupInvalidAvatars = () => invoke('cleanup_invalid_avatars');
export const cleanupInvalidPosts = (options: CleanupInvalidPostsOptions) => invoke('cleanup_invalid_posts', { options });

// Config
export const getConfig = () => invoke<Config>('get_config_command');
export const setConfig = (config: Config) => invoke('set_config_command', { config });
