use actix_web::web;
use chrono::Utc;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use crate::constants::ticker::{CLEANUP_INTERVAL_TICKS, INTERVAL_MS};
use crate::models::AppState;
use crate::models::HasId;
use crate::services::content_service::OutputType;
use crate::services::{ContentService, DlsService, MotService};
use crate::errors::ServiceResult;

pub struct TickerService;

impl TickerService {
    pub(crate) fn update_output_with<DlsFn, MotFn>(
        state: &mut AppState,
        now: chrono::DateTime<Utc>,
        dls_file: &PathBuf,
        mot_dir: &PathBuf,
        previous_output_type: &mut Option<OutputType>,
        previous_content_id: &mut Option<Uuid>,
        dls_updater: DlsFn,
        mot_updater: MotFn,
    ) -> ServiceResult<bool>
    where
        DlsFn: Fn(&PathBuf, &mut AppState) -> ServiceResult<()>,
        MotFn: Fn(&PathBuf, &mut AppState) -> ServiceResult<()>,
    {
        let current_output_type = ContentService::get_active_output_type(state, now);

        let current_content_id = match current_output_type {
            OutputType::Track => state.track.as_ref().and_then(|t| t.get_id()),
            OutputType::Program => state.program.as_ref().and_then(|p| p.get_id()),
            OutputType::Station => state.station.as_ref().and_then(|s| s.get_id()),
        };

        let has_changed = match &*previous_output_type {
            None => true,
            Some(prev) => prev != &current_output_type || *previous_content_id != current_content_id,
        };

        if has_changed {
            info!("Ticker: Content changed at {}", now);
            match current_output_type {
                OutputType::Track => {
                    if let Some(track) = &state.track {
                        let artist_display = track.item.artist.as_deref().unwrap_or("(no artist)");
                        info!(
                            "New content: Track \"{}\" by \"{}\" (ID: {:?})",
                            track.item.title,
                            artist_display,
                            track.get_id()
                        );
                    }
                }
                OutputType::Program => {
                    if let Some(program) = &state.program {
                        info!(
                            "New content: Program \"{}\" (ID: {:?})",
                            program.name,
                            program.get_id()
                        );
                    }
                }
                OutputType::Station => {
                    if let Some(station) = &state.station {
                        info!("New content: Station \"{}\"", station.name);
                    }
                }
            }

            dls_updater(dls_file, state)?;
            mot_updater(mot_dir, state)?;

            *previous_output_type = Some(current_output_type);
            *previous_content_id = current_content_id;

            return Ok(true);
        }

        Ok(false)
    }

    fn update_output(
        state: &mut AppState,
        now: chrono::DateTime<Utc>,
        dls_file: &PathBuf,
        mot_dir: &PathBuf,
        previous_output_type: &mut Option<OutputType>,
        previous_content_id: &mut Option<Uuid>,
    ) {
        if let Err(e) = Self::update_output_with(
            state,
            now,
            dls_file,
            mot_dir,
            previous_output_type,
            previous_content_id,
            |p, s| DlsService::update_output_file(p, s),
            |p, s| MotService::update_mot_output(p, s),
        ) {
            error!("Ticker: Failed to update outputs: {}", e);
        }
    }

