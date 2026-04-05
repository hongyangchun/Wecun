export type PostFilter = "original" | "all";
export type DateMode = "all" | "range";
export type ExportFormat = "md-single" | "md-multi" | "html";

export interface DownloadRequest {
  uid: string;
  cookie: string;
  filter: PostFilter;
  include_images: boolean;
  date_range: {
    start_timestamp: number | null;
    end_timestamp: number | null;
  };
  output_dir: string;
  min_text_length: number;
}

export interface ExportRequest {
  output_dir: string;
  export_format: ExportFormat;
}

export type ProgressPhase =
  | "FetchingUserInfo"
  | "FetchingPostList"
  | "FetchingLongText"
  | "DownloadingImages"
  | "Exporting"
  | "Complete"
  | "Error"
  | "Cancelled"
  | "Resuming";

export interface ProgressEvent {
  phase: ProgressPhase;
  current: number;
  total: number;
  message: string;
}

export type DownloadStatus = "ready" | "downloading" | "done" | "cancelled" | "error";
