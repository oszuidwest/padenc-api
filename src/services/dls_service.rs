use chrono::Utc;
use log::debug;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::errors::{ServiceError, ServiceResult};
use crate::models::tags::{ARTIST_TAG, PROGRAM_TAG, STATION_TAG, TITLE_TAG};
use crate::models::AppState;
use crate::services::ContentService;
use crate::services::content_service::OutputType;

pub struct DlsService;

impl DlsService {
    pub fn update_output_file(dls_path: &Path, app_state: &mut AppState) -> ServiceResult<()> {
        let now = Utc::now();
        debug!("Checking content states at {}", now);

        let output_type = ContentService::get_active_output_type(app_state, now);

        app_state.dl_plus_item_toggle = !app_state.dl_plus_item_toggle;
        let toggle_value = app_state.dl_plus_item_toggle as u8;

        let content = match output_type {
            OutputType::Track => {
                let track = &app_state.track.as_ref().expect("Track info missing");

                let title = &track.item.title;
                let artist = track.item.artist.as_deref().unwrap_or("");
                debug!("s {} - {}",
                    if artist.is_empty() { "(no artist)" } else { artist },
                    title
                );
                Self::generate_track_content(artist, title, toggle_value)
            },
            OutputType::Program => {
                let program = &app_state.program.as_ref().expect("Program info missing");

                let program_name = &program.name;
                debug!("Writing program info: {}", program_name);
                Self::generate_program_content(program_name, toggle_value)
            },
            OutputType::Station => {
                let station = &app_state.station.as_ref().expect("Station info missing");

                let station_name = &station.name;
                debug!("Writing station info: {}", station_name);
                Self::generate_station_content(station_name, toggle_value)
            }
        };

        Self::write_content_to_file(dls_path, &content)
    }

    fn write_content_to_file(dls_path: &Path, content: &str) -> ServiceResult<()> {
        let mut file = fs::File::create(dls_path).map_err(|e| {
            ServiceError::FileProcessing(format!("Failed to create output file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            ServiceError::FileProcessing(format!("Failed to write to output file: {}", e))
        })?;
        debug!(
            "Successfully wrote {} bytes to {:?}",
            content.len(),
            dls_path
        );

        Ok(())
    }

    pub fn generate_track_content(artist: &str, title: &str, toggle_value: u8) -> String {
        if artist.is_empty() {
            return format!(
                "##### parameters {{ #####\n\
                 DL_PLUS=1\n\
                 DL_PLUS_TAG={} 0 {}\n\
                 DL_PLUS_ITEM_RUNNING=1\n\
                 DL_PLUS_ITEM_TOGGLE={}\n\
                 ##### parameters }} #####\n\
                 {}",
                TITLE_TAG,
                title.chars().count(),
                toggle_value,
                title
            );
        }

        let separator = " - ";
        let display_text = format!("{}{}{}", artist, separator, title);

        let artist_start = 0;
        let artist_length = artist.chars().count() as u32;
        let title_start = artist_length + separator.chars().count() as u32;
        let title_length = title.chars().count() as u32;

        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} {} {}\n\
             DL_PLUS_TAG={} {} {}\n\
             DL_PLUS_ITEM_RUNNING=1\n\
             DL_PLUS_ITEM_TOGGLE={}\n\
             ##### parameters }} #####\n\
             {}",
            ARTIST_TAG,
            artist_start,
            artist_length,
            TITLE_TAG,
            title_start,
            title_length,
            toggle_value,
            display_text
        )
    }

