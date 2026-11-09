//! `nookery settings`: where the AI runs for photos and recipes (this Mac,
//! the cloud as a fallback, or the cloud), and which cloud model. Local is
//! the default; nothing leaves the Mac unless the user chose it. Also the
//! per-run `--local` / `--cloud` overrides shared by `photo` and `recipes`.

use std::io::Write;
use std::time::Instant;

use clap::{Args, Subcommand};
use nookery_core::AiMode;
use nookery_recipes::{RecipeError, TextModel};
use nookery_store::{Store, SETTING_CLOUD_MODEL, SETTING_PHOTOS_AI, SETTING_RECIPES_AI};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// What goes to the cloud model, for the settings screen and the note after
/// a run.
pub const PHOTOS_SENT: &str = "the photo, cut into parts";
pub const RECIPES_SENT: &str = "item names, amounts and template text";

/// Who answered a run, for the timing line and the cloud note.
pub struct Who {
    pub by: String,
    pub cloud: bool,
}

/// `--local` / `--cloud`: use that for this run only (mutually exclusive).
#[derive(Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiOverride {
    /// Use only the model on this Mac for this run.
    #[arg(long, conflicts_with = "cloud")]
    pub local: bool,
    /// Use the cloud model for this run.
    #[arg(long)]
    pub cloud: bool,
}

impl AiOverride {
    pub fn mode(&self, stored: AiMode) -> AiMode {
        if self.local {
            AiMode::Local
        } else if self.cloud {
            AiMode::Cloud
        } else {
            stored
        }
    }
}

/// The note printed to stderr after a run that used the cloud.
pub fn cloud_note(sent: &str) -> String {
    format!("sent to the cloud: {sent}")
}

/// The message for a 401 from a cloud model; `feature` is `photos` or
/// `recipes`.
pub fn sign_in_message(feature: &str) -> String {
    format!(
        "the cloud model needs an Ollama account: run `ollama signin`, or `nookery settings {feature} local`"
    )
}

#[derive(Args)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub cmd: Option<SettingsCmd>,
}

#[derive(Subcommand)]
pub enum SettingsCmd {
    /// Where photo scanning runs: local, fallback or cloud.
    Photos { mode: AiMode },
    /// Where recipe writing runs: local, fallback or cloud.
    Recipes { mode: AiMode },
    /// The cloud model (empty or `default` resets it).
    CloudModel { name: String },
    /// Send one tiny request to the cloud model to see if you're signed in.
    CheckCloud,
}

fn explain(mode: AiMode) -> &'static str {
    match mode {
        AiMode::Local => "this Mac only",
        AiMode::Fallback => "this Mac first, the cloud if the local model is missing or fails",
        AiMode::Cloud => "the cloud model",
    }
}

fn show(store: &Store, out: &mut dyn Write) -> Res<()> {
    for (name, key, sent) in [
        ("photos", SETTING_PHOTOS_AI, PHOTOS_SENT),
        ("recipes", SETTING_RECIPES_AI, RECIPES_SENT),
    ] {
        let mode = store.ai_mode(key)?;
        writeln!(out, "{name:<12} {:<9} {}", mode.as_str(), explain(mode))?;
        writeln!(out, "{:<12} {:<9} cloud use sends: {sent}", "", "")?;
    }
    writeln!(out, "{:<12} {}", "cloud model", store.cloud_model()?)?;
    writeln!(
        out,
        "\nChange: nookery settings photos|recipes <local|fallback|cloud>, nookery settings cloud-model <name>"
    )?;
    Ok(())
}

/// `model` is the cloud text model, used by `check-cloud` only.
pub fn run(
    store: &mut Store,
    args: SettingsArgs,
    model: &dyn TextModel,
    out: &mut dyn Write,
) -> Res<()> {
    match args.cmd {
        None => {}
        Some(SettingsCmd::Photos { mode }) => {
            store.set_setting(SETTING_PHOTOS_AI, mode.as_str())?
        }
        Some(SettingsCmd::Recipes { mode }) => {
            store.set_setting(SETTING_RECIPES_AI, mode.as_str())?
        }
        Some(SettingsCmd::CloudModel { name }) => {
            let name = name.trim();
            // Empty is stored as "unset": the default can change later.
            let value = if name.eq_ignore_ascii_case("default") {
                ""
            } else {
                name
            };
            store.set_setting(SETTING_CLOUD_MODEL, value)?;
        }
        Some(SettingsCmd::CheckCloud) => return check_cloud(model, out),
    }
    show(store, out)
}

