export interface Task {
  id: string;
  text: string;
  completed: boolean;
  source_file: string;
  line_number: number;
  indent: number;
}

export interface AppConfig {
  vault_path: string | null;
  inbox_file: string;
  always_on_top: boolean;
}
