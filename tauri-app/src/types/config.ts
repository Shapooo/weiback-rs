// Mirror of the Rust Config struct
export interface SdkConfig {
    fav_count: number;
    status_count: number;
    retry_times: number;
}

export enum PictureDefinition {
    RealOriginal = "RealOriginal",
    Thumbnail = "Thumbnail",
    Bmiddle = "Bmiddle",
    Large = "Large",
    Original = "Original",
    Mw2000 = "Mw2000",
    Largest = "Largest",
}

export interface Config {
    db_path: string;
    session_path: string;
    download_pictures: boolean;
    picture_definition: PictureDefinition;
    backup_task_interval: number; // it's a Duration on Rust side, but serialized as seconds
    other_task_interval: number; // same
    posts_per_html: number;
    picture_path: string;
    video_path: string;
    sdk_config: SdkConfig;
    dev_mode_out_dir?: string;
}