fn check_cloud(model: &dyn TextModel, out: &mut dyn Write) -> Res<()> {
    let t = Instant::now();
    match model.generate("Reply with OK.", 16, &mut |_| {}) {
        Ok(_) => {
            writeln!(
                out,
                "signed in, answered in {:.1} s ({})",
                t.elapsed().as_secs_f64(),
                model.name()
            )?;
            Ok(())
        }
        Err(RecipeError::SignIn) => Err("not signed in: run `ollama signin`".into()),
        Err(e) => Err(format!("the cloud check failed: {e}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use nookery_core::CLOUD_MODEL;

    struct Fixed(fn() -> Result<String, RecipeError>);
    impl TextModel for Fixed {
        fn name(&self) -> String {
            "fixed".into()
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

    fn ok() -> Fixed {
        Fixed(|| Ok("OK".into()))
    }

    fn go(store: &mut Store, cmd: Option<SettingsCmd>) -> Res<String> {
        let mut out = Vec::new();
        run(store, SettingsArgs { cmd }, &ok(), &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn defaults_are_local_and_the_default_cloud_model() {
        let mut s = Store::open_in_memory().unwrap();
        let o = go(&mut s, None).unwrap();
        assert!(o.contains("photos       local"), "{o}");
        assert!(o.contains("recipes      local"), "{o}");
        assert!(o.contains(PHOTOS_SENT) && o.contains(RECIPES_SENT), "{o}");
        assert!(o.contains(CLOUD_MODEL), "{o}");
    }

    #[test]
    fn modes_and_the_cloud_model_can_be_set_and_reset() {
        let mut s = Store::open_in_memory().unwrap();
        go(
            &mut s,
            Some(SettingsCmd::Photos {
                mode: AiMode::Fallback,
            }),
        )
        .unwrap();
        go(
            &mut s,
            Some(SettingsCmd::Recipes {
                mode: AiMode::Cloud,
            }),
        )
        .unwrap();
        assert_eq!(s.ai_mode(SETTING_PHOTOS_AI).unwrap(), AiMode::Fallback);
        assert_eq!(s.ai_mode(SETTING_RECIPES_AI).unwrap(), AiMode::Cloud);
        let o = go(
            &mut s,
            Some(SettingsCmd::CloudModel {
                name: " x:cloud ".into(),
            }),
        )
        .unwrap();
        assert!(o.contains("x:cloud"), "{o}");
        assert_eq!(s.cloud_model().unwrap(), "x:cloud");
        for reset in ["default", "", "  "] {
            go(&mut s, Some(SettingsCmd::CloudModel { name: "y".into() })).unwrap();
            go(&mut s, Some(SettingsCmd::CloudModel { name: reset.into() })).unwrap();
            assert_eq!(s.cloud_model().unwrap(), CLOUD_MODEL);
        }
    }

    #[test]
    fn a_bad_mode_is_rejected_by_the_parser() {
        #[derive(Parser)]
        struct C {
            #[command(subcommand)]
            cmd: SettingsCmd,
        }
        assert!(C::try_parse_from(["x", "photos", "sky"]).is_err());
        let ok = C::try_parse_from(["x", "recipes", "fallback"]).unwrap();
        assert!(matches!(
            ok.cmd,
            SettingsCmd::Recipes {
                mode: AiMode::Fallback
            }
        ));
    }

    #[test]
    fn check_cloud_reports_signed_in_and_not_signed_in() {
        let mut s = Store::open_in_memory().unwrap();
        let mut out = Vec::new();
        let args = SettingsArgs {
            cmd: Some(SettingsCmd::CheckCloud),
        };
        run(&mut s, args, &ok(), &mut out).unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .starts_with("signed in, answered in"));

        let args = SettingsArgs {
            cmd: Some(SettingsCmd::CheckCloud),
        };
        let e = run(
            &mut s,
            args,
            &Fixed(|| Err(RecipeError::SignIn)),
            &mut Vec::new(),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(e, "not signed in: run `ollama signin`");
    }

    #[derive(Parser)]
    struct Flags {
        #[command(flatten)]
        ai: AiOverride,
    }

    #[test]
    fn overrides_parse_and_exclude_each_other() {
        let none = Flags::try_parse_from(["x"]).unwrap().ai;
        assert_eq!(none.mode(AiMode::Fallback), AiMode::Fallback);
        let l = Flags::try_parse_from(["x", "--local"]).unwrap().ai;
        assert_eq!(l.mode(AiMode::Cloud), AiMode::Local);
        let c = Flags::try_parse_from(["x", "--cloud"]).unwrap().ai;
        assert_eq!(c.mode(AiMode::Local), AiMode::Cloud);
        assert!(Flags::try_parse_from(["x", "--local", "--cloud"]).is_err());
    }

    #[test]
    fn sign_in_message_names_the_feature() {
        let m = sign_in_message("recipes");
        assert!(m.contains("ollama signin") && m.contains("nookery settings recipes local"));
    }
}
