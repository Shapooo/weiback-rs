export enum TaskType {
  BackupUser = 'BackupUser',
  BackupFavorites = 'BackupFavorites',
  UnfavoritePosts = 'UnfavoritePosts',
  Export = 'Export',
  CleanupPictures = 'CleanupPictures',
  CleanupAvatars = 'CleanupAvatars',
  CleanupInvalidPosts = 'CleanupInvalidPosts',
  RebackupPosts = 'RebackupPosts',
  RebackupMissingImages = 'RebackupMissingImages',
  CleanupInvalidPictures = 'CleanupInvalidPictures',
}

export interface CleanupInvalidPostsOptions {
  clean_retweeted_invalid: boolean
}

export enum ResolutionPolicy {
  Highest = 'Highest',
  Lowest = 'Lowest',
}

export interface CleanupPicturesOptions {
  policy: ResolutionPolicy
}

export enum TaskStatus {
  InProgress = 'InProgress',
  Completed = 'Completed',
  Failed = 'Failed',
}

export interface Task {
  id: number
  task_type: TaskType
  description: string
  status: TaskStatus
  progress: number
  total: number
  error: string | null
}

export enum TaskErrorType {
  DownloadMedia = 'DownloadMedia',
}

export interface TaskError {
  error_type: { [key in TaskErrorType]?: string } // e.g., { "DownloadMedia": "http://..." }
  message: string
}

// --- From OnlineBackup ---
export enum BackupType {
  Normal = 'Normal',
  Original = 'Original',
  Picture = 'Picture',
  Video = 'Video',
  Article = 'Article',
}
