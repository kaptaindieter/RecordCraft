use std::path::{Path, PathBuf};

/// Drive detection and project folder management for RecordCraft & CutCraft.
#[derive(Clone, Debug)]
pub struct RecordCraftDrive {
    pub detected_drive_root: PathBuf,
    pub custom_drive_override: Option<PathBuf>,
    pub auto_save_enabled: bool,
}

impl RecordCraftDrive {
    pub fn new() -> Self {
        let detected = Self::detect_google_drive();
        let s = Self {
            detected_drive_root: detected,
            custom_drive_override: None,
            auto_save_enabled: true,
        };
        s.ensure_base_directory();
        s
    }

    pub fn user_configured_sync_root() -> Option<PathBuf> {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
        let cfg_path = PathBuf::from(home).join(".craft-suite").join("cloud-config.json");
        let raw = std::fs::read_to_string(cfg_path).ok()?;
        let val: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let sync_path = val.get("sync_path")?.as_str()?.trim();
        if sync_path.is_empty() {
            None
        } else {
            Some(PathBuf::from(sync_path))
        }
    }

    /// Automatically detects a cloud sync folder or falls back to the user's Documents directory.
    pub fn detect_google_drive() -> PathBuf {
        if let Some(custom) = Self::user_configured_sync_root() {
            return custom;
        }

        let candidates = [
            Path::new(r"G:\My Drive"),
            Path::new(r"G:\Meine Ablage"),
            Path::new(r"G:\"),
        ];

        for c in candidates {
            if c.exists() && c.is_dir() {
                return c.to_path_buf();
            }
        }

        if let Some(onedrive) = std::env::var_os("OneDrive") {
            let od = PathBuf::from(onedrive);
            if od.exists() && od.is_dir() {
                return od;
            }
        }

        if let Some(user_profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            let docs = PathBuf::from(user_profile).join("Documents");
            if docs.exists() {
                return docs;
            }
        }

        PathBuf::from("ArtCraft_Sync")
    }

    pub fn drive_root(&self) -> PathBuf {
        if let Some(ref o) = self.custom_drive_override {
            return o.clone();
        }
        if let Some(user_root) = Self::user_configured_sync_root() {
            return user_root;
        }
        self.detected_drive_root.clone()
    }

    pub fn is_drive_detected(&self) -> bool {
        self.custom_drive_override.is_some()
            || Self::user_configured_sync_root().is_some()
            || self.drive_root().starts_with("G:")
            || Path::new(r"G:\").exists()
            || self.drive_root().exists()
    }

    /// Base CutCraft project suite directory on drive:
    /// `G:\My Drive\Art Craft\CutCraft\`
    pub fn cutcraft_base_dir(&self) -> PathBuf {
        self.drive_root().join("Art Craft").join("CutCraft")
    }


    pub fn ensure_base_directory(&self) {
        let dir = self.cutcraft_base_dir();
        let _ = std::fs::create_dir_all(&dir);
    }

    /// Returns the project directory for a recording:
    /// `G:\My Drive\Art Craft\CutCraft\<Name>\`
    pub fn project_dir(&self, project_name: &str) -> PathBuf {
        self.cutcraft_base_dir().join(project_name)
    }

    /// Returns the raw recordings directory:
    /// `G:\My Drive\Art Craft\CutCraft\<Name>\raw_recordings\`
    pub fn raw_recordings_dir(&self, project_name: &str) -> PathBuf {
        self.project_dir(project_name).join("raw_recordings")
    }

    /// Ensures the project folder and raw_recordings folder exist.
    pub fn prepare_project_folders(&self, project_name: &str) -> (PathBuf, PathBuf) {
        let proj_dir = self.project_dir(project_name);
        let raw_dir = self.raw_recordings_dir(project_name);
        let _ = std::fs::create_dir_all(&raw_dir);
        (proj_dir, raw_dir)
    }

    /// Creates a complete CutCraft project (concat.json) with media item and timeline clip
    pub fn create_cutcraft_project(&self, project_name: &str, media_file: &Path) -> std::io::Result<PathBuf> {
        let (proj_dir, _) = self.prepare_project_folders(project_name);
        let project_file = proj_dir.join("concat.json");

        let media_path_str = media_file.to_string_lossy().to_string();
        let media_name = media_file.file_name().and_then(|n| n.to_str()).unwrap_or("Recording");

        let concat_json = serde_json::json!({
            "activeTimelineId": "TL1",
            "concat": "0.1.0",
            "fonts": [],
            "name": project_name,
            "media": [
                {
                    "id": "m1",
                    "name": media_name,
                    "path": media_path_str,
                    "kind": "video",
                    "hasAudio": true,
                    "audioTracks": [
                        { "index": 0 },
                        { "index": 1 }
                    ]
                }
            ],
            "timelines": [
                {
                    "id": "TL1",
                    "name": "Timeline 1",
                    "tracks": [
                        { "id": "T1", "muted": false, "visible": true },
                        { "id": "T2", "muted": false, "visible": true },
                        { "id": "T3", "muted": false, "visible": true },
                        { "id": "T4", "muted": false, "visible": true }
                    ],
                    "clips": [
                        {
                            "id": "c1",
                            "trackId": "T1",
                            "mediaId": "m1",
                            "name": media_name,
                            "kind": "video",
                            "start": 0.0,
                            "duration": 600.0,
                            "sourceStart": 0.0,
                            "volume": 1.0,
                            "fadeIn": 0.0,
                            "fadeOut": 0.0,
                            "scale": 1.0,
                            "offsetX": 0.0,
                            "offsetY": 0.0,
                            "rotation": 0.0,
                            "opacity": 1.0,
                            "speed": 1.0
                        }
                    ],
                    "video": {
                        "width": 1920,
                        "height": 1080,
                        "rateNum": 60,
                        "rateDen": 1
                    }
                }
            ],
            "tracks": [
                { "id": "T1", "muted": false, "visible": true },
                { "id": "T2", "muted": false, "visible": true },
                { "id": "T3", "muted": false, "visible": true },
                { "id": "T4", "muted": false, "visible": true }
            ]
        });

        std::fs::write(&project_file, serde_json::to_string_pretty(&concat_json)?)?;
        let project_concat_file = proj_dir.join("project.concat");
        let _ = std::fs::write(&project_concat_file, serde_json::to_string_pretty(&concat_json)?);
        Ok(proj_dir)
    }

}