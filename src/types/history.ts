import type { Quadrant } from './task';

export interface TaskLineState {
  done: boolean;
  text: string;
  quadrant?: Quadrant | null;
}

export interface LineSnapshot {
  line: number;
  state?: TaskLineState | null;
  raw: string;
  hash: string;
}

export interface DiffSummary {
  added: number;
  removed: number;
  modified: number;
}

interface HistoryEventBase {
  id: string;
  ts: string;
  source_id: string;
  file: string;
}

export type HistoryEvent =
  | (HistoryEventBase & {
      kind: 'toggle';
      task_id: string;
      before: LineSnapshot;
      after: LineSnapshot;
    })
  | (HistoryEventBase & {
      kind: 'edit';
      task_id: string;
      before: LineSnapshot;
      after: LineSnapshot;
    })
  | (HistoryEventBase & {
      kind: 'add';
      after: LineSnapshot;
    })
  | (HistoryEventBase & {
      kind: 'move';
      task_id: string;
      before: LineSnapshot;
      after: LineSnapshot;
    })
  | (HistoryEventBase & {
      kind: 'external_edit';
      diff_summary: DiffSummary;
      size_bytes_delta: number;
      note: string;
    });

