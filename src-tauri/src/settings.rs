//! The AI settings: where photos and recipes run (this Mac, cloud if it
//! fails, cloud), and which cloud model. Local is the default.
//!
//! `set_ai_settings` reports a bad field as `BadInput("cloudModel")`.

use std::time::Instant;

use nookery_core::{AiMode, CLOUD_MODEL};
use nookery_recipes::{ChosenText, RecipeError, TextModel};
use nookery_store::{Store, SETTING_CLOUD_MODEL, SETTING_PHOTOS_AI, SETTING_RECIPES_AI};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use super::{bad, AppState, CommandError};

/// Where one feature runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AiModeView {
    Local,
    Fallback,
    Cloud,
}

impl From<AiMode> for AiModeView {
    fn from(m: AiMode) -> Self {
        match m {
            AiMode::Local => Self::Local,
            AiMode::Fallback => Self::Fallback,
            AiMode::Cloud => Self::Cloud,
        }
    }
}

impl From<AiModeView> for AiMode {
    fn from(m: AiModeView) -> Self {
        match m {
            AiModeView::Local => AiMode::Local,
            AiModeView::Fallback => AiMode::Fallback,
            AiModeView::Cloud => AiMode::Cloud,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub photos: AiModeView,
    pub recipes: AiModeView,
    pub cloud_model: String,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CloudCheckView {
    pub signed_in: bool,
    pub model: String,
    pub ms: f64,
    /// Why the check failed, when it wasn't just a missing sign-in.
    pub detail: Option<String>,
}

pub fn get_in(store: &Store) -> Result<AiSettingsView, CommandError> {
    Ok(AiSettingsView {
        photos: store.ai_mode(SETTING_PHOTOS_AI)?.into(),
        recipes: store.ai_mode(SETTING_RECIPES_AI)?.into(),
        cloud_model: store.cloud_model()?,
    })
}

pub fn set_in(store: &mut Store, input: &AiSettingsView) -> Result<AiSettingsView, CommandError> {
    let model = input.cloud_model.trim();
    let model = if model.is_empty() { CLOUD_MODEL } else { model };
    if model.chars().any(char::is_whitespace) {
        return Err(bad("cloudModel"));
    }
    store.set_setting(SETTING_PHOTOS_AI, AiMode::from(input.photos).as_str())?;
    store.set_setting(SETTING_RECIPES_AI, AiMode::from(input.recipes).as_str())?;
    store.set_setting(SETTING_CLOUD_MODEL, model)?;
    get_in(store)
}

/// One tiny request through `model`.
pub fn check_with(model: &dyn TextModel) -> CloudCheckView {
    let t = Instant::now();
    let r = model.generate("Reply with OK.", 8, &mut |_| {});
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let (signed_in, detail) = match r {
        Ok(_) => (true, None),
        Err(RecipeError::SignIn) => (false, None),
        Err(e) => (false, Some(e.to_string())),
    };
    CloudCheckView {
        signed_in,
        model: model.name(),
        ms,
        detail,
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_ai_settings(state: State<'_, AppState>) -> Result<AiSettingsView, CommandError> {
    let store = state.store()?;
    get_in(&store)
}

#[tauri::command]
#[specta::specta]
pub fn set_ai_settings(
    state: State<'_, AppState>,
    input: AiSettingsView,
) -> Result<AiSettingsView, CommandError> {
    let mut store = state.store()?;
    set_in(&mut store, &input)
}

#[tauri::command]
#[specta::specta]
pub async fn check_cloud(state: State<'_, AppState>) -> Result<CloudCheckView, CommandError> {
    let cloud_model = {
        let store = state.store()?;
        store.cloud_model()?
    };
    tauri::async_runtime::spawn_blocking(move || {
        check_with(&ChosenText::for_mode(AiMode::Cloud, &cloud_model))
    })
    .await
    .map_err(|_| CommandError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(fn() -> Result<String, RecipeError>);
    impl TextModel for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn generate(
            &self,
            _: &str,
            _: u32,
            _: &mut dyn FnMut(&str),
        ) -> Result<String, RecipeError> {
            (self.0)()
        }
    }

    fn settings(photos: AiModeView, recipes: AiModeView, model: &str) -> AiSettingsView {
        AiSettingsView {
            photos,
            recipes,
            cloud_model: model.into(),
        }
    }

    #[test]
    fn defaults_validation_and_round_trip() {
        let mut store = Store::open_in_memory().unwrap();
        let d = get_in(&store).unwrap();
        assert_eq!(d.photos, AiModeView::Local);
        assert_eq!(d.recipes, AiModeView::Local);
        assert_eq!(d.cloud_model, CLOUD_MODEL);
        let v = set_in(
            &mut store,
            &settings(AiModeView::Cloud, AiModeView::Fallback, "  other:1b-cloud "),
        )
        .unwrap();
        assert_eq!(v.photos, AiModeView::Cloud);
        assert_eq!(v.recipes, AiModeView::Fallback);
        assert_eq!(v.cloud_model, "other:1b-cloud");
        assert_eq!(get_in(&store).unwrap(), v);
        let v = set_in(
            &mut store,
            &settings(AiModeView::Local, AiModeView::Local, "  "),
        )
        .unwrap();
        assert_eq!(v.cloud_model, CLOUD_MODEL);
        let e = set_in(
            &mut store,
            &settings(AiModeView::Cloud, AiModeView::Cloud, "a b"),
        )
        .unwrap_err();
        assert!(matches!(e, CommandError::BadInput(f) if f == "cloudModel"));
        // Nothing was saved by the rejected call.
        assert_eq!(get_in(&store).unwrap().photos, AiModeView::Local);
    }

    #[test]
    fn modes_round_trip() {
        for m in AiMode::ALL {
            assert_eq!(AiMode::from(AiModeView::from(m)), m);
        }
    }

    #[test]
    fn check_maps_sign_in_success_and_other_errors() {
        let ok = check_with(&Fake(|| Ok("OK".into())));
        assert!(ok.signed_in && ok.detail.is_none());
        let s = check_with(&Fake(|| Err(RecipeError::SignIn)));
        assert!(!s.signed_in && s.detail.is_none());
        let u = check_with(&Fake(|| Err(RecipeError::Unreachable("http://x".into()))));
        assert!(!u.signed_in && u.detail.unwrap().contains("http://x"));
        assert!(matches!(
            CommandError::from(RecipeError::SignIn),
            CommandError::CloudSignIn
        ));
        assert!(matches!(
            CommandError::from(nookery_vision::VisionError::SignIn),
            CommandError::CloudSignIn
        ));
    }
}
