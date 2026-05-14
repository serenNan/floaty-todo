export type SourceKind = 'folder' | 'file';

export interface Source {
  id: string;
  path: string;
  kind: SourceKind;
  label: string | null;
  project_root: string | null;
}

export interface Task {
  id: string;
  text: string;
  completed: boolean;
  source_file: string;
  line_number: number;
  indent: number;
  source_id: string;
}

export interface AppConfig {
  sources: Source[];
  default_source_id: string | null;
  inbox_file: string;
  always_on_top: boolean;
  /// Keyed by file path string (canonical, dunce-simplified on the Rust side).
  file_labels: Record<string, string>;
}