    pub fn generate_program_content(program_name: &str, toggle_value: u8) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             DL_PLUS_ITEM_RUNNING=0\n\
             DL_PLUS_ITEM_TOGGLE={}\n\
             ##### parameters }} #####\n\
             {}",
            PROGRAM_TAG,
            program_name.chars().count(),
            toggle_value,
            program_name
        )
    }

    pub fn generate_station_content(station_name: &str, toggle_value: u8) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             DL_PLUS_ITEM_RUNNING=0\n\
             DL_PLUS_ITEM_TOGGLE={}\n\
             ##### parameters }} #####\n\
             {}",
            STATION_TAG,
            station_name.chars().count(),
            toggle_value,
            station_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data::{Item, Program, Station, Track};
    use crate::models::AppState;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn generate_station_content_includes_tag_and_name() {
        let s = DlsService::generate_station_content("MyStation", 1);
        assert!(s.contains(&format!("DL_PLUS_TAG={} 0 9", STATION_TAG)));
        assert!(s.contains("DL_PLUS_ITEM_RUNNING=0"));
        assert!(s.ends_with("MyStation"));
    }

    #[test]
    fn generate_station_content_toggle_value() {
        let s = DlsService::generate_station_content("MyStation", 0);
        assert!(s.contains("DL_PLUS_ITEM_TOGGLE=0"));

        let s = DlsService::generate_station_content("MyStation", 1);
        assert!(s.contains("DL_PLUS_ITEM_TOGGLE=1"));
    }

    #[test]
    fn generate_program_content_includes_tag_and_name() {
        let p = DlsService::generate_program_content("MyProgram", 0);
        assert!(p.contains(&format!("DL_PLUS_TAG={} 0 9", PROGRAM_TAG)));
        assert!(p.contains("DL_PLUS_ITEM_RUNNING=0"));
        assert!(p.ends_with("MyProgram"));
    }

    #[test]
    fn generate_program_content_toggle_value() {
        let p = DlsService::generate_program_content("MyProgram", 0);
        assert!(p.contains("DL_PLUS_ITEM_TOGGLE=0"));

        let p = DlsService::generate_program_content("MyProgram", 1);
        assert!(p.contains("DL_PLUS_ITEM_TOGGLE=1"));
    }

    #[test]
    fn generate_track_content_with_artist_formats_both_tags() {
        let t = DlsService::generate_track_content("Artist", "Title", 1);
        assert!(t.contains(&format!("DL_PLUS_TAG={} 0 6", ARTIST_TAG)));
        assert!(t.contains(&format!("DL_PLUS_TAG={} 9 5", TITLE_TAG)));
        assert!(t.contains("DL_PLUS_ITEM_RUNNING=1"));
        assert!(t.ends_with("Artist - Title"));
    }

    #[test]
    fn generate_track_content_without_artist_uses_title_tag_only() {
        let t = DlsService::generate_track_content("", "SoloTitle", 0);
        assert!(t.contains(&format!("DL_PLUS_TAG={} 0 9", TITLE_TAG)));
        assert!(t.contains("DL_PLUS_ITEM_RUNNING=1"));
        assert!(t.ends_with("SoloTitle"));
    }

    #[test]
    fn generate_track_content_toggle_value() {
        let t = DlsService::generate_track_content("", "SoloTitle", 0);
        assert!(t.contains("DL_PLUS_ITEM_TOGGLE=0"));

        let t = DlsService::generate_track_content("", "SoloTitle", 1);
        assert!(t.contains("DL_PLUS_ITEM_TOGGLE=1"));
    }

    #[test]
    fn update_output_file_writes_station_content_and_toggles() {
        // Create a temp file
        let tmp = NamedTempFile::new().expect("create tmp file");
        let path = tmp.path().to_path_buf();

        // Minimal app state with only station
        let mut app = AppState::default();
        app.station = Some(Station { name: "StationX".into(), id: uuid::Uuid::new_v4(), image: None });
        app.dl_plus_item_toggle = false; // expect toggle to flip to true (1)

        // Call update_output_file
        let res = DlsService::update_output_file(&path, &mut app);
        assert!(res.is_ok());

        // Read file and assert contents
        let content = fs::read_to_string(&path).expect("read tmp file");
        assert!(content.contains("StationX"));
        assert!(content.contains("DL_PLUS_ITEM_TOGGLE=1"));
        // toggle value in app state should have flipped
        assert!(app.dl_plus_item_toggle);
    }

    #[test]
    fn update_output_file_writes_track_content() {
        let tmp = NamedTempFile::new().expect("tmp file");
        let path = tmp.path().to_path_buf();

        let mut app = AppState::default();
        app.track = Some(Track {
            id: uuid::Uuid::new_v4(),
            item: Item { title: "Title".into(), artist: Some("Artist".into()) },
            expires_at: None,
            image: None,
        });

        DlsService::update_output_file(&path, &mut app).expect("ok");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("DL_PLUS_ITEM_RUNNING=1"));
        assert!(content.ends_with("Artist - Title"));
    }

    #[test]
    fn update_output_file_writes_track_without_artist() {
        let tmp = NamedTempFile::new().expect("tmp file");
        let path = tmp.path().to_path_buf();

        let mut app = AppState::default();
        app.track = Some(Track {
            id: uuid::Uuid::new_v4(),
            item: Item { title: "Solo".into(), artist: None },
            expires_at: None,
            image: None,
        });

        DlsService::update_output_file(&path, &mut app).expect("ok");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.ends_with("Solo"));
        assert!(content.contains(&format!("DL_PLUS_TAG={} 0 4", TITLE_TAG)));
    }

    #[test]
    fn update_output_file_writes_program_content() {
        let tmp = NamedTempFile::new().expect("tmp file");
        let path = tmp.path().to_path_buf();

        let mut app = AppState::default();
        app.program = Some(Program {
            id: uuid::Uuid::new_v4(),
            name: "Prog".into(),
            expires_at: None,
            image: None,
        });

        DlsService::update_output_file(&path, &mut app).expect("ok");
        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("DL_PLUS_ITEM_RUNNING=0"));
        assert!(content.ends_with("Prog"));
    }

    #[test]
    fn update_output_file_to_unwritable_path_is_err() {
        // A path whose parent directory does not exist makes File::create fail,
        // exercising the write error branch.
        let dir = tempfile::tempdir().unwrap();
        let bad_path = dir.path().join("nonexistent-subdir").join("dls.txt");

        let mut app = AppState::default();
        app.station = Some(Station { id: uuid::Uuid::new_v4(), name: "S".into(), image: None });

        let res = DlsService::update_output_file(&bad_path, &mut app);
        assert!(matches!(res, Err(ServiceError::FileProcessing(_))));
    }

    #[test]
    fn generate_track_content_uses_char_count_not_byte_count() {
        // "café" is 4 chars but 5 UTF-8 bytes; the DL Plus length must be chars.
        let t = DlsService::generate_track_content("café", "naïve", 1);
        // artist starts at 0, length 4 (chars)
        assert!(t.contains(&format!("DL_PLUS_TAG={} 0 4", ARTIST_TAG)));
        // title starts after "café" (4) + " - " (3) = 7, length 5 (chars)
        assert!(t.contains(&format!("DL_PLUS_TAG={} 7 5", TITLE_TAG)));
    }
}
