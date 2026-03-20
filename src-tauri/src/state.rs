use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::merge::{append_text, MergeStrategy};

#[derive(Default)]
pub struct SharedState {
    pub inner: Mutex<AppState>,
}

#[derive(Default)]
pub struct AppState {
    pub next_snapshot_id: u64,
    pub next_segment_id: u64,
    pub current_snapshot: Option<StoredSnapshot>,
    pub segments: Vec<StoredSegment>,
    pub merged_text: String,
}

#[derive(Clone)]
pub struct StoredSnapshot {
    pub id: u64,
    pub png_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPayload {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub data_url: String,
}

impl StoredSnapshot {
    pub fn to_payload(&self) -> SnapshotPayload {
        SnapshotPayload {
            id: self.id,
            width: self.width,
            height: self.height,
            data_url: format!("data:image/png;base64,{}", STANDARD.encode(&self.png_bytes)),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct StoredSegment {
    pub id: u64,
    pub order: usize,
    pub snapshot_id: u64,
    pub selection: SelectionRect,
    pub recognized_text: String,
    pub merge_strategy: MergeStrategy,
    pub overlap_lines: usize,
    pub created_at_epoch_ms: u128,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentPayload {
    pub id: u64,
    pub order: usize,
    pub snapshot_id: u64,
    pub selection: SelectionRect,
    pub recognized_text: String,
    pub merge_strategy: String,
    pub overlap_lines: usize,
    pub created_at_epoch_ms: u128,
}

impl From<&StoredSegment> for SegmentPayload {
    fn from(value: &StoredSegment) -> Self {
        Self {
            id: value.id,
            order: value.order,
            snapshot_id: value.snapshot_id,
            selection: value.selection.clone(),
            recognized_text: value.recognized_text.clone(),
            merge_strategy: value.merge_strategy.as_str().to_string(),
            overlap_lines: value.overlap_lines,
            created_at_epoch_ms: value.created_at_epoch_ms,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub merged_text: String,
    pub segments: Vec<SegmentPayload>,
    pub current_snapshot: Option<SnapshotSummary>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: u64,
    pub width: u32,
    pub height: u32,
}

impl AppState {
    pub fn to_payload(&self) -> AppStatePayload {
        AppStatePayload {
            merged_text: self.merged_text.clone(),
            segments: self.segments.iter().map(SegmentPayload::from).collect(),
            current_snapshot: self
                .current_snapshot
                .as_ref()
                .map(|snapshot| SnapshotSummary {
                    id: snapshot.id,
                    width: snapshot.width,
                    height: snapshot.height,
                }),
        }
    }

    pub fn clear(&mut self) {
        self.current_snapshot = None;
        self.segments.clear();
        self.merged_text.clear();
    }

    pub fn store_snapshot(
        &mut self,
        png_bytes: Vec<u8>,
        width: u32,
        height: u32,
    ) -> SnapshotPayload {
        self.next_snapshot_id += 1;
        let snapshot = StoredSnapshot {
            id: self.next_snapshot_id,
            png_bytes,
            width,
            height,
        };
        let payload = snapshot.to_payload();
        self.current_snapshot = Some(snapshot);
        payload
    }

    pub fn push_segment(
        &mut self,
        snapshot_id: u64,
        selection: SelectionRect,
        recognized_text: String,
    ) {
        self.next_segment_id += 1;
        self.segments.push(StoredSegment {
            id: self.next_segment_id,
            order: self.segments.len() + 1,
            snapshot_id,
            selection,
            recognized_text,
            merge_strategy: MergeStrategy::Initial,
            overlap_lines: 0,
            created_at_epoch_ms: now_epoch_ms(),
        });
        self.rebuild_merge();
    }

    pub fn undo_last_segment(&mut self) {
        self.segments.pop();
        self.rebuild_merge();
    }

    fn rebuild_merge(&mut self) {
        let mut merged_text = String::new();

        for segment in &mut self.segments {
            let outcome = append_text(&merged_text, &segment.recognized_text);
            segment.merge_strategy = outcome.strategy;
            segment.overlap_lines = outcome.overlap_lines;
            merged_text = outcome.merged_text;
        }

        self.merged_text = merged_text;
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