    pub(crate) fn maybe_run_cleanup_with<Cb>(tick_count: u64, image_dir: &PathBuf, state: &mut AppState, cleanup_cb: Cb) -> ServiceResult<bool>
    where
        Cb: Fn(&PathBuf, &mut AppState) -> ServiceResult<()>,
    {
        if CLEANUP_INTERVAL_TICKS > 0 {
            if tick_count % (CLEANUP_INTERVAL_TICKS as u64) == 0 {
                debug!("Ticker: Running image cleanup (tick {})", tick_count);
                cleanup_cb(image_dir, state)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn maybe_run_cleanup(tick_count: u64, image_dir: &PathBuf, state: &mut AppState) {
        if let Err(e) = Self::maybe_run_cleanup_with(tick_count, image_dir, state, |p, s| {
            MotService::cleanup_expired_images(p, s)
        }) {
            error!("Ticker: Failed to run cleanup: {}", e);
        }
    }

    pub async fn start(app_state: Arc<web::Data<Mutex<AppState>>>, mot_dir: PathBuf, dls_file: PathBuf, image_dir: PathBuf) {
        info!(
            "Starting ticker service with {}-millisecond interval",
            INTERVAL_MS
        );
        let mut interval_timer = interval(Duration::from_millis(INTERVAL_MS));
        let mut previous_output_type: Option<OutputType> = None;
        let mut previous_content_id: Option<Uuid> = None;
        let mut tick_count: u64 = 0;

        loop {
            interval_timer.tick().await;
            tick_count = tick_count.wrapping_add(1);

            match app_state.lock() {
                Ok(mut state) => {
                    let now = Utc::now();

                    Self::update_output(&mut state, now, &dls_file, &mot_dir, &mut previous_output_type, &mut previous_content_id);
                    Self::maybe_run_cleanup(tick_count, &image_dir, &mut state);
                }
                Err(e) => {
                    error!("Ticker: Failed to acquire lock on app state: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data::{Station};
    use crate::models::AppState;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::{NamedTempFile, tempdir};
    use uuid::Uuid;

    #[test]
    fn update_output_with_calls_dls_and_mot() {
        let tmp_dls = NamedTempFile::new().expect("tmp dls");
        let mot_dir = tempdir().expect("mot dir");

        let mut app = AppState::default();
        app.station = Some(Station { id: Uuid::new_v4(), name: "TestStation".into(), image: None });

        let mut prev_type: Option<OutputType> = None;
        let mut prev_id: Option<Uuid> = None;

        let dls_called = Arc::new(AtomicBool::new(false));
        let mot_called = Arc::new(AtomicBool::new(false));

        let dls_flag = dls_called.clone();
        let mot_flag = mot_called.clone();

        let now = chrono::Utc::now();

        let changed = TickerService::update_output_with(
            &mut app,
            now,
            &tmp_dls.path().to_path_buf(),
            &mot_dir.path().to_path_buf(),
            &mut prev_type,
            &mut prev_id,
            move |_p, _s| {
                dls_flag.store(true, Ordering::SeqCst);
                Ok(())
            },
            move |_p, _s| {
                mot_flag.store(true, Ordering::SeqCst);
                Ok(())
            },
        ).expect("update_output_with should succeed");

        assert!(dls_called.load(Ordering::SeqCst), "DLS updater should have been called");
        assert!(mot_called.load(Ordering::SeqCst), "MOT updater should have been called");
        assert!(changed);
        assert_eq!(prev_type.unwrap(), OutputType::Station);
        assert!(prev_id.is_some());
    }

    #[test]
    fn update_output_with_is_noop_when_no_change() {
        let tmp_dls = NamedTempFile::new().expect("tmp dls");
        let mot_dir = tempdir().expect("mot dir");

        let mut app = AppState::default();
        let station = Station { id: Uuid::new_v4(), name: "S".into(), image: None };
        let station_id = station.id;
        app.station = Some(station);

        // seed previous to same values so no change is detected
        let mut prev_type = Some(OutputType::Station);
        let mut prev_id = Some(station_id);

        let dls_called = Arc::new(AtomicBool::new(false));
        let mot_called = Arc::new(AtomicBool::new(false));
        let dls_flag = dls_called.clone();
        let mot_flag = mot_called.clone();

        let now = chrono::Utc::now();

        let changed = TickerService::update_output_with(
            &mut app,
            now,
            &tmp_dls.path().to_path_buf(),
            &mot_dir.path().to_path_buf(),
            &mut prev_type,
            &mut prev_id,
            move |_p, _s| {
                dls_flag.store(true, Ordering::SeqCst);
                Ok(())
            },
            move |_p, _s| {
                mot_flag.store(true, Ordering::SeqCst);
                Ok(())
            },
        ).expect("update_output_with should succeed");

        assert!(!dls_called.load(Ordering::SeqCst), "DLS updater should NOT have been called");
        assert!(!mot_called.load(Ordering::SeqCst), "MOT updater should NOT have been called");
        assert!(!changed);
    }

    #[test]
    fn maybe_run_cleanup_with_calls_cleanup_on_interval() {
        // skip if CLEANUP_INTERVAL_TICKS == 0 (guard in implementation)
        if CLEANUP_INTERVAL_TICKS == 0 {
            eprintln!("Skipped maybe_run_cleanup test because CLEANUP_INTERVAL_TICKS == 0");
            return;
        }

        let image_dir = tempdir().expect("image dir");
        let mut app = AppState::default();

        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_flag = cleanup_called.clone();

        let tick = CLEANUP_INTERVAL_TICKS as u64;

        let ran = TickerService::maybe_run_cleanup_with(
            tick,
            &image_dir.path().to_path_buf(),
            &mut app,
            move |_p, _s| {
                cleanup_flag.store(true, Ordering::SeqCst);
                Ok(())
            },
        ).expect("maybe_run_cleanup_with should succeed");

        assert!(cleanup_called.load(Ordering::SeqCst), "cleanup should have been called on interval");
        assert!(ran);
    }
}
