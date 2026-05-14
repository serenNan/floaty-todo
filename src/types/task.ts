export type SourceKind = 'folder' | 'file';

export type QuickActionKind = 'vscode' | 'terminal' | 'claude_code' | 'reveal';

export type Quadrant =
  | 'urgent_important'
  | 'not_urgent_important'
  | 'urgent_not_important'
  | 'not_urgent_not_important';

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
  quadrant: Quadrant | null;
}

export interface AppConfig {
  sources: Source[];
  default_source_id: string | null;
  inbox_file: string;
  always_on_top: boolean;
  /// Keyed by file path string (canonical, dunce-simplified on the Rust side).
  file_labels: Record<string, string>;
  enabled_quick_actions: QuickActionKind[];
  /// Folder mirroring every source via OS-level filesystem links so AI
  /// tools can find every project's TODO in one place. `null` = off.
  hub_folder: string | null;
  auto_create_quadrant_headers: boolean;
}
