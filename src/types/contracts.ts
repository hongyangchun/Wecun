export type PostFilter = "original" | "all";
export type DateMode = "all" | "range";
export type ExportFormat = "md-single" | "md-obsidian" | "html" | "pdf";

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
  ignore_deleted: boolean;
  min_text_length: number;
  source_type: "profile" | "favorites";
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

export interface ProfileResult {
  basicStats: BasicStats;
  contentAnalysis: ContentAnalysis;
  influenceMetrics: InfluenceMetrics;
  deepProfile: DeepProfile | null;
}

export interface BasicStats {
  totalPosts: number;
  totalOriginal: number;
  totalReposts: number;
  avgTextLength: number;
  maxTextLength: number;
  minTextLength: number;
  postsPerDay: number;
  postsPerWeek: number;
  postsPerMonth: number;
  totalActiveDays: number;
  mostActiveDate: string;
  activeHours: ActiveHours;
  hourlyDistribution: number[];
  weeklyDistribution: number[];
  monthlyDistribution: [string, number][];
}

export interface ActiveHours {
  peakStart: number;
  peakEnd: number;
}

export interface ContentAnalysis {
  topWords: [string, number][];
  topHashtags: [string, number][];
  topMentions: [string, number][];
  avgHashtagsPerPost: number;
  avgMentionsPerPost: number;
  postsWithImagesRatio: number;
  emojiFrequency: [string, number][];
}

export interface InfluenceMetrics {
  sourceDistribution: [string, number][];
  regionDistribution: [string, number][];
  originalRatio: number;
}

export interface DeepProfile {
  personalInfo: PersonalInfo;
  personality: PersonalityAnalysis;
  interests: InterestAnalysis;
  values: ValueAnalysis;
  contentStyle: ContentStyleAnalysis;
  sentiment: AiSentimentAnalysis;
}

export interface PersonalInfo {
  estimatedAgeRange: string;
  zodiacSign: string;
  mbtiType: string;
  gender: string;
  possibleCities: string[];
  possibleOccupation: string;
}

export interface PersonalityAnalysis {
  traits: string[];
  communicationStyle: string;
  socialOrientation: string;
  humorStyle: string;
}

export interface InterestAnalysis {
  music: string[];
  books: string[];
  movies: string[];
  hobbies: string[];
  sports: string[];
  food: string[];
}

export interface ValueAnalysis {
  politicalStance: string;
  philosophy: string;
  worldview: string;
  coreValues: string[];
  attitudeTowardLife: string;
}

export interface ContentStyleAnalysis {
  writingStyle: string;
  commonTopics: string[];
  emotionalTone: string;
  expressionHabits: string;
}

export interface AiSentimentAnalysis {
  overallSentiment: string;
  emotionalStability: string;
  emotionalTriggers: string[];
  happinessIndex: number;
}

export interface HistoryEntry {
  uid: string;
  screen_name: string;
  output_dir: string;
  last_download: string;
  post_count: number;
  source_type?: "profile" | "favorites";
  // New fields
  filter?: "original" | "all";
  include_images?: boolean;
  date_mode?: "all" | "range";
  date_start?: string;
  date_end?: string;
  ignore_deleted?: boolean;
  min_text_length?: number;
}

export type RedownloadMode = "overwrite" | "incremental";
